//! This module implements conversion from callbacks to futures for callbacks
//! that are called only once.

use crate::{callback, panic::FutureCatchUnwind};
use std::{future::Future, panic, pin::Pin, task};

macro_rules! sync_once {
    ($self:expr, $callback:expr) => {{
        let (notifier, inner_listener) = callback::shared::channel();

        let handler = Box::new(move || {
            let result =
                panic::catch_unwind(panic::AssertUnwindSafe($callback));
            match result {
                Ok(data) => notifier.success(data),
                Err(payload) => notifier.panicked(payload),
            }
        });
        let ret = ($self.register_fn)(handler as SyncCbHandler);

        (ret, Listener::new(inner_listener))
    }};
}

macro_rules! async_once {
    ($self:expr, $callback:expr) => {{
        let (notifier, inner_listener) = callback::shared::channel();

        let handler = Box::pin(async move {
            let result = FutureCatchUnwind::new($callback).await;
            match result {
                Ok(data) => notifier.success(data),
                Err(payload) => notifier.panicked(payload),
            }
        });
        let ret = ($self.register_fn)(handler as AsyncCbHandler);

        (ret, Listener::new(inner_listener))
    }};
}

/// The type of synchronous, oneshot callback handlers (i.e. the handler that
/// calls callbacks): a boxed function.
pub type SyncCbHandler<'cb> = Box<dyn FnOnce() + 'cb>;

/// The type of asynchronous, oneshot callback handlers (i.e. the handler that
/// calls callbacks): a boxed future.
pub type AsyncCbHandler<'cb> = Pin<Box<dyn Future<Output = ()> + 'cb>>;

/// Register of oneshot callbacks into an event, where the callback is
/// syncrhonous (though waiting for the callback to complete is still
/// asynchronous).
#[derive(Debug, Clone, Copy)]
pub struct SyncRegister<F> {
    register_fn: F,
}

impl<F> SyncRegister<F> {
    /// Creates a new register using an inner register function that can be used
    /// only once. Such function receives callbacks handlers and register
    /// them. Callback handlers are register-internal functions that are called
    /// when the event completes, and then, they call the actual callbacks, i.e.
    /// a wrapper for the actual callback.
    pub fn new<'cb, T>(register_fn: F) -> Self
    where
        F: FnOnce(SyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times, with mutability, however. Such function receives
    /// callbacks handlers and register them. Callback handlers are
    /// register-internal functions that are called when the event completes,
    /// and then, they call the actual callbacks, i.e. a wrapper for the actual
    /// callback.
    pub fn new_mut<'cb, T>(register_fn: F) -> Self
    where
        F: FnMut(SyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times and imutabily. Such function receives callbacks handlers
    /// and register them. Callback handlers are register-internal functions
    /// that are called when the event completes, and then, they call the
    /// actual callbacks, i.e. a wrapper for the actual callback.
    pub fn new_ref<'cb, T>(register_fn: F) -> Self
    where
        F: Fn(SyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asyncrhonous: a future is returned, and when awaited, it waits
    /// for the callback to complete.
    ///
    /// This method consumes the register.
    ///
    /// # Examples
    ///
    /// ## Dummy Example
    ///
    /// ```no_run
    /// use webio::callback;
    ///
    /// # use webio::task;
    /// # fn main() {
    /// # task::detach(async {
    /// let register = callback::once::SyncRegister::new(|callback| {
    ///     callback();
    /// });
    /// let result = register.listen(|| 42).await;
    /// assert_eq!(result.unwrap(), 42);
    /// # });
    /// # }
    /// ```
    pub fn listen<'cb, C, U>(self, callback: C) -> Listener<U>
    where
        F: FnOnce(SyncCbHandler<'cb>),
        C: FnOnce() -> U + 'cb,
        U: 'cb,
    {
        let (_, listener) = self.listen_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asyncrhonous: a future is returned, and when awaited, it waits
    /// for the callback to complete.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut<'cb, C, U>(&mut self, callback: C) -> Listener<U>
    where
        F: FnMut(SyncCbHandler<'cb>),
        C: FnOnce() -> U + 'cb,
        U: 'cb,
    {
        let (_, listener) = self.listen_mut_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asyncrhonous: a future is returned, and when awaited, it waits
    /// for the callback to complete.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref<'cb, C, U>(&self, callback: C) -> Listener<U>
    where
        F: Fn(SyncCbHandler<'cb>),
        C: FnOnce() -> U + 'cb,
        U: 'cb,
    {
        let (_, listener) = self.listen_ref_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asyncrhonous: a future is returned, and when awaited, it
    /// waits for the callback to complete. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the wait-future.
    ///
    /// This method consumes the register.
    ///
    /// # Examples
    ///
    /// ## Dummy Example
    ///
    /// ```no_run
    /// use webio::callback;
    ///
    /// # use webio::task;
    /// # fn main() {
    /// # task::detach(async {
    /// let register = callback::once::SyncRegister::new(|callback| {
    ///     callback();
    ///     "my-ret-value"
    /// });
    /// let (ret_value, future) = register.listen_returning(|| 42);
    /// assert_eq!(ret_value, "my-ret-value");
    /// let result = future.await;
    /// assert_eq!(result.unwrap(), 42);
    /// # });
    /// # }
    /// ```
    pub fn listen_returning<'cb, C, T, U>(self, callback: C) -> (T, Listener<U>)
    where
        F: FnOnce(SyncCbHandler<'cb>) -> T,
        C: FnOnce() -> U + 'cb,
        U: 'cb,
    {
        sync_once!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asyncrhonous: a future is returned, and when awaited, it
    /// waits for the callback to complete. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the wait-future.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut_returning<'cb, C, T, U>(
        &mut self,
        callback: C,
    ) -> (T, Listener<U>)
    where
        F: FnMut(SyncCbHandler<'cb>) -> T,
        C: FnOnce() -> U + 'cb,
        U: 'cb,
    {
        sync_once!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asyncrhonous: a future is returned, and when awaited, it
    /// waits for the callback to complete. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the wait-future.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref_returning<'cb, C, T, U>(
        &self,
        callback: C,
    ) -> (T, Listener<U>)
    where
        F: Fn(SyncCbHandler<'cb>) -> T,
        C: FnOnce() -> U + 'cb,
        U: 'cb,
    {
        sync_once!(self, callback)
    }
}

/// Register of oneshot callbacks into an event, where the callback is
/// asyncrhonous (waiting for the callback to complete is also asynchronous).
#[derive(Debug, Clone, Copy)]
pub struct AsyncRegister<F> {
    register_fn: F,
}

impl<F> AsyncRegister<F> {
    /// Creates a new register using an inner register function that can be used
    /// only once. Such function receives callbacks handlers and register
    /// them. Callback handlers are register-internal futures that are awaited
    /// for the event's completion, and that then await the actual callbacks,
    /// i.e. a wrapper for the actual callback.
    pub fn new<'cb, T>(register_fn: F) -> Self
    where
        F: FnOnce(AsyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times, with mutability, however. Such function receives
    /// callbacks handlers and register them. Callback handlers are
    /// register-internal futures that are awaited for the event's
    /// completion, and that then await the actual callbacks,
    /// i.e. a wrapper for the actual callback.
    pub fn new_mut<'cb, T>(register_fn: F) -> Self
    where
        F: FnMut(AsyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times and does not require mutability. Such function receives
    /// callbacks handlers and register them. Callback handlers are
    /// register-internal futures that are awaited for the event's
    /// completion, and that then await the actual callbacks,
    /// i.e. a wrapper for the actual callback.
    pub fn new_ref<'cb, T>(register_fn: F) -> Self
    where
        F: Fn(AsyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asynchronous: a future is returned, and when awaited, it
    /// waits for the callback to complete.
    ///
    /// This method consumes the register.
    ///
    /// # Examples
    ///
    /// ## Dummy Example
    ///
    /// ```no_run
    /// use webio::{callback, task};
    ///
    /// # fn main() {
    /// # task::detach(async {
    /// let register = callback::once::AsyncRegister::new(|callback| {
    ///     task::detach(callback);
    /// });
    /// let result = register.listen(async { 42 }).await;
    /// assert_eq!(result.unwrap(), 42);
    /// # });
    /// # }
    /// ```
    pub fn listen<'cb, A>(self, callback: A) -> Listener<A::Output>
    where
        F: FnOnce(AsyncCbHandler<'cb>),
        A: Future + 'cb,
    {
        let (_, listener) = self.listen_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asynchronous: a future is returned, and when awaited, it
    /// waits for the callback to complete.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut<'cb, A>(&mut self, callback: A) -> Listener<A::Output>
    where
        F: FnMut(AsyncCbHandler<'cb>),
        A: Future + 'cb,
    {
        let (_, listener) = self.listen_mut_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asynchronous: a future is returned, and when awaited, it
    /// waits for the callback to complete.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref<'cb, A>(&self, callback: A) -> Listener<A::Output>
    where
        F: Fn(AsyncCbHandler<'cb>),
        A: Future + 'cb,
    {
        let (_, listener) = self.listen_ref_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asynchronous: a future is returned, and when awaited, it
    /// waits for the callback to complete. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the wait-future.
    ///
    /// This method consumes the register.
    ///
    /// # Examples
    ///
    /// ## Dummy Example
    ///
    /// ```no_run
    /// use webio::{callback, task};
    ///
    /// # fn main() {
    /// # task::detach(async {
    /// let register = callback::once::AsyncRegister::new(|callback| {
    ///     task::detach(callback);
    ///     "my-ret-value"
    /// });
    /// let (ret_value, future) = register.listen_returning(async { 42 });
    /// assert_eq!(ret_value, "my-ret-value");
    /// let result = future.await;
    /// assert_eq!(result.unwrap(), 42);
    /// # });
    /// # }
    /// ```
    pub fn listen_returning<'cb, A, T>(
        self,
        callback: A,
    ) -> (T, Listener<A::Output>)
    where
        F: FnOnce(AsyncCbHandler<'cb>) -> T,
        A: Future + 'cb,
    {
        async_once!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asynchronous: a future is returned, and when awaited, it
    /// waits for the callback to complete. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the wait-future.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut_returning<'cb, A, T>(
        &mut self,
        callback: A,
    ) -> (T, Listener<A::Output>)
    where
        F: FnMut(AsyncCbHandler<'cb>) -> T,
        A: Future + 'cb,
    {
        async_once!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. This
    /// method is asynchronous: a future is returned, and when awaited, it
    /// waits for the callback to complete. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the wait-future.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref_returning<'cb, A, T>(
        &self,
        callback: A,
    ) -> (T, Listener<A::Output>)
    where
        F: Fn(AsyncCbHandler<'cb>) -> T,
        A: Future + 'cb,
    {
        async_once!(self, callback)
    }
}

/// A handle to a oneshot callback registered in an event.
#[derive(Debug)]
pub struct Listener<T> {
    inner: callback::shared::Listener<T>,
}

impl<T> Listener<T> {
    fn new(inner: callback::shared::Listener<T>) -> Self {
        Self { inner }
    }
}

impl<T> Future for Listener<T> {
    type Output = Result<T, callback::Error>;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        match self.inner.receive() {
            Some(output) => task::Poll::Ready(output),
            None => {
                self.inner.subscribe(ctx.waker());
                task::Poll::Pending
            },
        }
    }
}
