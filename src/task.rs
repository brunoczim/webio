//! This module exports items related to task spawning.

use crate::callback;
use std::{error::Error, fmt, future::Future, pin::Pin, task};
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
    let register = callback::once::AsyncRegister::new(|callback| {
        spawn_local(callback(()))
    });
    let callback_handle = register.listen(|()| future);
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
    spawn(async {}).await.unwrap()
}

/// An error that might happen when waiting for a task, typically caused because
/// the task was cancelled.
#[derive(Debug)]
pub struct JoinError {
    cause: callback::Cancelled,
}

impl fmt::Display for JoinError {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        write!(fmtr, "{}", self.cause)
    }
}

impl Error for JoinError {
    fn cause(&self) -> Option<&dyn Error> {
        Some(&self.cause)
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
            .map(|result| result.map_err(|cause| JoinError { cause }))
    }
}
