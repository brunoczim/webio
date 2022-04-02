//! This module exports items related to task spawning.

use std::{
    any::Any,
    cell::UnsafeCell,
    fmt,
    future::Future,
    mem,
    panic,
    pin::Pin,
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering::*},
        Mutex,
    },
    task,
};
use wasm_bindgen_futures::spawn_local;

type PanicPayload = Box<dyn Any + Send + 'static>;

#[derive(Debug)]
enum JoinErrorKind {
    Panicked(PanicPayload),
    Cancelled,
}

impl fmt::Display for JoinErrorKind {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JoinErrorKind::Panicked(payload) => {
                write!(fmtr, "task panicked: {:?}", payload)
            },
            JoinErrorKind::Cancelled => write!(fmtr, "task cancelled"),
        }
    }
}

/// An error that might happen when waiting for a task.
#[derive(Debug)]
pub struct JoinError {
    kind: JoinErrorKind,
}

impl fmt::Display for JoinError {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        write!(fmtr, "{}", self.kind)
    }
}

impl JoinError {
    /// Tests whether the target task was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(&self.kind, JoinErrorKind::Cancelled)
    }

    /// Tests whether the target task panicked.
    pub fn is_panic(&self) -> bool {
        matches!(&self.kind, JoinErrorKind::Panicked(_))
    }

    /// Attempts to convert this error into a panic payload. Fails if the target
    /// task didn't panicked.
    pub fn try_into_panic(self) -> Result<PanicPayload, Self> {
        match self.kind {
            JoinErrorKind::Panicked(payload) => Ok(payload),
            kind => Err(Self { kind }),
        }
    }

    /// Converts this error into a panic payload.
    ///
    /// # Panics
    /// Panics if this error was not caused by panic (e.g. the task was
    /// cancelled).
    pub fn into_panic(self) -> PanicPayload {
        self.try_into_panic().unwrap()
    }
}

#[derive(Debug)]
struct Shared<T> {
    connected: AtomicBool,
    waker: Mutex<Option<task::Waker>>,
    data: UnsafeCell<Result<T, JoinErrorKind>>,
}

struct TaskHandle<T> {
    shared: NonNull<Shared<T>>,
}

impl<T> TaskHandle<T> {
    unsafe fn new(shared: NonNull<Shared<T>>) -> Self {
        Self { shared }
    }

    fn shared(&self) -> &Shared<T> {
        unsafe { self.shared.as_ref() }
    }

    fn panicked(self, payload: PanicPayload) {
        unsafe {
            *self.shared().data.get() = Err(JoinErrorKind::Panicked(payload));
        }
    }

    fn success(self, data: T) {
        unsafe {
            *self.shared().data.get() = Ok(data);
        }
    }
}

impl<T> Drop for TaskHandle<T> {
    fn drop(&mut self) {
        let was_connected = self.shared().connected.swap(false, AcqRel);
        if was_connected {
            if let Some(waker) = self.shared().waker.lock().unwrap().take() {
                waker.wake();
            }
        } else {
            unsafe {
                Box::from_raw(self.shared.as_ptr());
            }
        }
    }
}

/// A handle that allows the caller to join a task (i.e. wait for it to end).
pub struct JoinHandle<T> {
    shared: NonNull<Shared<T>>,
}

impl<T> JoinHandle<T> {
    unsafe fn new(shared: NonNull<Shared<T>>) -> Self {
        Self { shared }
    }

    fn shared(&self) -> &Shared<T> {
        unsafe { self.shared.as_ref() }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        if self.shared().connected.load(Acquire) {
            let mut waker = self.shared().waker.lock().unwrap();
            if waker.is_none() {
                *waker = Some(ctx.waker().clone());
            }
            task::Poll::Pending
        } else {
            let result = mem::replace(
                unsafe { &mut *self.shared().data.get() },
                Err(JoinErrorKind::Cancelled),
            );
            task::Poll::Ready(result.map_err(|kind| JoinError { kind }))
        }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        let was_connected = self.shared().connected.swap(false, AcqRel);
        if !was_connected {
            unsafe {
                Box::from_raw(self.shared.as_ptr());
            }
        }
    }
}

struct CatchUnwind<A>
where
    A: Future,
{
    future: A,
}

impl<A> Future for CatchUnwind<A>
where
    A: Future,
{
    type Output = Result<A::Output, PanicPayload>;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(move || {
            let future =
                unsafe { self.map_unchecked_mut(|this| &mut this.future) };
            future.poll(ctx)
        }));

        match result {
            Ok(task::Poll::Pending) => task::Poll::Pending,
            Ok(task::Poll::Ready(data)) => task::Poll::Ready(Ok(data)),
            Err(error) => task::Poll::Ready(Err(error)),
        }
    }
}

/// Spawns an asynchronous task in JS event loop.
pub fn spawn<A>(future: A) -> JoinHandle<A::Output>
where
    A: Future + 'static,
{
    let boxed_shared = Box::new(Shared {
        connected: AtomicBool::new(true),
        waker: Mutex::new(None),
        data: UnsafeCell::new(Err(JoinErrorKind::Cancelled)),
    });

    let shared = NonNull::from(&*boxed_shared);
    Box::into_raw(boxed_shared);

    let task_handle = unsafe { TaskHandle::new(shared) };
    let join_handle = unsafe { JoinHandle::new(shared) };

    spawn_local(async move {
        let result = CatchUnwind { future }.await;
        match result {
            Ok(data) => task_handle.success(data),
            Err(payload) => task_handle.panicked(payload),
        }
    });

    join_handle
}
