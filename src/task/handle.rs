use crate::panic::PanicPayload;
use std::{cell::Cell, fmt, future::Future, pin::Pin, rc::Rc, task};

#[derive(Debug)]
enum JoinErrorKind {
    Panicked(PanicPayload),
    Cancelled,
}

impl fmt::Display for JoinErrorKind {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JoinErrorKind::Panicked(payload) => {
                write!(fmtr, "task panicked: {:?}", payload)
            },
            JoinErrorKind::Cancelled => write!(fmtr, "task cancelled"),
        }
    }
}

/// An error that might happen when waiting for a task.
#[derive(Debug)]
pub struct JoinError {
    kind: JoinErrorKind,
}

impl fmt::Display for JoinError {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        write!(fmtr, "{}", self.kind)
    }
}

impl JoinError {
    /// Tests whether the target task was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(&self.kind, JoinErrorKind::Cancelled)
    }

    /// Tests whether the target task panicked.
    pub fn is_panic(&self) -> bool {
        matches!(&self.kind, JoinErrorKind::Panicked(_))
    }

    /// Attempts to convert this error into a panic payload. Fails if the target
    /// task didn't panicked.
    pub fn try_into_panic(self) -> Result<PanicPayload, Self> {
        match self.kind {
            JoinErrorKind::Panicked(payload) => Ok(payload),
            kind => Err(Self { kind }),
        }
    }

    /// Converts this error into a panic payload.
    ///
    /// # Panics
    /// Panics if this error was not caused by panic (e.g. the task was
    /// cancelled).
    pub fn into_panic(self) -> PanicPayload {
        self.try_into_panic().unwrap()
    }
}

pub(super) struct Shared<T> {
    connected: Cell<bool>,
    waker: Cell<Option<task::Waker>>,
    data: Cell<Result<T, JoinErrorKind>>,
}

impl<T> Shared<T> {
    pub(super) fn init_connected() -> Self {
        Self {
            connected: Cell::new(true),
            waker: Cell::new(None),
            data: Cell::new(Err(JoinErrorKind::Cancelled)),
        }
    }
}

pub(super) struct TaskHandle<T> {
    shared: Rc<Shared<T>>,
}

impl<T> TaskHandle<T> {
    pub(super) fn new(shared: Rc<Shared<T>>) -> Self {
        Self { shared }
    }

    pub(super) fn panicked(self, payload: PanicPayload) {
        self.shared.data.set(Err(JoinErrorKind::Panicked(payload)));
    }

    pub(super) fn success(self, data: T) {
        self.shared.data.set(Ok(data));
    }
}

impl<T> Drop for TaskHandle<T> {
    fn drop(&mut self) {
        let was_connected = self.shared.connected.replace(false);
        if was_connected {
            if let Some(waker) = self.shared.waker.take() {
                waker.wake();
            }
        }
    }
}

/// A handle that allows the caller to join a task (i.e. wait for it to end).
pub struct JoinHandle<T> {
    shared: Rc<Shared<T>>,
}

impl<T> JoinHandle<T> {
    pub(super) fn new(shared: Rc<Shared<T>>) -> Self {
        Self { shared }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        if self.shared.connected.get() {
            let mut waker = self.shared.waker.take();
            if waker.is_none() {
                waker = Some(ctx.waker().clone());
            }
            self.shared.waker.set(waker);
            task::Poll::Pending
        } else {
            let result =
                self.shared.data.replace(Err(JoinErrorKind::Cancelled));
            task::Poll::Ready(result.map_err(|kind| JoinError { kind }))
        }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        self.shared.connected.set(false);
    }
}
