//! This module exports items related to task spawning.

mod handle;

use handle::{PanicPayload, Shared, TaskHandle};
use std::{future::Future, panic, pin::Pin, rc::Rc, task};
use wasm_bindgen_futures::spawn_local;

pub use handle::JoinHandle;

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
///
/// # Examples
/// ```ignore
/// use webio::task;
/// use futures::executor::block_on;
/// use futures::join;
///
/// # fn main() {
/// block_on(async {
///     let first_handle = task::spawn(async { 3 });
///     let second_handle = task::spawn(async { 5 });
///     let third_handle = task::spawn(async { 7 });
///     let (first, second, third) = join!(first_handle, second_handle, third_handle);
///     assert_eq!((first.unwrap(), second.unwrap(), third.unwrap()), (3, 5, 7));
/// });
/// # }
/// ```
pub fn spawn<A>(future: A) -> JoinHandle<A::Output>
where
    A: Future + 'static,
{
    let shared = Rc::new(Shared::init_connected());

    let task_handle = TaskHandle::new(shared.clone());
    let join_handle = JoinHandle::new(shared);

    spawn_local(async move {
        let result = CatchUnwind { future }.await;
        match result {
            Ok(data) => task_handle.success(data),
            Err(payload) => task_handle.panicked(payload),
        }
    });

    join_handle
}
