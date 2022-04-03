use std::time::Duration;
use wasm_bindgen_test::wasm_bindgen_test;
use webio::{
    task,
    time::{timeout, Instant},
};

#[wasm_bindgen_test]
fn timeout_and_instant() {
    task::detach(async {
        let then = Instant::now();
        let time = Duration::from_millis(100);
        timeout(time).await;
        let passed = then.elapsed();
        assert!(passed >= time && passed < time + Duration::from_millis(20));
    });
}
