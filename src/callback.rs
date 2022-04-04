//! This module defines utilities for translating a callback into asyncrhonous
//! functions.

use crate::panic::{CatchUnwind, Payload};
use std::{cell::Cell, fmt, future::Future, panic, pin::Pin, rc::Rc, task};

pub type SyncCallback<'cb> = Box<dyn FnOnce() + 'cb>;

pub type AsyncCallback<'cb> = Pin<Box<dyn Future<Output = ()> + 'cb>>;

/// An event on which callbacks are registered.
pub struct Event<F> {
    register: F,
}

impl<F> Event<F> {
    /// Creates an event from a registration function, which is used to
    /// register callbacks into events.
    pub fn new(register_callback: F) -> Self {
        Self { register: register_callback }
    }

    /// Register a callback for a oneshot call. Returns a future that completes
    /// when the event completes.
    pub fn listen_once<'cb, C, U>(self, callback: C) -> CallbackOnce<U>
    where
        F: FnOnce(SyncCallback<'cb>),
        C: FnOnce() -> U + 'cb,
        U: 'cb,
    {
        let (_, callback) = self.listen_once_returning(callback);
        callback
    }

    /// Register a callback for a oneshot call. Returns a future that completes
    /// when the event completes, and also the return value of the registration
    /// function.
    pub fn listen_once_returning<'cb, C, T, U>(
        self,
        callback: C,
    ) -> (T, CallbackOnce<U>)
    where
        F: FnOnce(SyncCallback<'cb>) -> T,
        C: FnOnce() -> U + 'cb,
        U: 'cb,
    {
        let shared = Rc::new(Shared::init_connected());

        let callback_handle = CallbackHandle::new(shared.clone());
        let callback_once = CallbackOnce::new(shared);

        let boxed_fn = Box::new(move || {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(callback));
            match result {
                Ok(data) => callback_handle.success(data),
                Err(payload) => callback_handle.panicked(payload),
            }
        });
        let ret = (self.register)(boxed_fn as SyncCallback);

        (ret, callback_once)
    }

    /// Register an asynchronous callback for a oneshot call. Returns a future
    /// that completes when the event completes.
    pub fn listen_once_async<'fut, A>(
        self,
        callback: A,
    ) -> CallbackOnce<A::Output>
    where
        F: FnOnce(AsyncCallback<'fut>),
        A: Future + 'fut,
    {
        let (_, callback) = self.listen_once_async_returning(callback);
        callback
    }

    /// Register an asynchronous callback for a oneshot call. Returns a future
    /// that completes when the event completes, and also the return value of
    /// the registration function.
    pub fn listen_once_async_returning<'fut, T, A>(
        self,
        callback: A,
    ) -> (T, CallbackOnce<A::Output>)
    where
        F: FnOnce(AsyncCallback<'fut>) -> T,
        A: Future + 'fut,
    {
        let shared = Rc::new(Shared::init_connected());

        let callback_handle = CallbackHandle::new(shared.clone());
        let callback_once = CallbackOnce::new(shared);

        let future = Box::pin(async move {
            let result = CatchUnwind::new(callback).await;
            match result {
                Ok(data) => callback_handle.success(data),
                Err(payload) => callback_handle.panicked(payload),
            }
        });
        let ret = (self.register)(future as AsyncCallback);

        (ret, callback_once)
    }
}

/// An error that might happen when the event completes.
#[derive(Debug)]
pub enum Error {
    /// The callback panicked! And here is panic's payload.
    Panicked(Payload),
    /// The callback's future was cancelled.
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Panicked(payload) => {
                write!(fmtr, "task panicked: {:?}", payload)
            },
            Error::Cancelled => write!(fmtr, "task cancelled"),
        }
    }
}

struct Shared<T> {
    connected: Cell<bool>,
    waker: Cell<Option<task::Waker>>,
    data: Cell<Result<T, Error>>,
}

impl<T> fmt::Debug for Shared<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        let waker = self.waker.take();
        let data = self.data.replace(Err(Error::Cancelled));
        let result = fmtr
            .debug_struct("callback::Shared")
            .field("connected", &self.connected)
            .field("waker", &waker)
            .field("data", &data)
            .finish();
        self.waker.set(waker);
        self.data.set(data);
        result
    }
}

impl<T> Shared<T> {
    fn init_connected() -> Self {
        Self {
            connected: Cell::new(true),
            waker: Cell::new(None),
            data: Cell::new(Err(Error::Cancelled)),
        }
    }
}

#[derive(Debug)]
struct CallbackHandle<T> {
    shared: Rc<Shared<T>>,
}

impl<T> CallbackHandle<T> {
    fn new(shared: Rc<Shared<T>>) -> Self {
        Self { shared }
    }

    fn panicked(self, payload: Payload) {
        self.shared.data.set(Err(Error::Panicked(payload)));
    }

    fn success(self, data: T) {
        self.shared.data.set(Ok(data));
    }
}

impl<T> Drop for CallbackHandle<T> {
    fn drop(&mut self) {
        let was_connected = self.shared.connected.replace(false);
        if was_connected {
            if let Some(waker) = self.shared.waker.take() {
                waker.wake();
            }
        }
    }
}

/// A handle to a oneshot callback registered in an event.
#[derive(Debug)]
pub struct CallbackOnce<T> {
    shared: Rc<Shared<T>>,
}

impl<T> CallbackOnce<T> {
    fn new(shared: Rc<Shared<T>>) -> Self {
        Self { shared }
    }
}

impl<T> Future for CallbackOnce<T> {
    type Output = Result<T, Error>;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        if self.shared.connected.get() {
            let mut waker = self.shared.waker.take();
            if waker.is_none() {
                waker = Some(ctx.waker().clone());
            }
            self.shared.waker.set(waker);
            task::Poll::Pending
        } else {
            let data = self.shared.data.replace(Err(Error::Cancelled));
            task::Poll::Ready(data)
        }
    }
}

impl<T> Drop for CallbackOnce<T> {
    fn drop(&mut self) {
        self.shared.connected.set(false);
    }
}
