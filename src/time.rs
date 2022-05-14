//! This module implements time-related utilities.

mod instant;

use crate::callback;
use js_sys::Function;
use std::{future::Future, pin::Pin, task, time::Duration};
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue};

#[cfg(feature = "stream")]
use futures::stream::Stream;

pub use instant::Instant;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "setTimeout")]
    fn set_timeout(function: &Function, milliseconds: i32) -> JsValue;
    #[wasm_bindgen(js_name = "clearTimeout")]
    fn clear_timeout(timeout_id: &JsValue);

    #[wasm_bindgen(js_name = "setInterval")]
    fn set_interval(function: &Function, milliseconds: i32) -> JsValue;
    #[wasm_bindgen(js_name = "clearInterval")]
    fn clear_interval(interval_id: &JsValue);
}

fn duration_to_millis(duration: Duration) -> i32 {
    duration.as_millis().min(i32::MAX as u128) as i32
}

/// A handle to a [`timeout`] call. The timeout can be waited through `.await`,
/// or it can be cancelled when the handle is dropped without the timeout
/// completing.
pub struct TimeoutHandle {
    listener: callback::once::Listener<()>,
    timeout_id: JsValue,
    _closure: JsValue,
}

impl TimeoutHandle {
    fn new(
        listener: callback::once::Listener<()>,
        timeout_id: JsValue,
        closure: JsValue,
    ) -> Self {
        Self { listener, timeout_id, _closure: closure }
    }
}

impl Future for TimeoutHandle {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        unsafe { self.map_unchecked_mut(|this| &mut this.listener) }
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
    timeout_ms(duration_to_millis(duration))
}

fn timeout_ms(milliseconds: i32) -> TimeoutHandle {
    let register = callback::once::SyncRegister::new(|callback| {
        let closure = Closure::once_into_js(move || callback(()));
        let timeout_id = set_timeout(closure.dyn_ref().unwrap(), milliseconds);
        (timeout_id, closure)
    });

    let ((id, closure), listener) = register.listen_returning(|()| ());

    TimeoutHandle::new(listener, id, closure)
}

/// A handle to an [`interval`] call. An interval can be waited through
/// `.tick().await`, or it can be cancelled when the handle is dropped without
/// the interval completing.
pub struct IntervalHandle {
    listener: callback::multi::Listener<()>,
    interval_id: JsValue,
    _closure: JsValue,
}

impl IntervalHandle {
    fn new(
        listener: callback::multi::Listener<()>,
        interval_id: JsValue,
        closure: JsValue,
    ) -> Self {
        Self { listener, interval_id, _closure: closure }
    }

    /// Ticks for the next interval. This is an asynchronous function.
    pub fn tick<'this>(&'this self) -> IntervalTick<'this> {
        IntervalTick { listener: self.listener.listen_next() }
    }
}

impl Drop for IntervalHandle {
    fn drop(&mut self) {
        clear_interval(&self.interval_id);
    }
}

#[cfg(feature = "stream")]
impl Stream for IntervalHandle {
    type Item = ();

    fn poll_next(
        mut self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.listener).poll_next(ctx)
    }
}

/// A single interval tick that can be awaited.
pub struct IntervalTick<'handle> {
    listener: callback::multi::ListenNext<'handle, ()>,
}

impl<'handle> Future for IntervalTick<'handle> {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        Pin::new(&mut self.listener).poll(ctx).map(Result::unwrap)
    }
}

/// Creates a handle that produces [`Future`]s that, when awaited, are always
/// completed approximately with the same interval between them, they "tick",
/// given by a `duration`.
///
/// ```no_run
/// use std::time::Duration;
/// use webio::time::{interval, Instant};
///
/// # use webio::task;
/// # fn main() {
/// # task::detach(async {
/// let time = Duration::from_millis(100);
/// let handle = interval(time);
/// let then = Instant::now();
///
/// handle.tick().await;
/// let passed = then.elapsed();
/// assert!(passed >= time - Duration::from_millis(50));
/// assert!(passed < time + Duration::from_millis(50));
///
/// handle.tick().await;
/// let passed = then.elapsed();
/// assert!(passed >= time * 2 - Duration::from_millis(50));
/// assert!(passed < time * 2 + Duration::from_millis(50));
///
/// handle.tick().await;
/// let passed = then.elapsed();
/// assert!(passed >= time * 3 - Duration::from_millis(50));
/// assert!(passed < time * 3 + Duration::from_millis(50));
/// # });
/// # }
/// ```
pub fn interval(duration: Duration) -> IntervalHandle {
    interval_ms(duration_to_millis(duration))
}

fn interval_ms(milliseconds: i32) -> IntervalHandle {
    let register = callback::multi::SyncRegister::new(|mut callback| {
        let boxed_callback = Box::new(move || callback(()));
        let closure =
            Closure::wrap(boxed_callback as Box<dyn FnMut()>).into_js_value();
        let timeout_id = set_interval(closure.dyn_ref().unwrap(), milliseconds);
        (timeout_id, closure)
    });

    let ((id, closure), listener) = register.listen_returning(|()| ());

    IntervalHandle::new(listener, id, closure)
}
