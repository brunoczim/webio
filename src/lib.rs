//! This is an experimental implementation of a web-based async runtime. Web as
//! in "browser".

#![warn(missing_docs)]

#[macro_use]
mod macros;

mod panic;

pub mod task;

pub mod callback;

pub mod time;

pub use webio_macros::{console, join};

#[doc(hidden)]
pub use js_sys;
#[doc(hidden)]
pub use wasm_bindgen;
#[doc(hidden)]
pub use wasm_bindgen_futures;
#[doc(hidden)]
pub use web_sys;
