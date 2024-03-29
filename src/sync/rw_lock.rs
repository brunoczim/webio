use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    collections::{BTreeMap, BTreeSet},
    fmt,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

type Token = usize;

#[derive(Debug, Clone, Default)]
struct Queue {
    write_owner: Option<Token>,
    read_owners: BTreeSet<Token>,
    reads_on_hold: BTreeMap<Token, Waker>,
    writes_on_hold: BTreeMap<Token, Waker>,
}

impl Queue {
    fn new() -> Self {
        Self::default()
    }

    fn new_token(&self) -> Token {
        let max_write_owner = self.write_owner;
        let max_read_owner = self.read_owners.iter().next_back().copied();
        let max_write_on_hold = self.writes_on_hold.keys().next_back().copied();
        let max_read_on_hold = self.reads_on_hold.keys().next_back().copied();
        max_write_owner
            .max(max_read_owner)
            .max(max_write_on_hold)
            .max(max_read_on_hold)
            .map_or(0, |token| token + 1)
    }

    fn acquire_read(&mut self, waker: Waker, token: Token) {
        if self.write_owner.is_some()
            || self
                .writes_on_hold
                .last_key_value()
                .is_some_and(|(max, _)| token > *max)
        {
            self.reads_on_hold.insert(token, waker);
        } else {
            self.read_owners.insert(token);
            waker.wake();
        }
    }

    fn acquire_write(&mut self, waker: Waker, token: Token) {
        if self.write_owner.is_some() || !self.read_owners.is_empty() {
            self.writes_on_hold.insert(token, waker);
        } else {
            self.write_owner = Some(token);
            waker.wake();
        }
    }

    fn try_acquire_read(&mut self) -> Option<Token> {
        let token = self.new_token();
        if self.write_owner.is_some()
            || self
                .writes_on_hold
                .last_key_value()
                .is_some_and(|(max, _)| token > *max)
        {
            None
        } else {
            self.read_owners.insert(token);
            Some(token)
        }
    }

    fn try_acquire_write(&mut self) -> Option<Token> {
        if self.write_owner.is_some() || !self.read_owners.is_empty() {
            None
        } else {
            let token = self.new_token();
            self.write_owner = Some(token);
            Some(token)
        }
    }

    fn release_read(&mut self, token: Token) {
        self.read_owners.remove(&token);

        if self.read_owners.is_empty() {
            if let Some((write_token, write_waker)) =
                self.writes_on_hold.pop_first()
            {
                self.write_owner = Some(write_token);
                write_waker.wake();
            }
        }
    }

    fn release_write(&mut self) {
        self.write_owner = None;

        if let Some((write_token, write_waker)) =
            self.writes_on_hold.pop_first()
        {
            self.forward_reads(Some(write_token));

            if self.read_owners.is_empty() {
                self.write_owner = Some(write_token);
                write_waker.wake();
            } else {
                self.writes_on_hold.insert(write_token, write_waker);
            }
        } else {
            self.forward_reads(None);
        }
    }

    fn cancel_read(&mut self, token: Token) {
        if self.read_owners.contains(&token) {
            self.release_read(token);
        } else {
            self.reads_on_hold.remove(&token);
        }
    }

    fn cancel_write(&mut self, token: Token) {
        if self.write_owner == Some(token) {
            self.release_write();
        } else {
            self.writes_on_hold.remove(&token);
            self.forward_reads(
                self.writes_on_hold.first_key_value().map(|(token, _)| *token),
            );
        }
    }

    fn forward_reads(&mut self, write_token: Option<Token>) {
        while let Some((read_token, read_waker)) =
            self.reads_on_hold.pop_first()
        {
            if write_token.is_some_and(|token| token < read_token) {
                self.reads_on_hold.insert(read_token, read_waker);
                break;
            }
            self.read_owners.insert(read_token);
            read_waker.wake();
        }
    }
}

/// Read-Write lock over critical sections where data can be locked shared or
/// exclusively. When locked shared, only reads are allowed, while when locked
/// exclusively, writes are also allowed.  Behaves much like
/// [`tokio::sync::RwLock`], but designed for WASM (single-thread, thus this
/// struct is Unsync). This lock is fair.
pub struct RwLock<T> {
    data: RefCell<T>,
    queue: Cell<Queue>,
}

impl<T> RwLock<T> {
    fn with_queue<F, A>(&self, visitor: F) -> A
    where
        F: FnOnce(&mut Queue) -> A,
    {
        let mut queue = self.queue.take();
        let output = visitor(&mut queue);
        self.queue.set(queue);
        output
    }

    /// Creates a read-write-lock from initial protected data.
    pub fn new(data: T) -> Self {
        Self { data: RefCell::new(data), queue: Cell::new(Queue::new()) }
    }

    /// Using a mutable reference to the lock, get protected data mutably as
    /// well.
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// Consumes the lock to take back protected data.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    /// Tries to read-lock without blocking. If write-locked, returns `None`,
    /// otherwise, locks and returns a guard. While the guard is not dropped,
    /// the lock remains locked.
    pub fn try_read(&self) -> Option<ReadGuard<T>> {
        self.with_queue(|queue| {
            if let Some(token) = queue.try_acquire_read() {
                Some(self.do_read(token))
            } else {
                None
            }
        })
    }

    /// Read-locks, waiting if write-locked. When the lock is acquired, returns
    /// a guard. While the guard is not dropped, the lock remains locked.
    pub async fn read(&self) -> ReadGuard<T> {
        let subscriber = ReadSubscriber {
            rw_lock: self,
            state: ReadSubscriberState::NotSubscribed,
        };
        let token = subscriber.await;
        self.do_read(token)
    }

    fn do_read(&self, token: Token) -> ReadGuard<T> {
        ReadGuard { rw_lock: self, token, ref_borrow: self.data.borrow() }
    }

    /// Tries to write-lock without blocking. If already write-locked, or if
    /// read-locked, returns `None`, otherwise, locks and returns a guard.
    /// While the guard is not dropped, the lock remains locked.
    pub fn try_write(&self) -> Option<WriteGuard<T>> {
        self.with_queue(|queue| {
            if queue.try_acquire_write().is_some() {
                Some(self.do_write())
            } else {
                None
            }
        })
    }

    /// Write-locks, waiting if already write-locked, or if read-locked. When
    /// the lock is acquired, returns a guard. While the guard is not
    /// dropped, the lock remains locked.
    pub async fn write(&self) -> WriteGuard<T> {
        let subscriber = WriteSubscriber {
            rw_lock: self,
            state: WriteSubscriberState::NotSubscribed,
        };
        subscriber.await;
        self.do_write()
    }

    fn do_write(&self) -> WriteGuard<T> {
        WriteGuard { rw_lock: self, ref_mut: self.data.borrow_mut() }
    }
}

impl<T> Default for RwLock<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> fmt::Debug for RwLock<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        self.with_queue(|queue| {
            fmtr.debug_struct("RwLock")
                .field("data", &self.data)
                .field("queue", &queue)
                .finish()
        })
    }
}

/// A guard of a current read/shared-locking on a [`RwLock`]. Can be
/// derreferenced to get read access to protected data.
#[derive(Debug)]
pub struct ReadGuard<'rw, T> {
    rw_lock: &'rw RwLock<T>,
    token: Token,
    ref_borrow: Ref<'rw, T>,
}

impl<'rw, T> Deref for ReadGuard<'rw, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.ref_borrow
    }
}

impl<'rw, T> Drop for ReadGuard<'rw, T> {
    fn drop(&mut self) {
        self.rw_lock.with_queue(|queue| queue.release_read(self.token));
    }
}

/// A guard of a current write/exclusive-locking on a [`RwLock`]. Can be
/// derreferenced to get both read and write access to protected data.
#[derive(Debug)]
pub struct WriteGuard<'rw, T> {
    rw_lock: &'rw RwLock<T>,
    ref_mut: RefMut<'rw, T>,
}

impl<'rw, T> Deref for WriteGuard<'rw, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.ref_mut
    }
}

impl<'rw, T> DerefMut for WriteGuard<'rw, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.ref_mut
    }
}

impl<'rw, T> Drop for WriteGuard<'rw, T> {
    fn drop(&mut self) {
        self.rw_lock.with_queue(|queue| queue.release_write());
    }
}

#[derive(Debug, Clone, Copy)]
enum ReadSubscriberState {
    NotSubscribed,
    Subscribed(Token),
    Acquired(Token),
}

#[derive(Debug)]
struct ReadSubscriber<'rw, T> {
    rw_lock: &'rw RwLock<T>,
    state: ReadSubscriberState,
}

impl<'rw, T> Future for ReadSubscriber<'rw, T> {
    type Output = Token;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        match self.state {
            ReadSubscriberState::Acquired(token) => Poll::Ready(token),
            ReadSubscriberState::Subscribed(token) => {
                self.rw_lock.with_queue(|queue| {
                    if queue.read_owners.contains(&token) {
                        self.state = ReadSubscriberState::Acquired(token);
                        Poll::Ready(token)
                    } else {
                        Poll::Pending
                    }
                })
            },
            ReadSubscriberState::NotSubscribed => {
                self.rw_lock.with_queue(|queue| {
                    let token = queue.new_token();
                    queue.acquire_read(cx.waker().clone(), token);
                    self.state = ReadSubscriberState::Subscribed(token);
                    Poll::Pending
                })
            },
        }
    }
}

impl<'rw, T> Drop for ReadSubscriber<'rw, T> {
    fn drop(&mut self) {
        if let ReadSubscriberState::Subscribed(token) = self.state {
            self.rw_lock.with_queue(|queue| {
                queue.cancel_read(token);
            })
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum WriteSubscriberState {
    NotSubscribed,
    Subscribed(Token),
    Acquired,
}

#[derive(Debug)]
struct WriteSubscriber<'rw, T> {
    rw_lock: &'rw RwLock<T>,
    state: WriteSubscriberState,
}

impl<'rw, T> Future for WriteSubscriber<'rw, T> {
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        match self.state {
            WriteSubscriberState::Acquired => Poll::Ready(()),
            WriteSubscriberState::Subscribed(token) => {
                self.rw_lock.with_queue(|queue| {
                    if queue.write_owner == Some(token) {
                        self.state = WriteSubscriberState::Acquired;
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                })
            },
            WriteSubscriberState::NotSubscribed => {
                self.rw_lock.with_queue(|queue| {
                    let token = queue.new_token();
                    queue.acquire_write(cx.waker().clone(), token);
                    self.state = WriteSubscriberState::Subscribed(token);
                    Poll::Pending
                })
            },
        }
    }
}

impl<'rw, T> Drop for WriteSubscriber<'rw, T> {
    fn drop(&mut self) {
        if let WriteSubscriberState::Subscribed(token) = self.state {
            self.rw_lock.with_queue(|queue| {
                queue.cancel_write(token);
            })
        }
    }
}
