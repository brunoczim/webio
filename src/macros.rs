//! This module defines macros.

/// Logs to the JavaScript/browser/node console using a given method. Syntax:
/// ```ignore
/// console_log!($($arguments),*)
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
macro_rules! console_log {
    ($($arguments:tt)*) => {
        $crate::console!(log; $($arguments)*)
    };
}

/// Debugs to the JavaScript/browser/node console using a given method. Syntax:
/// ```ignore
/// console_debug!($($arguments),*)
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
macro_rules! console_debug {
    ($($arguments:tt)*) => {
        $crate::console!(debug; $($arguments)*)
    };
}

/// Shows info in the JavaScript/browser/node console using a given method.
/// Syntax:
/// ```ignore
/// console_info!($($arguments),*)
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
macro_rules! console_info {
    ($($arguments:tt)*) => {
        $crate::console!(info; $($arguments)*)
    };
}

/// Warns to the JavaScript/browser/node console using a given method. Syntax:
/// ```ignore
/// console_warn!($($arguments),*)
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
macro_rules! console_warn {
    ($($arguments:tt)*) => {
        $crate::console!(warn; $($arguments)*)
    };
}

/// Shows error in the JavaScript/browser/node console using a given method.
/// Syntax:
/// ```ignore
/// console_error!($($arguments),*)
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
macro_rules! console_error {
    ($($arguments:tt)*) => {
        $crate::console!(error; $($arguments)*)
    };
}
