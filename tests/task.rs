use wasm_bindgen_test::wasm_bindgen_test;
use webio::{join, task};

#[wasm_bindgen_test]
fn triple_spawn_join_with_detach() {
    task::detach(async {
        let first_handle = task::spawn(async { 3 });
        let second_handle = task::spawn(async { 5 });
        let third_handle = task::spawn(async { 7 });
        let (first, second, third) =
            join!(first_handle, second_handle, third_handle);
        assert_eq!(
            (first.unwrap(), second.unwrap(), third.unwrap()),
            (3, 5, 7)
        );
    });
}

#[webio::test]
async fn triple_spawn_join_with_test_macro() {
    let first_handle = task::spawn(async { 3 });
    let second_handle = task::spawn(async { 5 });
    let third_handle = task::spawn(async { 7 });
    let (first, second, third) =
        join!(first_handle, second_handle, third_handle);
    assert_eq!((first.unwrap(), second.unwrap(), third.unwrap()), (3, 5, 7));
}

const _ASSERT_TEST_MACRO: fn() -> () = triple_spawn_join_with_test_macro;
