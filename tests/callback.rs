use wasm_bindgen_test::wasm_bindgen_test;
use webio::{callback, task::detach};

#[wasm_bindgen_test]
fn sync_once() {
    detach(async {
        let event = callback::Event::new(|callback: callback::SyncCallback| {
            callback();
        });
        let result = event.listen_once(|| 42).await;
        assert_eq!(result.unwrap(), 42);
    });
}

#[wasm_bindgen_test]
fn sync_once_with_ret() {
    detach(async {
        let event = callback::Event::new(|callback: callback::SyncCallback| {
            callback();
            "my-return-abc"
        });
        let (ret, future) = event.listen_once_returning(|| 42);
        assert_eq!(ret, "my-return-abc");
        let result = future.await;
        assert_eq!(result.unwrap(), 42);
    });
}

#[wasm_bindgen_test]
fn async_once() {
    detach(async {
        let event = callback::Event::new(
            |callback: callback::AsyncCallback<'static>| {
                detach(callback);
            },
        );
        let result = event.listen_once_async(async { 42 }).await;
        assert_eq!(result.unwrap(), 42);
    });
}

#[wasm_bindgen_test]
fn async_once_with_ret() {
    detach(async {
        let event = callback::Event::new(
            |callback: callback::AsyncCallback<'static>| {
                detach(callback);
                "my-return-abc"
            },
        );
        let (ret, future) = event.listen_once_async_returning(async { 42 });
        assert_eq!(ret, "my-return-abc");
        let result = future.await;
        assert_eq!(result.unwrap(), 42);
    });
}
