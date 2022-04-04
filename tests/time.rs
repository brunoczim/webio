use std::time::Duration;
use wasm_bindgen_test::wasm_bindgen_test;
use webio::{
    join,
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
        assert!(passed >= time);
        assert!(passed < time + Duration::from_millis(50));
    });
}

#[wasm_bindgen_test]
fn timeout_spawn() {
    task::detach(async {
        let first_handle = task::spawn(async {
            timeout(Duration::from_millis(50)).await;
            3
        });
        let second_handle = task::spawn(async {
            timeout(Duration::from_millis(60)).await;
            5
        });
        let third_handle = task::spawn(async {
            timeout(Duration::from_millis(40)).await;
            7
        });
        let (first, second, third) =
            join!(first_handle, second_handle, third_handle);
        assert_eq!(
            (first.unwrap(), second.unwrap(), third.unwrap()),
            (3, 5, 7)
        );
    });
}
