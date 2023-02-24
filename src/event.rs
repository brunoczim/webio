//! Module for listening and handling JS events from Rust.
//!
//! # Examples
//!
//! ## On Click
//!
//! ```no_run
//! use webio::event::EventType;
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
use js_sys::Function;
use std::{future::Future, pin::Pin, task};
use wasm_bindgen::{
    closure::Closure,
    convert::FromWasmAbi,
    JsCast,
    JsValue,
    UnwrapThrowExt,
};
use wasm_bindgen_futures::future_to_promise;
use web_sys::EventTarget;

#[cfg(feature = "stream")]
use futures::stream::Stream;

macro_rules! event_type {
    ($ident:ident, $name:literal, $data:ty) => {
        #[doc = concat!(
            "Safe wrapper for adding event listeners for events of type \"",
            $name,
            "\"."
        )]
        #[derive(Debug, Clone, Copy)]
        pub struct $ident;

        impl EventType for $ident {
            type Data = $data;

            fn name(&self) -> String {
                String::from($name)
            }
        }
    };
}

/// A listener: listens to event occurences. Created by one of
/// [`add_listener_raw`], [`add_listener_with_sync_cb_raw`],
/// [`add_listener_with_async_cb_raw`], [`EventType::add_listener`],
/// [`EventType::add
#[derive(Debug)]
pub struct Listener<T> {
    inner: callback::multi::Listener<T>,
    event_type: String,
    target: EventTarget,
    id: Function,
}

impl<T> Listener<T> {
    fn new(
        inner: callback::multi::Listener<T>,
        target: EventTarget,
        event_type: String,
        id: Function,
    ) -> Self {
        Self { inner, target, event_type, id }
    }

    /// Ticks for the next interval. This is an asynchronous function.
    pub fn listen_next<'this>(&'this self) -> ListenNext<'this, T> {
        ListenNext { listener: self.inner.listen_next() }
    }
}

impl<T> Drop for Listener<T> {
    fn drop(&mut self) {
        self.target
            .remove_event_listener_with_callback(&self.event_type, &self.id)
            .unwrap_throw();
    }
}

#[cfg(feature = "stream")]
impl<T> Stream for Listener<T> {
    type Item = T;

    fn poll_next(
        mut self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(ctx)
    }
}

/// A single interval tick that can be awaited.
pub struct ListenNext<'listener, T> {
    listener: callback::multi::ListenNext<'listener, T>,
}

impl<'listener, T> Future for ListenNext<'listener, T> {
    type Output = Result<T, callback::Cancelled>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        Pin::new(&mut self.listener).poll(ctx)
    }
}

/// Raw function for adding event listeners to JS's event targets. This function
/// is asynchronous and a future is returned.
///
/// It is up to the caller to ensure that the `event_type` is correct and
/// generic parameter `E` matches the `event_type`, as well to ensure the
/// `target` supports such `event_type`.
pub fn add_listener_raw<S, E>(
    target: &EventTarget,
    event_type: S,
) -> Listener<E>
where
    S: Into<String>,
    E: FromWasmAbi + 'static,
{
    add_listener_with_sync_cb_raw(target, event_type, |evt| evt)
}

/// Raw function for adding event listeners to JS's event targets, using
/// synchronous event listeners. However, this function is asynchronous and a
/// future is returned.
///
/// It is up to the caller to ensure that the `event_type` is correct and
/// generic parameter `E` matches the `event_type`, as well to ensure the
/// `target` supports such `event_type`.
pub fn add_listener_with_sync_cb_raw<S, E, F, T>(
    target: &EventTarget,
    event_type: S,
    callback: F,
) -> Listener<T>
where
    S: Into<String>,
    E: FromWasmAbi + 'static,
    F: FnMut(E) -> T + 'static,
    T: 'static,
{
    let event_type = event_type.into();
    let register = callback::multi::SyncRegister::new(|callback| {
        let boxed_callback = Box::new(callback);
        let closure = Closure::wrap(boxed_callback as Box<dyn FnMut(E)>)
            .into_js_value()
            .dyn_into()
            .unwrap();
        target.add_event_listener_with_callback(&event_type, &closure).unwrap();
        closure
    });

    let (id, listener) = register.listen_returning(callback);
    Listener::new(listener, target.clone(), event_type, id)
}

/// Raw function for adding event listeners to JS's event targets, using
/// asynchronous event listeners. This function is asynchronous and a future is
/// returned.
///
/// It is up to the caller to ensure that the `event_type` is correct and
/// generic parameter `E` matches the `event_type`, as well to ensure the
/// `target` supports such `event_type`.
pub fn add_listener_with_async_cb_raw<S, E, F, A>(
    target: &EventTarget,
    event_type: S,
    callback: F,
) -> Listener<A::Output>
where
    E: FromWasmAbi + 'static,
    F: FnMut(E) -> A + 'static,
    A: Future + 'static,
    S: Into<String>,
{
    let event_type = event_type.into();
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
                .into_js_value()
                .dyn_into()
                .unwrap();
        target.add_event_listener_with_callback(&event_type, &closure).unwrap();
        closure
    });

    let (id, listener) = register.listen_returning(callback);
    Listener::new(listener, target.clone(), event_type, id)
}

/// Trait for safe wrappers over JS event types and JS event listening.
///
/// It is up to the implementor to ensure that the `event_type` is correct and
/// associated type `Data` matches `.name()`.
pub trait EventType {
    /// Data of an event's occurence, passed to the listener.
    type Data: FromWasmAbi + 'static;

    /// Name of this event type.
    fn name(&self) -> String;

    /// Adds event listeners to JS's event targets, where events are of this
    /// event type. This function is asynchronous and a future is returned.
    ///
    /// It is up to the caller to ensure the `target` supports this event type.
    fn add_listener(&self, target: &EventTarget) -> Listener<Self::Data> {
        add_listener_raw(target, self.name())
    }

    /// Adds event listeners to JS's event targets, where events are of this
    /// event type, using synchronous event listeners. However, this function is
    /// asynchronous and a future is returned.
    ///
    /// It is up to the caller to ensure the `target` supports this event type.
    fn add_listener_with_sync_cb<F, T>(
        &self,
        target: &EventTarget,
        callback: F,
    ) -> Listener<T>
    where
        F: FnMut(Self::Data) -> T + 'static,
        T: 'static,
    {
        add_listener_with_sync_cb_raw(target, self.name(), callback)
    }

    /// Adds event listeners to JS's event targets, where events are of this
    /// event type, using asynchronous event listeners. This function is
    /// asynchronous and a future is returned.
    ///
    /// It is up to the caller to ensure the `target` supports this event type.
    fn add_listener_with_async_cb<F, A>(
        &self,
        target: &EventTarget,
        callback: F,
    ) -> Listener<A::Output>
    where
        F: FnMut(Self::Data) -> A + 'static,
        A: Future + 'static,
    {
        add_listener_with_async_cb_raw(target, self.name(), callback)
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
event_type!(WindowResize, "resize", web_sys::UiEvent);
