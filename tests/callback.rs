use std::future::Future;
use webio::{callback, task};

#[webio::test]
async fn sync_once() {
    let register = callback::once::SyncRegister::new(|callback| {
        callback(40);
    });
    let result = register.listen(|event_data| event_data + 2).await;
    assert_eq!(result.unwrap(), 42);
}

#[webio::test]
async fn sync_once_with_ret() {
    let register = callback::once::SyncRegister::new(|callback| {
        callback(40);
        "my-return-abc"
    });
    let (ret, future) = register.listen_returning(|event_data| event_data + 2);
    assert_eq!(ret, "my-return-abc");
    let result = future.await;
    assert_eq!(result.unwrap(), 42);
}

#[webio::test]
async fn async_once() {
    let register = callback::once::AsyncRegister::new(|callback| {
        task::detach(callback(40));
    });
    let result =
        register.listen(|event_data| async move { event_data + 2 }).await;
    assert_eq!(result.unwrap(), 42);
}

#[webio::test]
async fn async_once_with_ret() {
    let register = callback::once::AsyncRegister::new(|callback| {
        task::detach(callback(40));
        "my-return-abc"
    });
    let (ret, future) =
        register.listen_returning(|event_data| async move { event_data + 2 });
    assert_eq!(ret, "my-return-abc");
    let result = future.await;
    assert_eq!(result.unwrap(), 42);
}

fn sync_multi_event<F>(limit: u32, mut callback: F)
where
    F: FnMut(u32) + 'static,
{
    if let Some(new_limit) = limit.checked_sub(1) {
        task::detach(async move {
            callback(new_limit);
            task::yield_now().await;
            sync_multi_event(new_limit, callback);
        });
    }
}

fn async_multi_event<A, F>(limit: u32, mut callback: F)
where
    F: FnMut(u32) -> A + 'static,
    A: Future + 'static,
{
    if let Some(new_limit) = limit.checked_sub(1) {
        task::detach(async move {
            callback(new_limit).await;
            task::yield_now().await;
            async_multi_event(new_limit, callback);
        });
    }
}

#[webio::test]
async fn sync_multi() {
    let register = callback::multi::SyncRegister::new(|callback| {
        sync_multi_event(3, callback);
    });

    let listener = register.listen(|exp| 2u32.pow(exp));

    assert_eq!(listener.next().await.unwrap(), 4);
    assert_eq!(listener.next().await.unwrap(), 2);
    assert_eq!(listener.next().await.unwrap(), 1);
}

#[webio::test]
async fn sync_multi_with_ret() {
    let register = callback::multi::SyncRegister::new(|callback| {
        sync_multi_event(3, callback);
        "my-return-abc"
    });

    let (ret, listener) = register.listen_returning(|exp| 2u32.pow(exp));

    assert_eq!(ret, "my-return-abc");
    assert_eq!(listener.next().await.unwrap(), 4);
    assert_eq!(listener.next().await.unwrap(), 2);
    assert_eq!(listener.next().await.unwrap(), 1);
}

#[webio::test]
async fn async_multi() {
    let register = callback::multi::AsyncRegister::new(|callback| {
        async_multi_event(3, callback);
    });

    let listener = register.listen(|exp| async move { 2u32.pow(exp) });

    assert_eq!(listener.next().await.unwrap(), 4);
    assert_eq!(listener.next().await.unwrap(), 2);
    assert_eq!(listener.next().await.unwrap(), 1);
}

#[webio::test]
async fn async_multi_with_ret() {
    let register = callback::multi::AsyncRegister::new(|callback| {
        async_multi_event(3, callback);
        "my-return-abc"
    });

    let (ret, listener) =
        register.listen_returning(|exp| async move { 2u32.pow(exp) });

    assert_eq!(ret, "my-return-abc");
    assert_eq!(listener.next().await.unwrap(), 4);
    assert_eq!(listener.next().await.unwrap(), 2);
    assert_eq!(listener.next().await.unwrap(), 1);
}
