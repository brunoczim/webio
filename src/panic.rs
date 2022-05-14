//! Utilities for panic recovering.

use crate::select;
use std::{
    collections::VecDeque,
    error::Error,
    fmt,
    future::Future,
    mem,
    panic,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering::*},
        Arc,
        Mutex,
        Once,
    },
    task,
};

type Hook = Box<dyn Fn(&panic::PanicInfo) + Send + Sync + 'static>;

static INIT_STATE: Once = Once::new();
static mut RECOVERER_STATE: Option<Mutex<RecovererState>> = None;
static HOOK_DURING_RECOVERY_DISABLE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// An instance of a panic. Currently, this holds no data, but there a plans for
/// making it hold panic payload.
#[derive(Debug)]
pub struct Panic {
    _priv: (),
}

impl fmt::Display for Panic {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        write!(fmtr, "task panicked")
    }
}

impl Error for Panic {}

/// Attempts to catch a panic from a given future. Note, however, that this is
/// far from perfect, because if there are concurrent futures and any of them
/// panicks, this function will catch their panic. Alternatively, one can think
/// of `catch` as "catch any panic during the execution of the give future".
#[cfg(feature = "macros")]
#[cfg_attr(
    feature = "feature-doc-cfg",
    doc(cfg(all(feature = "panic", feature = "macros")))
)]
pub async fn catch<A>(future: A) -> Result<A::Output, Panic>
where
    A: Future + 'static,
{
    let recovery = Recovery::start();
    select! {
        panic = recovery => Err(panic),
        data = future => Ok(data),
    }
}

/// A guard of hook disabling. While this struct lives, produced by
/// [`disable_hook_during_recovery`] function, and while there is a recovery in
/// action, the previously used panic hook won't be called. However, whenever
/// this struct is dropped, it enables the previous hook again. Whether or not
/// the hook is enabled, the recovery will happen.
#[derive(Debug)]
pub struct DisableHookGuard {
    _priv: (),
}

impl Drop for DisableHookGuard {
    fn drop(&mut self) {
        HOOK_DURING_RECOVERY_DISABLE_COUNT.fetch_sub(1, Release);
    }
}

#[must_use]
pub fn disable_hook_during_recovery() -> DisableHookGuard {
    HOOK_DURING_RECOVERY_DISABLE_COUNT.fetch_add(1, Release);
    DisableHookGuard { _priv: () }
}

pub fn hook_during_recovery_enabled() -> bool {
    HOOK_DURING_RECOVERY_DISABLE_COUNT.load(Acquire) == 0
}

fn access_recoverer_state() -> &'static Mutex<RecovererState> {
    INIT_STATE.call_once(|| unsafe {
        RECOVERER_STATE = Some(Mutex::new(RecovererState::Inactive));
    });

    unsafe { RECOVERER_STATE.as_ref().unwrap() }
}

fn recoverable_hook(info: &panic::PanicInfo) {
    let mut guard = access_recoverer_state().lock().unwrap();
    let state = guard.assume_active_mut("recoverable_hook");

    if hook_during_recovery_enabled() {
        (state.previous_hook)(info);
    }

    loop {
        match state.recoveries.pop_front() {
            Some(channel) => {
                let mut state = channel.state.lock().unwrap();
                if let MessageState::Requested(waker) = &mut *state {
                    if let Some(waker) = waker.take() {
                        waker.wake();
                    }
                    *state = MessageState::Sent(Panic { _priv: () });
                    break;
                }
            },
            None => break,
        }
    }
}

#[derive(Debug)]
enum RecovererState {
    Inactive,
    Active(ActiveState),
}

impl RecovererState {
    fn assume_active_mut(&mut self, debug_msg: &str) -> &mut ActiveState {
        match self {
            RecovererState::Inactive => unreachable!("{}", debug_msg),
            RecovererState::Active(state) => state,
        }
    }
}

struct ActiveState {
    others_count: usize,
    recoveries: VecDeque<RecoveryChannel>,
    previous_hook: Hook,
}

impl ActiveState {
    fn new(previous_hook: Hook) -> Self {
        Self { others_count: 0, previous_hook, recoveries: VecDeque::new() }
    }
}

impl fmt::Debug for ActiveState {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.debug_struct("ActiveState")
            .field("others_count", &self.others_count)
            .field("recoveries", &self.recoveries)
            .field("previous_hook", &(&*self.previous_hook as *const _))
            .finish()
    }
}

#[derive(Debug)]
#[repr(usize)]
enum MessageState {
    NotRequested,
    Requested(Option<task::Waker>),
    Sent(Panic),
}

#[derive(Debug, Clone)]
struct RecoveryChannel {
    state: Arc<Mutex<MessageState>>,
}

impl RecoveryChannel {
    fn new() -> Self {
        Self { state: Arc::new(Mutex::new(MessageState::NotRequested)) }
    }
}

#[derive(Debug)]
pub struct Recovery {
    channel: RecoveryChannel,
}

impl Recovery {
    pub fn start() -> Self {
        let mut guard = access_recoverer_state().lock().unwrap();
        match &mut *guard {
            RecovererState::Inactive => {
                let previous = panic::take_hook();
                panic::set_hook(Box::new(recoverable_hook));
                *guard = RecovererState::Active(ActiveState::new(previous));
            },
            RecovererState::Active(state) => state.others_count += 1,
        }

        let channel = RecoveryChannel::new();
        guard.assume_active_mut("start").recoveries.push_back(channel.clone());
        Self { channel }
    }
}

impl Future for Recovery {
    type Output = Panic;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let mut state = self.channel.state.lock().unwrap();

        match mem::replace(&mut *state, MessageState::NotRequested) {
            MessageState::NotRequested => {
                *state = MessageState::Requested(Some(ctx.waker().clone()));
                drop(state);
                let channel = self.channel.clone();
                let mut guard = access_recoverer_state().lock().unwrap();
                guard.assume_active_mut("poll").recoveries.push_back(channel);
                task::Poll::Pending
            },

            MessageState::Requested(mut waker) => {
                if waker.is_none() {
                    waker = Some(ctx.waker().clone());
                }
                *state = MessageState::Requested(waker);
                task::Poll::Pending
            },

            MessageState::Sent(panic) => task::Poll::Ready(panic),
        }
    }
}

impl Drop for Recovery {
    fn drop(&mut self) {
        {
            let mut state = self.channel.state.lock().unwrap();
            *state = MessageState::NotRequested;
        }

        let mut guard = access_recoverer_state().lock().unwrap();
        let state = guard.assume_active_mut("drop");
        match state.others_count.checked_sub(1) {
            Some(new_count) => state.others_count = new_count,
            None => {
                let current = panic::take_hook();
                let previous = mem::replace(&mut state.previous_hook, current);
                panic::set_hook(previous);
                *guard = RecovererState::Inactive;
            },
        }
    }
}
