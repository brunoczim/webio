//! This module exports items related to task spawning.

use crate::{callback, panic::Payload};
use std::{fmt, future::Future, pin::Pin, task};
use wasm_bindgen_futures::spawn_local;

/// Spawns an asynchronous task in JS event loop.
///
/// # Examples
/// ## Simple Tasks
/// ```no_run
/// use webio::{task, join};
///
/// # fn main() {
/// # task::detach(async {
/// let first_handle = task::spawn(async { 3 });
/// let second_handle = task::spawn(async { 5 });
/// let third_handle = task::spawn(async { 7 });
/// let (first, second, third) = join!(first_handle, second_handle, third_handle);
/// assert_eq!((first.unwrap(), second.unwrap(), third.unwrap()), (3, 5, 7));
/// # });
/// # }
/// ```
pub fn spawn<A>(future: A) -> JoinHandle<A::Output>
where
    A: Future + 'static,
{
    let register = callback::once::AsyncRegister::new(spawn_local);
    let callback_handle = register.listen(future);
    JoinHandle::new(callback_handle)
}

/// Detaches a future from the current WASM call, but ensures the future
/// completes.
pub fn detach<A>(future: A)
where
    A: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

/// Yields control back to the event loop once and returns back to execution as
/// soon as possible.
///
/// # Example
///
/// ## Between Asynchronous Functions
/// ```no_run
/// use webio::task;
/// # fn main() {
/// # async fn foo() -> bool { false }
/// # async fn bar() {}
/// # task::detach(async {
/// while foo().await {
///     task::yield_now().await;
///     bar().await;
/// }
/// # });
/// # }
/// ```
pub async fn yield_now() {
    YieldNow::new().await
}

#[derive(Debug)]
struct YieldNow {
    done: bool,
}

impl YieldNow {
    fn new() -> Self {
        Self { done: false }
    }
}

impl Future for YieldNow {
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        if self.done {
            task::Poll::Ready(())
        } else {
            ctx.waker().wake_by_ref();
            self.done = true;
            task::Poll::Pending
        }
    }
}

/// An error that might happen when waiting for a task.
#[derive(Debug)]
pub struct JoinError {
    kind: callback::Error,
}

impl fmt::Display for JoinError {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        write!(fmtr, "{}", self.kind)
    }
}

impl JoinError {
    /// Tests whether the target task was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(&self.kind, callback::Error::Cancelled)
    }

    /// Tests whether the target task panicked.
    pub fn is_panic(&self) -> bool {
        matches!(&self.kind, callback::Error::Panicked(_))
    }

    /// Attempts to convert this error into a panic payload. Fails if the target
    /// task didn't panicked.
    pub fn try_into_panic(self) -> Result<Payload, Self> {
        match self.kind {
            callback::Error::Panicked(payload) => Ok(payload),
            kind => Err(Self { kind }),
        }
    }

    /// Converts this error into a panic payload.
    ///
    /// # Panics
    /// Panics if this error was not caused by panic (e.g. the task was
    /// cancelled).
    pub fn into_panic(self) -> Payload {
        self.try_into_panic().unwrap()
    }
}

/// A handle that allows the caller to join a task (i.e. wait for it to end).
pub struct JoinHandle<T> {
    inner: callback::once::Listener<T>,
}

impl<T> JoinHandle<T> {
    fn new(inner: callback::once::Listener<T>) -> Self {
        Self { inner }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        unsafe { self.map_unchecked_mut(|pinned| &mut pinned.inner) }
            .poll(ctx)
            .map(|result| result.map_err(|kind| JoinError { kind }))
    }
}
