//! This module implements time-related utilities.

mod instant;

use crate::callback;
use js_sys::Function;
use std::{future::Future, task, time::Duration};
use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast,
    JsValue,
};

pub use instant::Instant;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "setTimeout")]
    fn set_timeout(function: &Function, milliseconds: i32) -> JsValue;
    #[wasm_bindgen(js_name = "clearTimeout")]
    fn clear_timeout(timeout_id: &JsValue);
}

/// A handle to a [`timeout`] call. The timeout can be waited through `.await`,
/// or it can be cancelled when the handle is dropped without the timeout
/// completing.
pub struct TimeoutHandle {
    callback_handle: callback::once::Listener<()>,
    timeout_id: JsValue,
    _closure: JsValue,
}

impl TimeoutHandle {
    fn new(
        callback_handle: callback::once::Listener<()>,
        timeout_id: JsValue,
        closure: JsValue,
    ) -> Self {
        Self { callback_handle, timeout_id, _closure: closure }
    }
}

impl Future for TimeoutHandle {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        unsafe { self.map_unchecked_mut(|this| &mut this.callback_handle) }
            .poll(ctx)
            .map(|result| result.unwrap())
    }
}

impl Drop for TimeoutHandle {
    fn drop(&mut self) {
        clear_timeout(&self.timeout_id);
    }
}

/// Creates a [`Future`] that completes only after some duration of time has
/// passed.
///
/// ```no_run
/// use std::time::Duration;
/// use webio::time::{timeout, Instant};
///
/// # use webio::task;
/// # fn main() {
/// # task::detach(async {
/// let then = Instant::now();
/// let time = Duration::from_millis(200);
/// timeout(time).await;
/// let passed = then.elapsed();
/// assert!(passed >= time);
/// assert!(passed < time + Duration::from_millis(50));
/// # });
/// # }
/// ```
pub fn timeout(duration: Duration) -> TimeoutHandle {
    let millis = duration.as_millis().min(i32::MAX as u128) as i32;
    timeout_ms(millis)
}

fn timeout_ms(milliseconds: i32) -> TimeoutHandle {
    let register = callback::once::SyncRegister::new(|callback| {
        let closure = Closure::once_into_js(callback);
        let timeout_id = set_timeout(closure.dyn_ref().unwrap(), milliseconds);
        (timeout_id, closure)
    });

    let ((id, closure), callback_handle) = register.listen_returning(|| ());

    TimeoutHandle::new(callback_handle, id, closure)
}