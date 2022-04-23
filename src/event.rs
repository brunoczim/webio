//! Module for listening and handling JS events from Rust.

use crate::callback;
use std::future::Future;
use wasm_bindgen::{closure::Closure, convert::FromWasmAbi, JsCast, JsValue};
use wasm_bindgen_futures::future_to_promise;

macro_rules! event_type {
    ($ident:ident, $name:literal, $data:ty) => {
        #[doc = concat!(
            "Safe wrapper for adding event listeners for events of type \"",
            $name,
            "\"."
        )]
        #[derive(Debug, Clone, Copy)]
        pub struct $ident;

        impl Type for $ident {
            type Data = $data;

            fn name(&self) -> &str {
                $name
            }
        }
    };
}

/// Raw function for adding event listeners to JS's event targets, using
/// synchronous event listeners. However, this function is asynchronous and a
/// future is returned.
///
/// It is up to the caller to ensure that the `event_type` is correct and
/// generic parameter `E` matches the `event_type`, as well to ensure the
/// `target` supports such `event_type`.
pub fn add_sync_listener_raw<F, E, T>(
    target: &web_sys::EventTarget,
    event_type: &str,
    callback: F,
) -> callback::multi::Listener<T>
where
    F: FnMut(E) -> T + 'static,
    E: FromWasmAbi + 'static,
    T: 'static,
{
    let register = callback::multi::SyncRegister::new(|callback| {
        let boxed_callback = Box::new(callback);
        let closure =
            Closure::wrap(boxed_callback as Box<dyn FnMut(E)>).into_js_value();
        target
            .add_event_listener_with_callback(
                event_type,
                closure.dyn_ref().unwrap(),
            )
            .unwrap();
    });
    register.listen(callback)
}

/// Raw function for adding event listeners to JS's event targets, using
/// asynchronous event listeners. This function is asynchronous and a future is
/// returned.
///
/// It is up to the caller to ensure that the `event_type` is correct and
/// generic parameter `E` matches the `event_type`, as well to ensure the
/// `target` supports such `event_type`.
pub fn add_async_listener_raw<F, E, A>(
    target: &web_sys::EventTarget,
    event_type: &str,
    callback: F,
) -> callback::multi::Listener<A::Output>
where
    F: FnMut(E) -> A + 'static,
    E: FromWasmAbi + 'static,
    A: Future + 'static,
{
    let register = callback::multi::AsyncRegister::new(|mut callback| {
        let boxed_callback = Box::new(move |event_data| {
            let future = callback(event_data);
            let promise = future_to_promise(async move {
                future.await;
                Ok(JsValue::UNDEFINED)
            });
            JsValue::from(promise)
        });
        let closure =
            Closure::wrap(boxed_callback as Box<dyn FnMut(E) -> JsValue>)
                .into_js_value();
        target
            .add_event_listener_with_callback(
                event_type,
                closure.dyn_ref().unwrap(),
            )
            .unwrap();
    });
    register.listen(callback)
}

/// Trait for safe wrappers over JS event types and JS event listening.
///
/// It is up to the implementor to ensure that the `event_type` is correct and
/// associated type `Data` matches `.name()`.
pub trait Type {
    /// Data of an event's occurence, passed to the listener.
    type Data: FromWasmAbi + 'static;

    /// Name of this event type.
    fn name(&self) -> &str;

    /// Adds event listeners to JS's event targets, where events are of this
    /// event type, using synchronous event listeners. However, this function is
    /// asynchronous and a future is returned.
    ///
    /// It is up to the caller to ensure the `target` supports this event type.
    fn add_sync_listener<F, T>(
        &self,
        target: &web_sys::EventTarget,
        callback: F,
    ) -> callback::multi::Listener<T>
    where
        F: FnMut(Self::Data) -> T + 'static,
        T: 'static,
    {
        add_sync_listener_raw(target, self.name(), callback)
    }

    /// Adds event listeners to JS's event targets, where events are of this
    /// event type, using asynchronous event listeners. This function is
    /// asynchronous and a future is returned.
    ///
    /// It is up to the caller to ensure the `target` supports this event type.
    fn add_async_listener<F, A>(
        &self,
        target: &web_sys::EventTarget,
        callback: F,
    ) -> callback::multi::Listener<A::Output>
    where
        F: FnMut(Self::Data) -> A + 'static,
        A: Future + 'static,
    {
        add_async_listener_raw(target, self.name(), callback)
    }
}

event_type!(KeyUp, "keyup", web_sys::KeyEvent);
event_type!(KeyDown, "keydown", web_sys::KeyEvent);
event_type!(Click, "click", web_sys::MouseEvent);
event_type!(MouseDown, "mousedown", web_sys::MouseEvent);
event_type!(MouseUp, "mouseup", web_sys::MouseEvent);
event_type!(MouseEnter, "mouseenter", web_sys::MouseEvent);
event_type!(MouseLeave, "mouseleave", web_sys::MouseEvent);
event_type!(MouseMove, "mousemove", web_sys::MouseEvent);
event_type!(MouseOver, "mouseover", web_sys::MouseEvent);
event_type!(MouseOut, "mouseout", web_sys::MouseEvent);
event_type!(Drag, "drag", web_sys::DragEvent);
event_type!(DragStart, "dragstart", web_sys::DragEvent);
event_type!(DragEnd, "dragend", web_sys::DragEvent);
event_type!(DragEnter, "dragenter", web_sys::DragEvent);
event_type!(DragLeave, "dragleave", web_sys::DragEvent);
event_type!(DragOver, "dragover", web_sys::DragEvent);
event_type!(DragDrop, "drop", web_sys::DragEvent);
event_type!(TouchStart, "touchstart", web_sys::TouchEvent);
event_type!(TouchEnd, "touchend", web_sys::TouchEvent);
event_type!(TouchMove, "touchmove", web_sys::TouchEvent);
event_type!(TouchCancel, "touchcancel", web_sys::TouchEvent);
