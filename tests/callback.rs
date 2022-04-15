use futures::stream::{Stream, StreamExt};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
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

fn streaming_multi_event<S>(limit: u32, mut callback: Pin<Box<S>>)
where
    S: Stream + ?Sized + 'static,
{
    if let Some(new_limit) = limit.checked_sub(1) {
        task::detach(async move {
            callback.next().await;
            task::yield_now().await;
            streaming_multi_event(new_limit, callback);
        });
    }
}

#[derive(Debug)]
struct IncrementStream {
    value: u32,
    curr_done: bool,
}

impl Default for IncrementStream {
    fn default() -> Self {
        Self { value: 0, curr_done: false }
    }
}

impl Stream for IncrementStream {
    type Item = u32;

    fn poll_next(
        self: Pin<&mut Self>,
        _ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.curr_done {
            let output = this.value;
            this.curr_done = false;
            this.value = output.wrapping_add(1);
            Poll::Ready(Some(output))
        } else {
            this.curr_done = true;
            Poll::Pending
        }
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

#[webio::test]
async fn streaming_multi() {
    let register = callback::multi::StreamingRegister::new(|callback| {
        streaming_multi_event(3, callback);
    });

    let listener = register.listen(IncrementStream::default());

    assert_eq!(listener.next().await.unwrap(), 0);
    assert_eq!(listener.next().await.unwrap(), 1);
    assert_eq!(listener.next().await.unwrap(), 2);
}

#[webio::test]
async fn streaming_multi_with_ret() {
    let register = callback::multi::StreamingRegister::new(|callback| {
        streaming_multi_event(3, callback);
        "my-return-abc"
    });

    let (ret, listener) = register.listen_returning(IncrementStream::default());

    assert_eq!(ret, "my-return-abc");
    assert_eq!(listener.next().await.unwrap(), 0);
    assert_eq!(listener.next().await.unwrap(), 1);
    assert_eq!(listener.next().await.unwrap(), 2);
}
