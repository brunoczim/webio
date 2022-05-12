//! Module for listening and handling JS events from Rust.
//!
//! # Examples
//!
//! ## On Click
//!
//! ```no_run
//! use webio::event::Type;
//! use webio::event::Click;
//!
//! # fn main() {
//! # webio::task::detach(async {
//! let document =
//!     web_sys::window().expect("only browser supported").document().unwrap();
//!
//! let element = document.create_element("button").unwrap();
//! document.body().unwrap().append_child(&element).unwrap();
//! let mut count = 0;
//! let listener = Click.add_listener(&element);
//! element.dispatch_event(&web_sys::MouseEvent::new("click").unwrap()).unwrap();
//! listener.listen_next().await.unwrap();
//! element.dispatch_event(&web_sys::MouseEvent::new("click").unwrap()).unwrap();
//! listener.listen_next().await.unwrap();
//! element.dispatch_event(&web_sys::MouseEvent::new("click").unwrap()).unwrap();
//! listener.listen_next().await.unwrap();
//!
//! document.remove_child(&element).unwrap();
//! # });
//! # }
//! ```

use crate::callback;
use wasm_bindgen::{closure::Closure, convert::FromWasmAbi, JsCast};

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

/// Raw function for adding event listeners to JS's event targets. This function
/// is asynchronous and a future is returned.
///
/// It is up to the caller to ensure that the `event_type` is correct and
/// generic parameter `E` matches the `event_type`, as well to ensure the
/// `target` supports such `event_type`.
pub fn add_listener_raw<E>(
    target: &web_sys::EventTarget,
    event_type: &str,
) -> callback::multi::Listener<E>
where
    E: FromWasmAbi + 'static,
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
    register.listen(|evt| evt)
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
    /// event type. This function is asynchronous and a future is returned.
    ///
    /// It is up to the caller to ensure the `target` supports this event type.
    fn add_listener(
        &self,
        target: &web_sys::EventTarget,
    ) -> callback::multi::Listener<Self::Data> where {
        add_listener_raw(target, self.name())
    }
}

event_type!(KeyUp, "keyup", web_sys::KeyboardEvent);
event_type!(KeyDown, "keydown", web_sys::KeyboardEvent);
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
event_type!(TouchStart, "touchstart", web_sys::Event);
event_type!(TouchEnd, "touchend", web_sys::Event);
event_type!(TouchMove, "touchmove", web_sys::Event);
event_type!(TouchCancel, "touchcancel", web_sys::Event);
event_type!(Blur, "blur", web_sys::FocusEvent);
event_type!(Focus, "focus", web_sys::FocusEvent);
event_type!(FocusOut, "focusout", web_sys::FocusEvent);
event_type!(FocusIn, "focusin", web_sys::FocusEvent);
