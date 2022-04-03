//! This is an experimental implementation of a web-based async runtime. Web as
//! in "browser".

#![warn(missing_docs)]

#[doc(hidden)]
pub use js_sys;
#[doc(hidden)]
pub use wasm_bindgen;
#[doc(hidden)]
pub use wasm_bindgen_futures;

mod panic;

pub mod task;

//pub mod callback;

pub mod time;

pub use webio_macros::join;
