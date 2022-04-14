use futures::stream::Stream;
use std::{any::Any, future::Future, panic, pin::Pin, task};

pub type Payload = Box<dyn Any + Send + 'static>;

pub struct FutureCatchUnwind<A>
where
    A: Future,
{
    inner: A,
}

impl<A> FutureCatchUnwind<A>
where
    A: Future,
{
    pub fn new(inner: A) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> A {
        self.inner
    }
}

impl<A> Future for FutureCatchUnwind<A>
where
    A: Future,
{
    type Output = Result<A::Output, Payload>;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(move || {
            let inner =
                unsafe { self.map_unchecked_mut(|this| &mut this.inner) };
            inner.poll(ctx)
        }));

        match result {
            Ok(task::Poll::Pending) => task::Poll::Pending,
            Ok(task::Poll::Ready(data)) => task::Poll::Ready(Ok(data)),
            Err(error) => task::Poll::Ready(Err(error)),
        }
    }
}

pub struct StreamCatchUnwind<S>
where
    S: Stream,
{
    inner: S,
}

impl<S> StreamCatchUnwind<S>
where
    S: Stream,
{
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> Stream for StreamCatchUnwind<S>
where
    S: Stream,
{
    type Item = Result<S::Item, Payload>;

    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(move || {
            let inner =
                unsafe { self.map_unchecked_mut(|this| &mut this.inner) };
            inner.poll_next(ctx)
        }));

        match result {
            Ok(task::Poll::Pending) => task::Poll::Pending,
            Ok(task::Poll::Ready(None)) => task::Poll::Ready(None),
            Ok(task::Poll::Ready(Some(data))) => {
                task::Poll::Ready(Some(Ok(data)))
            },
            Err(error) => task::Poll::Ready(Some(Err(error))),
        }
    }
}
