//! This module defines macros.

/// Logs to the JavaScript/browser/node console using a given method. Syntax:
/// ```ignore
/// console_log!(argument0, argument1, argument2, ..., argument_n)
/// ```
/// Each argument is converted into a `JsValue` using `Into`.
///
/// # Examples
///
/// ```no_run
/// use webio::console_log;
/// # fn main() {
/// console_log!("Hello number", 5u8, "you're welcome!");
/// # }
/// ```
#[macro_export]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-tags", doc(cfg(feature = "macros")))]
macro_rules! console_log {
    ($($arguments:tt)*) => {
        $crate::console!(log; $($arguments)*)
    };
}

/// Debugs to the JavaScript/browser/node console using a given method. Syntax:
/// ```ignore
/// console_debug!(argument0, argument1, argument2, ..., argument_n)
/// ```
/// Each argument is converted into a `JsValue` using `Into`.
///
/// # Examples
///
/// ```no_run
/// use webio::console_debug;
/// # fn main() {
/// console_debug!("Hello number", 5u8, "you're welcome!");
/// # }
/// ```
#[macro_export]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-tags", doc(cfg(feature = "macros")))]
macro_rules! console_debug {
    ($($arguments:tt)*) => {
        $crate::console!(debug; $($arguments)*)
    };
}

/// Shows info in the JavaScript/browser/node console using a given method.
///
/// Syntax:
/// ```ignore
/// console_info!(argument0, argument1, argument2, ..., argument_n)
/// ```
/// Each argument is converted into a `JsValue` using `Into`.
///
/// # Examples
///
/// ```no_run
/// use webio::console_info;
/// # fn main() {
/// console_info!("Hello number", 5u8, "you're welcome!");
/// # }
/// ```
#[macro_export]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-tags", doc(cfg(feature = "macros")))]
macro_rules! console_info {
    ($($arguments:tt)*) => {
        $crate::console!(info; $($arguments)*)
    };
}

/// Warns to the JavaScript/browser/node console using a given method. Syntax:
/// ```ignore
/// console_warn!(argument0, argument1, argument2, ..., argument_n)
/// ```
/// Each argument is converted into a `JsValue` using `Into`.
///
/// # Examples
///
/// ```no_run
/// use webio::console_warn;
/// # fn main() {
/// console_warn!("Something bad might happen");
/// # }
/// ```
#[macro_export]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-tags", doc(cfg(feature = "macros")))]
macro_rules! console_warn {
    ($($arguments:tt)*) => {
        $crate::console!(warn; $($arguments)*)
    };
}

/// Shows error in the JavaScript/browser/node console using a given method.
///
/// Syntax:
/// ```ignore
/// console_error!(argument0, argument1, argument2, ..., argument_n)
/// ```
/// Each argument is converted into a `JsValue` using `Into`.
///
/// # Examples
///
/// ```no_run
/// use webio::console_error;
/// # fn main() {
/// console_error!("Very bad things happened");
/// # }
/// ```
#[macro_export]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-tags", doc(cfg(feature = "macros")))]
macro_rules! console_error {
    ($($arguments:tt)*) => {
        $crate::console!(error; $($arguments)*)
    };
}

/// Flags a test file as running in the browser instead of node.
///
/// Syntax:
/// ```
/// run_tests_in_browser! {}
/// ```
///
/// # Example
/// ```no_run
/// webio::run_tests_in_browser! {}
///
/// // Runs in the browser
/// #[webio::test]
/// fn my_test() {
///     assert!(true);
/// }
///
/// // Also runs in the browser
/// #[webio::test]
/// fn another_test() {
///     assert!(true & true);
/// }
/// ```
#[macro_export]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-tags", doc(cfg(feature = "macros")))]
macro_rules! run_tests_in_browser {
    {} => {
        $crate::wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    };
}
