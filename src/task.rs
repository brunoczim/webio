//! This module exports items related to task spawning.

mod handle;

use crate::panic::CatchUnwind;
use handle::{Shared, TaskHandle};
use std::{future::Future, rc::Rc};
use wasm_bindgen_futures::spawn_local;

pub use handle::JoinHandle;

/// Spawns an asynchronous task in JS event loop.
///
/// # Examples
/// ```ignore
/// use webio::{task, join};
///
/// # fn main() {
/// task::detach(async {
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
        let result = CatchUnwind::new(future).await;
        match result {
            Ok(data) => task_handle.success(data),
            Err(payload) => task_handle.panicked(payload),
        }
    });

    join_handle
}

/// Detaches a future from the current WASM call, but ensures the future
/// completes.
pub fn detach<A>(future: A)
where
    A: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}
