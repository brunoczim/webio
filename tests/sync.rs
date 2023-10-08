use std::rc::Rc;

use webio::{
    sync::{Mutex, RwLock},
    task,
};

#[webio::test]
async fn mutex_lock_correctness() {
    let mutex = Rc::new(Mutex::new(3));
    let task0 = task::spawn({
        let mutex = mutex.clone();
        async move {
            let mut guard = mutex.lock().await;
            *guard = 5;
            task::yield_now().await;
            assert_eq!(*guard, 5);
        }
    });
    let task1 = task::spawn({
        let mutex = mutex.clone();
        async move {
            let mut guard = mutex.lock().await;
            *guard = 4;
            task::yield_now().await;
            assert_eq!(*guard, 4);
        }
    });
    let task2 = task::spawn({
        let mutex = mutex.clone();
        async move {
            let mut guard = mutex.lock().await;
            *guard = 2;
            task::yield_now().await;
            assert_eq!(*guard, 2);
        }
    });

    webio::try_join!(task0, task1, task2).unwrap();
}

#[webio::test]
async fn mutex_try_lock_correctness() {
    let mutex = Rc::new(Mutex::new(3));
    let task0 = task::spawn({
        let mutex = mutex.clone();
        async move {
            let mut guard = mutex.try_lock().unwrap();
            *guard = 5;
            task::yield_now().await;
            assert_eq!(*guard, 5);
        }
    });
    let task1 = task::spawn({
        let mutex = mutex.clone();
        async move {
            assert!(mutex.try_lock().is_none());
        }
    });
    let task2 = task::spawn({
        let mutex = mutex.clone();
        async move {
            assert!(mutex.try_lock().is_none());
        }
    });

    webio::try_join!(task0, task1, task2).unwrap();
}

#[webio::test]
async fn mutex_lock_fairness() {
    let mutex = Rc::new(Mutex::new(0));
    let task0 = task::spawn({
        let mutex = mutex.clone();
        async move {
            let mut guard = mutex.lock().await;
            assert_eq!(*guard, 0);
            *guard += 1;
        }
    });
    let task1 = task::spawn({
        let mutex = mutex.clone();
        async move {
            let mut guard = mutex.lock().await;
            assert_eq!(*guard, 1);
            *guard += 2;
        }
    });
    let task2 = task::spawn({
        let mutex = mutex.clone();
        async move {
            let mut guard = mutex.lock().await;
            assert_eq!(*guard, 3);
            *guard += 3;
        }
    });

    webio::try_join!(task0, task1, task2).unwrap();
}

#[webio::test]
async fn rwlock_correctness() {
    let rwlock = Rc::new(RwLock::new(3));
    let task0 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let guard = rwlock.read().await;
            assert_eq!(*guard, 3);
            task::yield_now().await;
            assert_eq!(*guard, 3);
        }
    });
    let task1 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let guard = rwlock.read().await;
            assert_eq!(*guard, 3);
            task::yield_now().await;
            assert_eq!(*guard, 3);
        }
    });
    let task2 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let mut guard = rwlock.write().await;
            *guard = 4;
            task::yield_now().await;
            assert_eq!(*guard, 4);
        }
    });
    let task3 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            {
                let guard = rwlock.read().await;
                assert_eq!(*guard, 4);
                task::yield_now().await;
                assert_eq!(*guard, 4);
            }
            for _ in 0 .. 3 {
                task::yield_now().await;
            }
            {
                let guard = rwlock.read().await;
                assert_eq!(*guard, 9);
                task::yield_now().await;
                assert_eq!(*guard, 9);
            }
        }
    });
    let task4 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let mut guard = rwlock.write().await;
            *guard = 9;
            task::yield_now().await;
            assert_eq!(*guard, 9);
        }
    });

    webio::try_join!(task0, task1, task2, task3, task4).unwrap();
}

#[webio::test]
async fn rwlock_try_correctness() {
    let rwlock = Rc::new(RwLock::new(3));
    let task0 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let guard = rwlock.try_read().unwrap();
            assert_eq!(*guard, 3);
            task::yield_now().await;
            assert_eq!(*guard, 3);
        }
    });
    let task1 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            assert!(rwlock.try_write().is_none());
            for _ in 0 .. 3 {
                task::yield_now().await;
            }
            let mut guard = rwlock.try_write().unwrap();
            *guard = 4;
            task::yield_now().await;
            assert_eq!(*guard, 4);
        }
    });
    let task2 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            {
                let guard = rwlock.try_read().unwrap();
                assert_eq!(*guard, 3);
                task::yield_now().await;
                assert_eq!(*guard, 3);
            }
            for _ in 0 .. 4 {
                task::yield_now().await;
            }
            {
                let guard = rwlock.try_read().unwrap();
                let back_then = *guard;
                if back_then != 4 && back_then != 9 {
                    panic!(
                        "failed to verify that {:?} is either 4 or 9",
                        back_then
                    );
                }
                task::yield_now().await;
                assert_eq!(*guard, back_then);
            }
        }
    });
    let task3 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            assert!(rwlock.try_write().is_none());
        }
    });

    webio::try_join!(task0, task1, task2, task3).unwrap();
}

#[webio::test]
async fn rwlock_fairness() {
    let rwlock = Rc::new(RwLock::new(0));
    let task0 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let guard = rwlock.read().await;
            assert_eq!(*guard, 0);
            task::yield_now().await;
            assert_eq!(*guard, 0);
        }
    });
    let task1 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let guard = rwlock.read().await;
            assert_eq!(*guard, 0);
            task::yield_now().await;
            assert_eq!(*guard, 0);
        }
    });
    let task2 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let mut guard = rwlock.write().await;
            assert_eq!(*guard, 0);
            *guard += 1;
            task::yield_now().await;
            assert_eq!(*guard, 1);
        }
    });
    let task3 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            {
                let guard = rwlock.read().await;
                assert_eq!(*guard, 1);
                task::yield_now().await;
                assert_eq!(*guard, 1);
            }
            for _ in 0 .. 3 {
                task::yield_now().await;
            }
            {
                let guard = rwlock.read().await;
                assert_eq!(*guard, 3);
                task::yield_now().await;
                assert_eq!(*guard, 3);
            }
        }
    });
    let task4 = task::spawn({
        let rwlock = rwlock.clone();
        async move {
            let mut guard = rwlock.write().await;
            assert_eq!(*guard, 1);
            *guard += 2;
            task::yield_now().await;
            assert_eq!(*guard, 3);
        }
    });

    webio::try_join!(task0, task1, task2, task3, task4).unwrap();
}
