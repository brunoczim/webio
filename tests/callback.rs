use wasm_bindgen_test::wasm_bindgen_test;
use webio::{callback, task};

#[wasm_bindgen_test]
fn sync_once() {
    task::detach(async {
        let register = callback::once::SyncRegister::new(|callback| {
            callback();
        });
        let result = register.listen(|| 42).await;
        assert_eq!(result.unwrap(), 42);
    });
}

#[wasm_bindgen_test]
fn sync_once_with_ret() {
    task::detach(async {
        let event = callback::once::SyncRegister::new(|callback| {
            callback();
            "my-return-abc"
        });
        let (ret, future) = event.listen_returning(|| 42);
        assert_eq!(ret, "my-return-abc");
        let result = future.await;
        assert_eq!(result.unwrap(), 42);
    });
}

#[wasm_bindgen_test]
fn async_once() {
    task::detach(async {
        let event = callback::once::AsyncRegister::new(|callback| {
            task::detach(callback);
        });
        let result = event.listen(async { 42 }).await;
        assert_eq!(result.unwrap(), 42);
    });
}

#[wasm_bindgen_test]
fn async_once_with_ret() {
    task::detach(async {
        let event = callback::once::AsyncRegister::new(|callback| {
            task::detach(callback);
            "my-return-abc"
        });
        let (ret, future) = event.listen_returning(async { 42 });
        assert_eq!(ret, "my-return-abc");
        let result = future.await;
        assert_eq!(result.unwrap(), 42);
    });
}
