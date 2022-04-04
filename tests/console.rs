use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;
use webio::{
    console_debug,
    console_error,
    console_info,
    console_log,
    console_warn,
};

#[wasm_bindgen_test]
fn log_no_arguments() {
    console_log!();
}

#[wasm_bindgen_test]
fn log_one_argument_str() {
    console_log!("hello");
}

#[wasm_bindgen_test]
fn log_one_argument_number() {
    console_log!(3u32);
}

#[wasm_bindgen_test]
fn log_two_arguments() {
    console_log!("Hello, world!", -5i8);
}

#[wasm_bindgen_test]
fn log_three_arguments() {
    console_log!(true, 5i32, JsValue::NULL);
}

#[wasm_bindgen_test]
fn log_seven_arguments() {
    console_log!(true, 5i32, JsValue::NULL, "abc", 1.5f64, 2i32, "def");
}

#[wasm_bindgen_test]
fn log_eight_arguments() {
    console_log!(
        true,
        5i32,
        JsValue::NULL,
        "abc",
        1.5f64,
        2i32,
        "def",
        95.05f64
    );
}

#[wasm_bindgen_test]
fn debug_no_arguments() {
    console_debug!();
}

#[wasm_bindgen_test]
fn debug_one_argument_str() {
    console_debug!("hello");
}

#[wasm_bindgen_test]
fn debug_one_argument_number() {
    console_debug!(3u32);
}

#[wasm_bindgen_test]
fn debug_seven_arguments() {
    console_debug!(true, 5i32, JsValue::NULL, "abc", 1.5f64, 2i32, "def");
}

#[wasm_bindgen_test]
fn debug_eight_arguments() {
    console_debug!(
        true,
        5i32,
        JsValue::NULL,
        "abc",
        1.5f64,
        2i32,
        "def",
        95.05f64
    );
}

#[wasm_bindgen_test]
fn info_no_arguments() {
    console_info!();
}

#[wasm_bindgen_test]
fn info_one_argument_str() {
    console_info!("hello");
}

#[wasm_bindgen_test]
fn info_one_argument_number() {
    console_info!(3u32);
}

#[wasm_bindgen_test]
fn info_seven_arguments() {
    console_info!(true, 5i32, JsValue::NULL, "abc", 1.5f64, 2i32, "def");
}

#[wasm_bindgen_test]
fn info_eight_arguments() {
    console_info!(
        true,
        5i32,
        JsValue::NULL,
        "abc",
        1.5f64,
        2i32,
        "def",
        95.05f64
    );
}

#[wasm_bindgen_test]
fn warn_no_arguments() {
    console_warn!();
}

#[wasm_bindgen_test]
fn warn_one_argument_str() {
    console_warn!("hello");
}

#[wasm_bindgen_test]
fn warn_one_argument_number() {
    console_warn!(3u32);
}

#[wasm_bindgen_test]
fn warn_seven_arguments() {
    console_warn!(true, 5i32, JsValue::NULL, "abc", 1.5f64, 2i32, "def");
}

#[wasm_bindgen_test]
fn warn_eight_arguments() {
    console_warn!(
        true,
        5i32,
        JsValue::NULL,
        "abc",
        1.5f64,
        2i32,
        "def",
        95.05f64
    );
}

#[wasm_bindgen_test]
fn error_no_arguments() {
    console_error!();
}

#[wasm_bindgen_test]
fn error_one_argument_str() {
    console_error!("hello");
}

#[wasm_bindgen_test]
fn error_one_argument_number() {
    console_error!(3u32);
}

#[wasm_bindgen_test]
fn error_seven_arguments() {
    console_error!(true, 5i32, JsValue::NULL, "abc", 1.5f64, 2i32, "def");
}

#[wasm_bindgen_test]
fn error_eight_arguments() {
    console_error!(
        true,
        5i32,
        JsValue::NULL,
        "abc",
        1.5f64,
        2i32,
        "def",
        95.05f64
    );
}
