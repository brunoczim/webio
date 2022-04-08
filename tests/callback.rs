use webio::{callback, task};

#[webio::test]
async fn sync_once() {
    let register = callback::once::SyncRegister::new(|callback| {
        callback();
    });
    let result = register.listen(|| 42).await;
    assert_eq!(result.unwrap(), 42);
}

#[webio::test]
async fn sync_once_with_ret() {
    let event = callback::once::SyncRegister::new(|callback| {
        callback();
        "my-return-abc"
    });
    let (ret, future) = event.listen_returning(|| 42);
    assert_eq!(ret, "my-return-abc");
    let result = future.await;
    assert_eq!(result.unwrap(), 42);
}

#[webio::test]
async fn async_once() {
    let event = callback::once::AsyncRegister::new(|callback| {
        task::detach(callback);
    });
    let result = event.listen(async { 42 }).await;
    assert_eq!(result.unwrap(), 42);
}

#[webio::test]
async fn async_once_with_ret() {
    let event = callback::once::AsyncRegister::new(|callback| {
        task::detach(callback);
        "my-return-abc"
    });
    let (ret, future) = event.listen_returning(async { 42 });
    assert_eq!(ret, "my-return-abc");
    let result = future.await;
    assert_eq!(result.unwrap(), 42);
}
