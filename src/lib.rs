//! This is an experimental implementation of a web-based async runtime. Web as
//! in "browser".

#![warn(missing_docs)]

#[doc(hidden)]
pub use js_sys;
#[doc(hidden)]
pub use wasm_bindgen;
#[doc(hidden)]
pub use wasm_bindgen_futures;
#[doc(hidden)]
pub use web_sys;

mod panic;

pub mod task;

//pub mod callback;

pub mod time;

pub use webio_macros::{console, join};

#[macro_export]
macro_rules! console_log {
    ($($arguments:tt)*) => {
        $crate::console!(log; $($arguments)*)
    };
}

#[macro_export]
macro_rules! console_debug {
    ($($arguments:tt)*) => {
        $crate::console!(debug; $($arguments)*)
    };
}

#[macro_export]
macro_rules! console_info {
    ($($arguments:tt)*) => {
        $crate::console!(info; $($arguments)*)
    };
}

#[macro_export]
macro_rules! console_warn {
    ($($arguments:tt)*) => {
        $crate::console!(warn; $($arguments)*)
    };
}

#[macro_export]
macro_rules! console_error {
    ($($arguments:tt)*) => {
        $crate::console!(error; $($arguments)*)
    };
}
