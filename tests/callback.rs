use std::future::Future;
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
    let register = callback::once::SyncRegister::new(|callback| {
        callback();
        "my-return-abc"
    });
    let (ret, future) = register.listen_returning(|| 42);
    assert_eq!(ret, "my-return-abc");
    let result = future.await;
    assert_eq!(result.unwrap(), 42);
}

#[webio::test]
async fn async_once() {
    let register = callback::once::AsyncRegister::new(|callback| {
        task::detach(callback);
    });
    let result = register.listen(async { 42 }).await;
    assert_eq!(result.unwrap(), 42);
}

#[webio::test]
async fn async_once_with_ret() {
    let register = callback::once::AsyncRegister::new(|callback| {
        task::detach(callback);
        "my-return-abc"
    });
    let (ret, future) = register.listen_returning(async { 42 });
    assert_eq!(ret, "my-return-abc");
    let result = future.await;
    assert_eq!(result.unwrap(), 42);
}

fn sync_multi_event<F>(limit: u32, mut callback: F)
where
    F: FnMut() + 'static,
{
    if let Some(new_limit) = limit.checked_sub(1) {
        task::detach(async move {
            callback();
            task::yield_now().await;
            sync_multi_event(new_limit, callback);
        });
    }
}

fn async_multi_event<A, F>(limit: u32, mut callback: F)
where
    F: FnMut() -> A + 'static,
    A: Future + 'static,
{
    if let Some(new_limit) = limit.checked_sub(1) {
        task::detach(async move {
            callback().await;
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

    let mut count = 0;
    let listener = register.listen(move || {
        let output = count;
        count += 1;
        output
    });

    assert_eq!(listener.next().await.unwrap(), 0);
    assert_eq!(listener.next().await.unwrap(), 1);
    assert_eq!(listener.next().await.unwrap(), 2);
}

#[webio::test]
async fn sync_multi_with_ret() {
    let register = callback::multi::SyncRegister::new(|callback| {
        sync_multi_event(3, callback);
        "my-return-abc"
    });

    let mut count = 0;
    let (ret, listener) = register.listen_returning(move || {
        let output = count;
        count += 1;
        output
    });

    assert_eq!(ret, "my-return-abc");
    assert_eq!(listener.next().await.unwrap(), 0);
    assert_eq!(listener.next().await.unwrap(), 1);
    assert_eq!(listener.next().await.unwrap(), 2);
}

#[webio::test]
async fn async_multi() {
    let register = callback::multi::AsyncRegister::new(|callback| {
        async_multi_event(3, callback);
    });

    let mut count = 0;
    let listener = register.listen(move || {
        let output = count;
        count += 1;
        async move { output }
    });

    assert_eq!(listener.next().await.unwrap(), 0);
    assert_eq!(listener.next().await.unwrap(), 1);
    assert_eq!(listener.next().await.unwrap(), 2);
}

#[webio::test]
async fn async_multi_with_ret() {
    let register = callback::multi::AsyncRegister::new(|callback| {
        async_multi_event(3, callback);
        "my-return-abc"
    });

    let mut count = 0;
    let (ret, listener) = register.listen_returning(move || {
        let output = count;
        count += 1;
        async move { output }
    });

    assert_eq!(ret, "my-return-abc");
    assert_eq!(listener.next().await.unwrap(), 0);
    assert_eq!(listener.next().await.unwrap(), 1);
    assert_eq!(listener.next().await.unwrap(), 2);
}
