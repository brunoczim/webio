//! This module implements time-related utilities.

mod instant;

use js_sys::Function;
use std::{cell::Cell, future::Future, rc::Rc, task, time::Duration};
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

struct Shared {
    connected: Cell<bool>,
    waker: Cell<Option<task::Waker>>,
}

impl Shared {
    fn init_connected() -> Self {
        Self { connected: Cell::new(true), waker: Cell::new(None) }
    }
}

struct CallbackHandle {
    shared: Rc<Shared>,
}

impl CallbackHandle {
    fn new(shared: Rc<Shared>) -> Self {
        Self { shared }
    }
}

impl Drop for CallbackHandle {
    fn drop(&mut self) {
        self.shared.connected.set(false);
        if let Some(waker) = self.shared.waker.take() {
            waker.wake();
        }
    }
}

/// A handle to a [`timeout`] call. The timeout can be waited through `.await`,
/// or it can be cancelled when the handle is dropped without the timeout
/// completing.
pub struct TimeoutHandle {
    timeout_id: JsValue,
    shared: Rc<Shared>,
    _closure: JsValue,
}

impl TimeoutHandle {
    fn new(shared: Rc<Shared>, timeout_id: JsValue, closure: JsValue) -> Self {
        Self { shared, timeout_id, _closure: closure }
    }
}

impl Future for TimeoutHandle {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        if self.shared.connected.get() {
            task::Poll::Pending
        } else {
            let mut waker = self.shared.waker.take();
            if waker.is_none() {
                waker = Some(ctx.waker().clone());
            }
            self.shared.waker.set(waker);
            task::Poll::Ready(())
        }
    }
}

impl Drop for TimeoutHandle {
    fn drop(&mut self) {
        self.shared.connected.set(false);
        clear_timeout(&self.timeout_id);
    }
}

/// Creates a [`Future`] that completes only after some duration of time has
/// passed.
pub fn timeout(duration: Duration) -> TimeoutHandle {
    let millis = duration.as_millis().min(i32::MAX as u128) as i32;
    timeout_ms(millis)
}

fn timeout_ms(milliseconds: i32) -> TimeoutHandle {
    let shared = Rc::new(Shared::init_connected());
    let callback_handle = CallbackHandle::new(shared.clone());

    let closure = Closure::once_into_js(Box::new(move || {
        drop(callback_handle);
    }));

    let timeout_id = set_timeout(closure.dyn_ref().unwrap(), milliseconds);

    TimeoutHandle::new(shared, timeout_id, closure)
}
