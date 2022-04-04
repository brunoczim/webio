use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;
use webio::console_log;

#[wasm_bindgen_test]
fn no_arguments() {
    console_log!();
}

#[wasm_bindgen_test]
fn one_argument_str() {
    console_log!("hello");
}

#[wasm_bindgen_test]
fn one_argument_number() {
    console_log!(3u32);
}

#[wasm_bindgen_test]
fn two_arguments() {
    console_log!("Hello, world!", -5i8);
}

#[wasm_bindgen_test]
fn three_arguments() {
    console_log!(true, 5i32, JsValue::NULL);
}

#[wasm_bindgen_test]
fn seven_arguments() {
    console_log!(true, 5i32, JsValue::NULL, "abc", 1.5f64, 2i32, "def");
}

#[wasm_bindgen_test]
fn eight_arguments() {
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
