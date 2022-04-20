//! This module implements conversion from callbacks to futures for callbacks
//! that are called multiple times, i.e. callbacks of events that occur more
//! than once per callback.

use crate::{callback, panic::FutureCatchUnwind};

#[cfg(feature = "stream")]
use crate::panic::Payload;
#[cfg(feature = "stream")]
use futures::stream::Stream;

use std::{future::Future, panic, pin::Pin, task};

macro_rules! sync_multi {
    ($self:expr, $callback:expr) => {{
        let (notifier, inner_listener) = callback::shared::channel();

        let handler = Box::new(move |event_data| {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                $callback(event_data)
            }));
            match result {
                Ok(data) => notifier.success(data),
                Err(payload) => notifier.panicked(payload),
            }
        });
        let ret = ($self.register_fn)(handler as SyncCbHandler<_>);

        (ret, Listener::new(inner_listener))
    }};
}

macro_rules! async_multi {
    ($self:expr, $callback:expr) => {{
        let (notifier, inner_listener) = callback::shared::channel();

        let handler = Box::new(move |event_data| {
            let future = $callback(event_data);
            let notifier = notifier.clone();
            let handler_future = Box::pin(async move {
                let result = FutureCatchUnwind::new(future).await;
                match result {
                    Ok(data) => notifier.success(data),
                    Err(payload) => notifier.panicked(payload),
                }
            });
            handler_future as AsyncCbHandlerFuture
        });
        let ret = ($self.register_fn)(handler as AsyncCbHandler<_>);

        (ret, Listener::new(inner_listener))
    }};
}

/// The type of synchronous, multi-call callback handlers (i.e. the handler that
/// calls callbacks): a boxed mutable function, a wrapper over callbacks.
pub type SyncCbHandler<'cb, T> = Box<dyn FnMut(T) + 'cb>;

/// The type of futures used in asynchronous, multi-call callback handlers (i.e.
/// the handler that calls callbacks): a boxed future.
pub type AsyncCbHandlerFuture<'fut> = Pin<Box<dyn Future<Output = ()> + 'fut>>;

/// The type of asynchronous, multi-call callback handlers (i.e. the handler
/// that calls callbacks): a boxed mutable function yielding boxed futures, a
/// wrapper over callbacks.
pub type AsyncCbHandler<'cb, 'fut, T> =
    Box<dyn FnMut(T) -> AsyncCbHandlerFuture<'fut> + 'cb>;

/// Register of multi-call callbacks into an event, where the callback is
/// synchronous (though waiting for the callback to complete is still
/// asynchronous).
#[derive(Debug, Clone, Copy)]
pub struct SyncRegister<F> {
    register_fn: F,
}

impl<F> SyncRegister<F> {
    /// Creates a new register using an inner register function that can be used
    /// only once (although the registed callback can be called multiple times).
    /// Such function receives callbacks handlers and register them. Callback
    /// handlers are functions that are called when the event completes, and
    /// then, they call the actual callbacks, i.e. a wrapper for the actual
    /// callback.
    pub fn new<'cb, T, U>(register_fn: F) -> Self
    where
        F: FnOnce(SyncCbHandler<'cb, T>) -> U,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times, with mutability, however. Such function receives
    /// callbacks handlers and register them. Callback handlers are functions
    /// that are called when the event completes, and then, they call the actual
    /// callbacks, i.e. a wrapper for the actual callback.
    pub fn new_mut<'cb, T, U>(register_fn: F) -> Self
    where
        F: FnMut(SyncCbHandler<'cb, T>) -> U,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times, without mutability. Such function receives callbacks
    /// handlers and register them. Callback handlers are functions that are
    /// called when the event completes, and then, they call the actual
    /// callbacks, i.e. a wrapper for the actual callback.
    pub fn new_ref<'cb, T, U>(register_fn: F) -> Self
    where
        F: Fn(SyncCbHandler<'cb, T>) -> U,
    {
        Self { register_fn }
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event.
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
    /// fn fake_event<F>(limit: u32, mut callback: F)
    /// where
    ///     F: FnMut(u32) + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback(new_limit);
    ///             task::yield_now().await;
    ///             fake_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    /// # task::detach(async {
    /// let register = callback::multi::SyncRegister::new(|callback| {
    ///     fake_event(3, callback);
    /// });
    ///
    /// let listener = register.listen(|exp| 2u32.pow(exp));
    ///
    /// assert_eq!(listener.listen_next().await.unwrap(), 4);
    /// assert_eq!(listener.listen_next().await.unwrap(), 2);
    /// assert_eq!(listener.listen_next().await.unwrap(), 1);
    /// # });
    /// # }
    /// ```
    pub fn listen<'cb, C, T, V>(self, callback: C) -> Listener<V>
    where
        F: FnOnce(SyncCbHandler<'cb, T>),
        C: FnMut(T) -> V + 'cb,
        V: 'cb,
    {
        let (_, listener) = self.listen_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut<'cb, C, T, V>(&mut self, callback: C) -> Listener<V>
    where
        F: FnMut(SyncCbHandler<'cb, T>),
        C: FnMut(T) -> V + 'cb,
        V: 'cb,
    {
        let (_, listener) = self.listen_mut_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref<'cb, C, T, V>(&mut self, callback: C) -> Listener<V>
    where
        F: Fn(SyncCbHandler<'cb, T>),
        C: FnMut(T) -> V + 'cb,
        V: 'cb,
    {
        let (_, listener) = self.listen_ref_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the listener.
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
    /// fn fake_event<F>(limit: u32, mut callback: F)
    /// where
    ///     F: FnMut(u32) + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback(new_limit);
    ///             task::yield_now().await;
    ///             fake_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    /// # task::detach(async {
    /// let register = callback::multi::SyncRegister::new(|callback| {
    ///     fake_event(3, callback);
    ///     "my-return-abc"
    /// });
    ///
    /// let (ret, listener) = register.listen_returning(|exp| 2u32.pow(exp));
    ///
    /// assert_eq!(ret, "my-return-abc");
    /// assert_eq!(listener.listen_next().await.unwrap(), 4);
    /// assert_eq!(listener.listen_next().await.unwrap(), 2);
    /// assert_eq!(listener.listen_next().await.unwrap(), 1);
    /// # });
    /// # }
    /// ```
    pub fn listen_returning<'cb, C, T, U, V>(
        self,
        mut callback: C,
    ) -> (U, Listener<V>)
    where
        F: FnOnce(SyncCbHandler<'cb, T>) -> U,
        C: FnMut(T) -> V + 'cb,
        V: 'cb,
    {
        sync_multi!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the listener.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut_returning<'cb, C, T, U, V>(
        &mut self,
        mut callback: C,
    ) -> (U, Listener<V>)
    where
        F: FnMut(SyncCbHandler<'cb, T>) -> U,
        C: FnMut(T) -> V + 'cb,
        V: 'cb,
    {
        sync_multi!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the listener.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref_returning<'cb, C, T, U, V>(
        &self,
        mut callback: C,
    ) -> (U, Listener<V>)
    where
        F: Fn(SyncCbHandler<'cb, T>) -> U,
        C: FnMut(T) -> V + 'cb,
        V: 'cb,
    {
        sync_multi!(self, callback)
    }
}

/// Register of multi-call callbacks into an event, where the callback is
/// asynchronous (waiting for the callback to complete is also asynchronous).
#[derive(Debug, Clone, Copy)]
pub struct AsyncRegister<F> {
    register_fn: F,
}

impl<F> AsyncRegister<F> {
    /// Creates a new register using an inner register function that can be used
    /// only once (though the callback might be called multiple times). Such
    /// function receives callbacks handlers and register them. Callback
    /// handlers are register-internal mutable functions that
    /// return futures, and those futures are awaited when the event completes,
    /// and then, they await the actual callbacks, i.e. a wrapper for the actual
    /// callback.
    pub fn new<'cb, 'fut, T, U>(register_fn: F) -> Self
    where
        'fut: 'cb,
        F: FnOnce(AsyncCbHandler<'cb, 'fut, T>) -> U,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// many times, requiring mutability, however. Such function receives
    /// callbacks handlers and register them. Callback handlers are
    /// register-internal mutable functions that return futures, and those
    /// futures are awaited when the event completes, and then, they await
    /// the actual callbacks, i.e. a wrapper for the actual callback.
    pub fn new_mut<'cb, 'fut, T, U>(register_fn: F) -> Self
    where
        'fut: 'cb,
        F: FnMut(AsyncCbHandler<'cb, 'fut, T>) -> U,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// many times and does not require mutability, however. Such function
    /// receives callbacks handlers and register them. Callback handlers are
    /// register-internal mutable functions that return futures, and those
    /// futures are awaited when the event completes, and then, they await
    /// the actual callbacks, i.e. a wrapper for the actual callback.
    pub fn new_ref<'cb, 'fut, T, U>(register_fn: F) -> Self
    where
        'fut: 'cb,
        F: Fn(AsyncCbHandler<'cb, 'fut, T>) -> U,
    {
        Self { register_fn }
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event.
    ///
    /// This method consumes the register.
    ///
    /// # Examples
    ///
    /// ## Dummy Example
    ///
    /// ```no_run
    /// use webio::{callback, task};
    /// use std::future::Future;
    ///
    /// # fn main() {
    /// fn fake_event<A, F>(limit: u32, mut callback: F)
    /// where
    ///     F: FnMut(u32) -> A + 'static,
    ///     A: Future + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback(new_limit).await;
    ///             task::yield_now().await;
    ///             fake_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    /// # task::detach(async {
    /// let register = callback::multi::AsyncRegister::new(|callback| {
    ///     fake_event(3, callback);
    /// });
    ///
    /// let listener = register.listen(|exp| async move { 2u32.pow(exp) });
    ///
    /// assert_eq!(listener.listen_next().await.unwrap(), 4);
    /// assert_eq!(listener.listen_next().await.unwrap(), 2);
    /// assert_eq!(listener.listen_next().await.unwrap(), 1);
    /// # });
    /// # }
    /// ```
    pub fn listen<'cb, 'fut, C, T, A>(self, callback: C) -> Listener<A::Output>
    where
        'fut: 'cb,
        F: FnOnce(AsyncCbHandler<'cb, 'fut, T>),
        C: FnMut(T) -> A + 'cb,
        A: Future + 'fut,
    {
        let (_, listener) = self.listen_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut<'cb, 'fut, C, T, A>(
        &mut self,
        callback: C,
    ) -> Listener<A::Output>
    where
        'fut: 'cb,
        F: FnMut(AsyncCbHandler<'cb, 'fut, T>),
        C: FnMut(T) -> A + 'cb,
        A: Future + 'fut,
    {
        let (_, listener) = self.listen_mut_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref<'cb, 'fut, C, T, A>(
        &self,
        callback: C,
    ) -> Listener<A::Output>
    where
        'fut: 'cb,
        F: Fn(AsyncCbHandler<'cb, 'fut, T>),
        C: FnMut(T) -> A + 'cb,
        A: Future + 'fut,
    {
        let (_, listener) = self.listen_ref_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the listener.
    ///
    /// This method consumes the register.
    ///
    /// # Examples
    ///
    /// ## Dummy Example
    ///
    /// ```no_run
    /// use webio::{callback, task};
    /// use std::future::Future;
    ///
    /// # fn main() {
    /// fn fake_event<A, F>(limit: u32, mut callback: F)
    /// where
    ///     F: FnMut(u32) -> A + 'static,
    ///     A: Future + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback(new_limit).await;
    ///             task::yield_now().await;
    ///             fake_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    /// # task::detach(async {
    /// let register = callback::multi::AsyncRegister::new(|callback| {
    ///     fake_event(3, callback);
    ///     "my-return-abc"
    /// });
    ///
    /// let (ret, listener) =
    ///     register.listen_returning(|exp| async move { 2u32.pow(exp) });
    ///
    /// assert_eq!(ret, "my-return-abc");
    /// assert_eq!(listener.listen_next().await.unwrap(), 4);
    /// assert_eq!(listener.listen_next().await.unwrap(), 2);
    /// assert_eq!(listener.listen_next().await.unwrap(), 1);
    /// # });
    /// # }
    /// ```
    pub fn listen_returning<'cb, 'fut, C, T, U, A>(
        self,
        mut callback: C,
    ) -> (U, Listener<A::Output>)
    where
        'fut: 'cb,
        F: FnOnce(AsyncCbHandler<'cb, 'fut, T>) -> U,
        C: FnMut(T) -> A + 'cb,
        A: Future + 'fut,
    {
        async_multi!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the listener.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut_returning<'cb, 'fut, C, T, U, A>(
        &mut self,
        mut callback: C,
    ) -> (U, Listener<A::Output>)
    where
        'fut: 'cb,
        F: FnMut(AsyncCbHandler<'cb, 'fut, T>) -> U,
        C: FnMut(T) -> A + 'cb,
        A: Future + 'fut,
    {
        async_multi!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::listen_next]` yields a future that
    /// waits for an occurence of the event. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the listener.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref_returning<'cb, 'fut, C, T, U, A>(
        &self,
        mut callback: C,
    ) -> (U, Listener<A::Output>)
    where
        'fut: 'cb,
        F: Fn(AsyncCbHandler<'cb, 'fut, T>) -> U,
        C: FnMut(T) -> A + 'cb,
        A: Future + 'fut,
    {
        async_multi!(self, callback)
    }
}

/// A handle to a multi-call callback registered in an event. Typically, the
/// [`Listener`] is used with the [`Listener::listen_next`] method for awaiting
/// next occurences of an event, but it can also be used as a stream.
#[derive(Debug)]
pub struct Listener<T> {
    inner: callback::shared::Listener<T>,
}

impl<T> Listener<T> {
    fn new(inner: callback::shared::Listener<T>) -> Self {
        Self { inner }
    }

    /// Creates a future that waits for the next occurence of the event.
    pub fn listen_next<'this>(&'this self) -> ListenNext<'this, T> {
        ListenNext::new(self)
    }
}

#[cfg(feature = "stream")]
impl<T> Stream for Listener<T> {
    type Item = Result<T, Payload>;

    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        match self.inner.receive() {
            Some(Ok(output)) => task::Poll::Ready(Some(Ok(output))),
            Some(Err(callback::Error::Panicked(payload))) => {
                task::Poll::Ready(Some(Err(payload)))
            },
            Some(Err(callback::Error::Cancelled)) => task::Poll::Ready(None),
            None => {
                self.inner.subscribe(ctx.waker());
                task::Poll::Pending
            },
        }
    }
}

/// A handle to wait for the single next occurence of an event and a registered
/// callback.
#[derive(Debug)]
pub struct ListenNext<'list, T> {
    listener: &'list Listener<T>,
}

impl<'list, T> ListenNext<'list, T> {
    fn new(listener: &'list Listener<T>) -> Self {
        Self { listener }
    }
}

impl<'list, T> Future for ListenNext<'list, T> {
    type Output = Result<T, callback::Error>;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        match self.listener.inner.receive() {
            Some(output) => task::Poll::Ready(output),
            None => {
                self.listener.inner.subscribe(ctx.waker());
                task::Poll::Pending
            },
        }
    }
}
