use wasm_bindgen_test::wasm_bindgen_test;
use webio::panic;

#[wasm_bindgen_test]
async fn panicked() {
    let _guard = webio::panic::disable_hook_during_recovery();
    let result = panic::catch(async { panic!("error") }).await;
    assert!(result.is_err());
}
