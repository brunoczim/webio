use std::{
    cell::{Cell, RefCell, RefMut},
    collections::BTreeMap,
    fmt,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::Future;

type Card = usize;

#[derive(Debug, Clone, Default)]
struct MutexQueue {
    owner: Option<Card>,
    on_hold: BTreeMap<Card, Waker>,
}

impl MutexQueue {
    fn new() -> Self {
        Self::default()
    }

    fn new_card(&self) -> Card {
        self.on_hold
            .first_key_value()
            .map(|(card, _)| *card)
            .max(self.owner)
            .map_or(0, |card| card + 1)
    }

    fn acquire(&mut self, waker: Waker, card: Card) {
        if self.owner.is_some() {
            self.on_hold.insert(card, waker);
        } else {
            self.owner = Some(card);
            waker.wake();
        }
    }

    fn try_acquire(&mut self) -> Option<Card> {
        if self.owner.is_some() {
            None
        } else {
            let card = self.new_card();
            self.owner = Some(card);
            Some(card)
        }
    }

    fn release(&mut self) {
        self.owner = None;
        if let Some((card, waker)) = self.on_hold.pop_first() {
            self.owner = Some(card);
            waker.wake();
        }
    }
}

pub struct Mutex<T> {
    data: RefCell<T>,
    queue: Cell<MutexQueue>,
}

impl<T> Mutex<T> {
    fn with_queue<F, A>(&self, visitor: F) -> A
    where
        F: FnOnce(&mut MutexQueue) -> A,
    {
        let mut queue = self.queue.take();
        let output = visitor(&mut queue);
        self.queue.set(queue);
        output
    }

    pub fn new(data: T) -> Self {
        Self { data: RefCell::new(data), queue: Cell::new(MutexQueue::new()) }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.with_queue(|queue| {
            if queue.try_acquire().is_some() {
                Some(self.do_lock())
            } else {
                None
            }
        })
    }

    pub async fn lock(&self) -> MutexGuard<T> {
        let subscriber = MutexSubscriber {
            mutex: self,
            state: MutexSubscriberState::NotSubscribed,
        };
        subscriber.await;
        self.do_lock()
    }

    fn do_lock(&self) -> MutexGuard<T> {
        MutexGuard { mutex: self, ref_mut: self.data.borrow_mut() }
    }
}

impl<T> fmt::Debug for Mutex<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        self.with_queue(|queue| {
            fmtr.debug_struct("Mutex")
                .field("data", &self.data)
                .field("queue", &queue)
                .finish()
        })
    }
}

#[derive(Debug)]
pub struct MutexGuard<'mutex, T> {
    mutex: &'mutex Mutex<T>,
    ref_mut: RefMut<'mutex, T>,
}

impl<'mutex, T> Deref for MutexGuard<'mutex, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.ref_mut
    }
}

impl<'mutex, T> DerefMut for MutexGuard<'mutex, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.ref_mut
    }
}

impl<'mutex, T> Drop for MutexGuard<'mutex, T> {
    fn drop(&mut self) {
        self.mutex.with_queue(|queue| queue.release());
    }
}

#[derive(Debug, Clone, Copy)]
enum MutexSubscriberState {
    NotSubscribed,
    Subscribed(Card),
    Acquired,
}

#[derive(Debug, Clone, Copy)]
struct MutexSubscriber<'mutex, T> {
    mutex: &'mutex Mutex<T>,
    state: MutexSubscriberState,
}

impl<'mutex, T> Future for MutexSubscriber<'mutex, T> {
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        match self.state {
            MutexSubscriberState::Acquired => Poll::Ready(()),
            MutexSubscriberState::Subscribed(card) => {
                self.mutex.with_queue(|queue| {
                    if queue.owner == Some(card) {
                        self.state = MutexSubscriberState::Acquired;
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                })
            },
            MutexSubscriberState::NotSubscribed => {
                self.mutex.with_queue(|queue| {
                    let card = queue.new_card();
                    queue.acquire(cx.waker().clone(), card);
                    self.state = MutexSubscriberState::Subscribed(card);
                    Poll::Pending
                })
            },
        }
    }
}
