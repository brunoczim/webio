//! This is an experimental implementation of a web-based async runtime. Web as
//! in "browser".

#![warn(missing_docs)]
#![cfg_attr(feature = "feature-doc-cfg", feature(doc_cfg))]

#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
extern crate self as webio;

#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use webio_macros::{console, join, main, select, test, try_join};

#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use js_sys;
#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use wasm_bindgen;
#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use wasm_bindgen_futures;
#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use wasm_bindgen_test;
#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use web_sys;

#[macro_use]
mod macros;

#[doc(hidden)]
pub mod panic;

pub mod task;

pub mod callback;

#[cfg(feature = "time")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "time")))]
pub mod time;

#[cfg(feature = "event")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "event")))]
pub mod event;
