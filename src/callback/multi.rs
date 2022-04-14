//! This module implements conversion from callbacks to futures for callbacks
//! that are called multiple times, i.e. callbacks of events that occur more
//! than once per callback.

use crate::{
    callback,
    panic::{FutureCatchUnwind, Payload, StreamCatchUnwind},
};
use futures::stream::Stream;
use std::{future::Future, panic, pin::Pin, task};

macro_rules! sync_multi {
    ($self:expr, $callback:expr) => {{
        let (notifier, inner_listener) = callback::shared::channel();

        let handler = Box::new(move || {
            let result =
                panic::catch_unwind(panic::AssertUnwindSafe(&mut $callback));
            match result {
                Ok(data) => notifier.success(data),
                Err(payload) => notifier.panicked(payload),
            }
        });
        let ret = ($self.register_fn)(handler as SyncCbHandler);

        (ret, Listener::new(inner_listener))
    }};
}

macro_rules! async_multi {
    ($self:expr, $callback:expr) => {{
        let (notifier, inner_listener) = callback::shared::channel();

        let handler = Box::new(move || {
            let future = $callback();
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
        let ret = ($self.register_fn)(handler as AsyncCbHandler);

        (ret, Listener::new(inner_listener))
    }};
}

macro_rules! streaming_multi {
    ($self:expr, $callback:expr) => {{
        struct Handler<S>
        where
            S: Stream,
        {
            inner: StreamCatchUnwind<S>,
            notifier: Option<callback::shared::Notifier<S::Item>>,
        }

        impl<S> Handler<S>
        where
            S: Stream,
        {
            fn new(
                inner: S,
                notifier: callback::shared::Notifier<S::Item>,
            ) -> Self {
                Self {
                    inner: StreamCatchUnwind::new(inner),
                    notifier: Some(notifier),
                }
            }
        }

        impl<S> Stream for Handler<S>
        where
            S: Stream,
        {
            type Item = ();

            fn poll_next(
                self: Pin<&mut Self>,
                ctx: &mut task::Context<'_>,
            ) -> task::Poll<Option<Self::Item>> {
                let Self { ref mut notifier, ref mut inner } =
                    unsafe { self.get_unchecked_mut() };

                let some_notifier = match notifier {
                    Some(notif) => notif,
                    None => return task::Poll::Ready(None),
                };
                match unsafe { Pin::new_unchecked(inner) }.poll_next(ctx) {
                    task::Poll::Pending => task::Poll::Pending,
                    task::Poll::Ready(None) => {
                        *notifier = None;
                        task::Poll::Ready(None)
                    },
                    task::Poll::Ready(Some(Ok(item))) => {
                        some_notifier.success(item);
                        task::Poll::Ready(Some(()))
                    },
                    task::Poll::Ready(Some(Err(payload))) => {
                        some_notifier.panicked(payload);
                        task::Poll::Ready(Some(()))
                    },
                }
            }
        }

        let (notifier, inner_listener) = callback::shared::channel();
        let handler = Box::pin(Handler::new($callback, notifier));
        let ret = ($self.register_fn)(handler as StreamingCbHandler);
        (ret, Listener::new(inner_listener))
    }};
}

/// The type of synchronous, multi-call callback handlers (i.e. the handler that
/// calls callbacks): a boxed mutable function, a wrapper over callbacks.
pub type SyncCbHandler<'cb> = Box<dyn FnMut() + 'cb>;

/// The type of futures used in asynchronous, multi-call callback handlers (i.e.
/// the handler that calls callbacks): a boxed future.
pub type AsyncCbHandlerFuture<'fut> = Pin<Box<dyn Future<Output = ()> + 'fut>>;

/// The type of asynchronous, multi-call callback handlers (i.e. the handler
/// that calls callbacks): a boxed mutable function yielding boxed futures, a
/// wrapper over callbacks.
pub type AsyncCbHandler<'cb, 'fut> =
    Box<dyn FnMut() -> AsyncCbHandlerFuture<'fut> + 'cb>;

/// The type of streaming asyncrhonous, multi-call callbackhandlers (i.e. the
/// handler that calls stream callbacks): a boxed stream, a wrapper over
/// callbacks.
pub type StreamingCbHandler<'cb> = Pin<Box<dyn Stream<Item = ()> + 'cb>>;

/// Register of multi-call callbacks into an event, where the callback is
/// syncrhonous (though waiting for the callback to complete is still
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
    pub fn new<'cb, T>(register_fn: F) -> Self
    where
        F: FnOnce(SyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times, with mutability, however. Such function receives
    /// callbacks handlers and register them. Callback handlers are functions
    /// that are called when the event completes, and then, they call the actual
    /// callbacks, i.e. a wrapper for the actual callback.
    pub fn new_mut<'cb, T>(register_fn: F) -> Self
    where
        F: FnMut(SyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times, without mutability. Such function receives callbacks
    /// handlers and register them. Callback handlers are functions that are
    /// called when the event completes, and then, they call the actual
    /// callbacks, i.e. a wrapper for the actual callback.
    pub fn new_ref<'cb, T>(register_fn: F) -> Self
    where
        F: Fn(SyncCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event.
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
    /// fn fake_multi_event<F>(limit: u32, mut callback: F)
    /// where
    ///     F: FnMut() + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback();
    ///             task::yield_now().await;
    ///             fake_multi_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    /// # task::detach(async {
    /// let register = callback::multi::SyncRegister::new(|callback| {
    ///     fake_multi_event(3, callback);
    /// });
    ///
    /// let mut count = 0;
    /// let listener = register.listen(move || {
    ///     let output = count;
    ///     count += 1;
    ///     output
    /// });
    ///
    /// assert_eq!(listener.next().await.unwrap(), 0);
    /// assert_eq!(listener.next().await.unwrap(), 1);
    /// assert_eq!(listener.next().await.unwrap(), 2);
    /// # });
    /// # }
    /// ```
    pub fn listen<'cb, C, U>(self, callback: C) -> Listener<U>
    where
        F: FnOnce(SyncCbHandler<'cb>),
        C: FnMut() -> U + 'cb,
        U: 'cb,
    {
        let (_, listener) = self.listen_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut<'cb, C, U>(&mut self, callback: C) -> Listener<U>
    where
        F: FnMut(SyncCbHandler<'cb>),
        C: FnMut() -> U + 'cb,
        U: 'cb,
    {
        let (_, listener) = self.listen_mut_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref<'cb, C, U>(&self, callback: C) -> Listener<U>
    where
        F: Fn(SyncCbHandler<'cb>),
        C: FnMut() -> U + 'cb,
        U: 'cb,
    {
        let (_, listener) = self.listen_ref_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event. The register can also return a value, and
    /// so, this method returns both the register's return value and the
    /// listener.
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
    /// fn fake_multi_event<F>(limit: u32, mut callback: F)
    /// where
    ///     F: FnMut() + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback();
    ///             task::yield_now().await;
    ///             fake_multi_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    /// # task::detach(async {
    /// let register = callback::multi::SyncRegister::new(|callback| {
    ///     fake_multi_event(3, callback);
    ///     "my-return-value-blabla"
    /// });
    ///
    /// let mut count = 0;
    /// let (ret, listener) = register.listen_returning(move || {
    ///     let output = count;
    ///     count += 1;
    ///     output
    /// });
    ///
    /// assert_eq!(ret, "my-return-value-blabla");
    /// assert_eq!(listener.next().await.unwrap(), 0);
    /// assert_eq!(listener.next().await.unwrap(), 1);
    /// assert_eq!(listener.next().await.unwrap(), 2);
    /// # });
    /// # }
    /// ```
    pub fn listen_returning<'cb, C, T, U>(
        self,
        mut callback: C,
    ) -> (T, Listener<U>)
    where
        F: FnOnce(SyncCbHandler<'cb>) -> T,
        C: FnMut() -> U + 'cb,
        U: 'cb,
    {
        sync_multi!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event. The register can also return a value, and
    /// so, this method returns both the register's return value and the
    /// listener.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut_returning<'cb, C, T, U>(
        &mut self,
        mut callback: C,
    ) -> (T, Listener<U>)
    where
        F: FnMut(SyncCbHandler<'cb>) -> T,
        C: FnMut() -> U + 'cb,
        U: 'cb,
    {
        sync_multi!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event. The register can also return a value, and
    /// so, this method returns both the register's return value and the
    /// listener.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref_returning<'cb, C, T, U>(
        &self,
        mut callback: C,
    ) -> (T, Listener<U>)
    where
        F: Fn(SyncCbHandler<'cb>) -> T,
        C: FnMut() -> U + 'cb,
        U: 'cb,
    {
        sync_multi!(self, callback)
    }
}

/// Register of multi-call callbacks into an event, where the callback is
/// asyncrhonous (waiting for the callback to complete is also asynchronous).
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
    pub fn new<'cb, 'fut, T>(register_fn: F) -> Self
    where
        'fut: 'cb,
        F: FnOnce(AsyncCbHandler<'cb, 'fut>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// many times, requiring mutability, however. Such function receives
    /// callbacks handlers and register them. Callback handlers are
    /// register-internal mutable functions that return futures, and those
    /// futures are awaited when the event completes, and then, they await
    /// the actual callbacks, i.e. a wrapper for the actual callback.
    pub fn new_mut<'cb, 'fut, T>(register_fn: F) -> Self
    where
        'fut: 'cb,
        F: FnMut(AsyncCbHandler<'cb, 'fut>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// many times and does not require mutability, however. Such function
    /// receives callbacks handlers and register them. Callback handlers are
    /// register-internal mutable functions that return futures, and those
    /// futures are awaited when the event completes, and then, they await
    /// the actual callbacks, i.e. a wrapper for the actual callback.
    pub fn new_ref<'cb, 'fut, T>(register_fn: F) -> Self
    where
        'fut: 'cb,
        F: Fn(AsyncCbHandler<'cb, 'fut>) -> T,
    {
        Self { register_fn }
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event.
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
    /// fn fake_multi_event<A, F>(limit: u32, mut callback: F)
    /// where
    ///     F: FnMut() -> A + 'static,
    ///     A: Future + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback().await;
    ///             task::yield_now().await;
    ///             fake_multi_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    /// # task::detach(async {
    /// let register = callback::multi::AsyncRegister::new(|callback| {
    ///     fake_multi_event(3, callback);
    /// });
    ///
    /// let mut count = 0;
    /// let listener = register.listen(move || {
    ///     let output = count;
    ///     count += 1;
    ///     async move { output }
    /// });
    ///
    /// assert_eq!(listener.next().await.unwrap(), 0);
    /// assert_eq!(listener.next().await.unwrap(), 1);
    /// assert_eq!(listener.next().await.unwrap(), 2);
    /// # });
    /// # }
    /// ```
    pub fn listen<'cb, 'fut, C, A>(self, callback: C) -> Listener<A::Output>
    where
        'fut: 'cb,
        F: FnOnce(AsyncCbHandler<'cb, 'fut>),
        C: FnMut() -> A + 'cb,
        A: Future + 'fut,
    {
        let (_, listener) = self.listen_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut<'cb, 'fut, C, A>(
        &mut self,
        callback: C,
    ) -> Listener<A::Output>
    where
        'fut: 'cb,
        F: FnMut(AsyncCbHandler<'cb, 'fut>),
        C: FnMut() -> A + 'cb,
        A: Future + 'fut,
    {
        let (_, listener) = self.listen_mut_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref<'cb, 'fut, C, A>(
        &self,
        callback: C,
    ) -> Listener<A::Output>
    where
        'fut: 'cb,
        F: Fn(AsyncCbHandler<'cb, 'fut>),
        C: FnMut() -> A + 'cb,
        A: Future + 'fut,
    {
        let (_, listener) = self.listen_ref_returning(callback);
        listener
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event. The register can also return a
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
    /// fn fake_multi_event<A, F>(limit: u32, mut callback: F)
    /// where
    ///     F: FnMut() -> A + 'static,
    ///     A: Future + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback().await;
    ///             task::yield_now().await;
    ///             fake_multi_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    /// # task::detach(async {
    /// let register = callback::multi::AsyncRegister::new(|callback| {
    ///     fake_multi_event(3, callback);
    ///     "returned blabla foo"
    /// });
    ///
    /// let mut count = 0;
    /// let (ret, listener) = register.listen_returning(move || {
    ///     let output = count;
    ///     count += 1;
    ///     async move { output }
    /// });
    ///
    /// assert_eq!(ret, "returned blabla foo");
    /// assert_eq!(listener.next().await.unwrap(), 0);
    /// assert_eq!(listener.next().await.unwrap(), 1);
    /// assert_eq!(listener.next().await.unwrap(), 2);
    /// # });
    /// # }
    /// ```
    pub fn listen_returning<'cb, 'fut, C, A, T>(
        self,
        mut callback: C,
    ) -> (T, Listener<A::Output>)
    where
        'fut: 'cb,
        F: FnOnce(AsyncCbHandler<'cb, 'fut>) -> T,
        C: FnMut() -> A + 'cb,
        A: Future + 'fut,
    {
        async_multi!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the listener.
    ///
    /// This method does not consume the register, requiring mutability,
    /// however.
    pub fn listen_mut_returning<'cb, 'fut, C, A, T>(
        &mut self,
        mut callback: C,
    ) -> (T, Listener<A::Output>)
    where
        'fut: 'cb,
        F: FnMut(AsyncCbHandler<'cb, 'fut>) -> T,
        C: FnMut() -> A + 'cb,
        A: Future + 'fut,
    {
        async_multi!(self, callback)
    }

    /// Registers a callback and lets it listen for the target event. A listener
    /// is returned, and calling `[Listener::next]` yields a future that waits
    /// for an occurence of the event. The register can also return a
    /// value, and so, this method returns both the register's return value
    /// and the listener.
    ///
    /// This method does not consume the register and does not require
    /// mutability.
    pub fn listen_ref_returning<'cb, 'fut, C, A, T>(
        &self,
        mut callback: C,
    ) -> (T, Listener<A::Output>)
    where
        'fut: 'cb,
        F: Fn(AsyncCbHandler<'cb, 'fut>) -> T,
        C: FnMut() -> A + 'cb,
        A: Future + 'fut,
    {
        async_multi!(self, callback)
    }
}

/// Register of multi-call callbacks into an event, where the callback is an
/// asyncrhonous streaming (waiting for the callback to complete is also
/// asynchronous).
#[derive(Debug, Clone, Copy)]
pub struct StreamingRegister<F> {
    register_fn: F,
}

impl<F> StreamingRegister<F> {
    /// Creates a new register using an inner register function that can be used
    /// only once (though the callback might be called multiple times). Such
    /// function receives callbacks handlers and register them. Callback
    /// handlers are register-internal streams that are awaited for when the
    /// event completes, and then, they await the actual callbacks, i.e. a
    /// wrapper for the actual callback.
    pub fn new<'cb, T>(register_fn: F) -> Self
    where
        F: FnOnce(StreamingCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times, requiring mutability, however. Such function receives
    /// callbacks handlers and register them. Callback handlers are
    /// register-internal streams that are awaited for when the event completes,
    /// and then, they await the actual callbacks, i.e. a wrapper for the actual
    /// callback.
    pub fn new_mut<'cb, T>(register_fn: F) -> Self
    where
        F: FnMut(StreamingCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Creates a new register using an inner register function that can be used
    /// multiple times and does not require mutability. Such function receives
    /// callbacks handlers and register them. Callback handlers are
    /// register-internal streams that are awaited for when the event completes,
    /// and then, they await the actual callbacks, i.e. a wrapper for the actual
    /// callback.
    pub fn new_ref<'cb, T>(register_fn: F) -> Self
    where
        F: Fn(StreamingCbHandler<'cb>) -> T,
    {
        Self { register_fn }
    }

    /// Registers a callback (a stream) and lets it listen for the target event.
    /// A listener is returned, and calling `[Listener::next]` yields a
    /// future that waits for an occurence of the event. Whenever the
    /// callback returns `None` on [`Stream::poll_next`], the whole listener
    /// stops and awaiting next events will yield
    /// [`callback::Error::Cancelled`].
    ///
    /// This method consumes the register.
    ///
    /// # Examples
    ///
    /// ## Dummy Example
    ///
    /// ```no_run
    /// use futures::stream::{Stream, StreamExt};
    /// use std::{
    ///     future::Future,
    ///     pin::Pin,
    ///     task::{Context, Poll},
    /// };
    /// use webio::{callback, task};
    ///
    /// fn streaming_multi_event<S>(limit: u32, mut callback: Pin<Box<S>>)
    /// where
    ///     S: Stream + ?Sized + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback.next().await;
    ///             task::yield_now().await;
    ///             streaming_multi_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    ///
    /// #[derive(Debug)]
    /// struct IncrementStream {
    ///     value: u32,
    ///     curr_done: bool,
    /// }
    ///
    /// impl Default for IncrementStream {
    ///     fn default() -> Self {
    ///         Self { value: 0, curr_done: false }
    ///     }
    /// }
    ///
    /// impl Stream for IncrementStream {
    ///     type Item = u32;
    ///
    ///     fn poll_next(
    ///         self: Pin<&mut Self>,
    ///         _ctx: &mut Context<'_>,
    ///     ) -> Poll<Option<Self::Item>> {
    ///         let this = self.get_mut();
    ///         if this.curr_done {
    ///             let output = this.value;
    ///             this.curr_done = false;
    ///             this.value = output.wrapping_add(1);
    ///             Poll::Ready(Some(output))
    ///         } else {
    ///             this.curr_done = true;
    ///             Poll::Pending
    ///         }
    ///     }
    /// }
    ///
    /// # fn main() {
    /// # task::detach(async {
    /// let register = callback::multi::StreamingRegister::new(|callback| {
    ///     streaming_multi_event(3, callback);
    /// });
    ///
    /// let listener = register.listen(IncrementStream::default());
    ///
    /// assert_eq!(listener.next().await.unwrap(), 0);
    /// assert_eq!(listener.next().await.unwrap(), 1);
    /// assert_eq!(listener.next().await.unwrap(), 2);
    /// # });
    /// # }
    /// ```
    pub fn listen<'cb, S>(self, callback: S) -> Listener<S::Item>
    where
        F: FnOnce(StreamingCbHandler<'cb>),
        S: Stream + 'cb,
    {
        let (_, listener) = self.listen_returning(callback);
        listener
    }

    /// Registers a callback (a stream) and lets it listen for the target event.
    /// A listener is returned, and calling `[Listener::next]` yields a
    /// future that waits for an occurence of the event. Whenever the
    /// callback returns `None` on [`Stream::poll_next`], the whole listener
    /// stops and awaiting next events will yield
    /// [`callback::Error::Cancelled`].
    ///
    /// This method does not consume the register, but requires mutability.
    pub fn listen_mut<'cb, S>(&mut self, callback: S) -> Listener<S::Item>
    where
        F: FnMut(StreamingCbHandler<'cb>),
        S: Stream + 'cb,
    {
        let (_, listener) = self.listen_mut_returning(callback);
        listener
    }

    /// Registers a callback (a stream) and lets it listen for the target event.
    /// A listener is returned, and calling `[Listener::next]` yields a
    /// future that waits for an occurence of the event. Whenever the
    /// callback returns `None` on [`Stream::poll_next`], the whole listener
    /// stops and awaiting next events will yield
    /// [`callback::Error::Cancelled`].
    ///
    /// This method does not consume the register and does not requires
    /// mutability.
    pub fn listen_ref<'cb, S>(&self, callback: S) -> Listener<S::Item>
    where
        F: Fn(StreamingCbHandler<'cb>),
        S: Stream + 'cb,
    {
        let (_, listener) = self.listen_ref_returning(callback);
        listener
    }

    /// Registers a callback (a stream) and lets it listen for the target event.
    /// A listener is returned, and calling `[Listener::next]` yields a
    /// future that waits for an occurence of the event. Whenever the
    /// callback returns `None` on [`Stream::poll_next`], the whole listener
    /// stops and awaiting next events will yield
    /// [`callback::Error::Cancelled`]. The register can also return a value,
    /// and so, this method returns both the register's return value and the
    /// listener.
    ///
    /// This method consumes the register.
    ///
    /// # Examples
    ///
    /// ## Dummy Example
    ///
    /// ```no_run
    /// use futures::stream::{Stream, StreamExt};
    /// use std::{
    ///     future::Future,
    ///     pin::Pin,
    ///     task::{Context, Poll},
    /// };
    /// use webio::{callback, task};
    ///
    /// fn streaming_multi_event<S>(limit: u32, mut callback: Pin<Box<S>>)
    /// where
    ///     S: Stream + ?Sized + 'static,
    /// {
    ///     if let Some(new_limit) = limit.checked_sub(1) {
    ///         task::detach(async move {
    ///             callback.next().await;
    ///             task::yield_now().await;
    ///             streaming_multi_event(new_limit, callback);
    ///         });
    ///     }
    /// }
    ///
    /// #[derive(Debug)]
    /// struct IncrementStream {
    ///     value: u32,
    ///     curr_done: bool,
    /// }
    ///
    /// impl Default for IncrementStream {
    ///     fn default() -> Self {
    ///         Self { value: 0, curr_done: false }
    ///     }
    /// }
    ///
    /// impl Stream for IncrementStream {
    ///     type Item = u32;
    ///
    ///     fn poll_next(
    ///         self: Pin<&mut Self>,
    ///         _ctx: &mut Context<'_>,
    ///     ) -> Poll<Option<Self::Item>> {
    ///         let this = self.get_mut();
    ///         if this.curr_done {
    ///             let output = this.value;
    ///             this.curr_done = false;
    ///             this.value = output.wrapping_add(1);
    ///             Poll::Ready(Some(output))
    ///         } else {
    ///             this.curr_done = true;
    ///             Poll::Pending
    ///         }
    ///     }
    /// }
    ///
    /// # fn main() {
    /// # task::detach(async {
    /// let register = callback::multi::StreamingRegister::new(|callback| {
    ///     streaming_multi_event(3, callback);
    ///     "some-ret-string"
    /// });
    ///
    /// let (ret, listener) = register.listen_returning(IncrementStream::default());
    ///
    /// assert_eq!(ret, "some-ret-string");
    /// assert_eq!(listener.next().await.unwrap(), 0);
    /// assert_eq!(listener.next().await.unwrap(), 1);
    /// assert_eq!(listener.next().await.unwrap(), 2);
    /// # });
    /// # }
    /// ```
    pub fn listen_returning<'cb, S, T>(
        self,
        callback: S,
    ) -> (T, Listener<S::Item>)
    where
        F: FnOnce(StreamingCbHandler<'cb>) -> T,
        S: Stream + 'cb,
    {
        streaming_multi!(self, callback)
    }

    /// Registers a callback (a stream) and lets it listen for the target event.
    /// A listener is returned, and calling `[Listener::next]` yields a
    /// future that waits for an occurence of the event. Whenever the
    /// callback returns `None` on [`Stream::poll_next`], the whole listener
    /// stops and awaiting next events will yield
    /// [`callback::Error::Cancelled`]. The register can also return a value,
    /// and so, this method returns both the register's return value and the
    /// listener.
    ///
    /// This method does not consume the register but requires mutability.
    pub fn listen_mut_returning<'cb, S, T>(
        &mut self,
        callback: S,
    ) -> (T, Listener<S::Item>)
    where
        F: FnMut(StreamingCbHandler<'cb>) -> T,
        S: Stream + 'cb,
    {
        streaming_multi!(self, callback)
    }

    /// Registers a callback (a stream) and lets it listen for the target event.
    /// A listener is returned, and calling `[Listener::next]` yields a
    /// future that waits for an occurence of the event. Whenever the
    /// callback returns `None` on [`Stream::poll_next`], the whole listener
    /// stops and awaiting next events will yield
    /// [`callback::Error::Cancelled`]. The register can also return a value,
    /// and so, this method returns both the register's return value and the
    /// listener.
    ///
    /// This method does not consume the register and does not requires
    /// mutability.
    pub fn listen_ref_returning<'cb, S, T>(
        &self,
        callback: S,
    ) -> (T, Listener<S::Item>)
    where
        F: Fn(StreamingCbHandler<'cb>) -> T,
        S: Stream + 'cb,
    {
        streaming_multi!(self, callback)
    }
}

/// A handle to a multi-call callback registered in an event. Typically, the
/// [`Listener`] is used with the [`Listener::next`] method for awaiting next
/// occurences of an event, but it can also be used as a stream.
#[derive(Debug)]
pub struct Listener<T> {
    inner: callback::shared::Listener<T>,
}

impl<T> Listener<T> {
    fn new(inner: callback::shared::Listener<T>) -> Self {
        Self { inner }
    }

    /// Creates a future that waits for the next occurence of the event.
    pub fn next<'this>(&'this self) -> ListenNext<'this, T> {
        ListenNext::new(self)
    }
}

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
