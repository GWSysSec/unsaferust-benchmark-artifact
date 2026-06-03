use parking_lot::{const_reentrant_mutex, ReentrantMutex};
use std::cell::RefCell;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn remutex_const_reentrant_mutex_allows_same_thread_reentry_and_excludes_other_threads() {
    const INITIAL: ReentrantMutex<RefCell<Vec<i32>>> =
        const_reentrant_mutex(RefCell::new(Vec::new()));

    let shared = Arc::new(INITIAL);

    let outer_guard = shared.lock();
    outer_guard.borrow_mut().push(10);
    assert_eq!(outer_guard.borrow().as_slice(), &[10]);

    {
        let inner_guard = shared.lock();
        assert_eq!(inner_guard.borrow().len(), 1);
        inner_guard.borrow_mut().extend([20, 30]);
        assert_eq!(inner_guard.borrow().as_slice(), &[10, 20, 30]);

        let try_guard = shared
            .try_lock()
            .expect("same thread should be able to re-enter with try_lock");
        try_guard.borrow_mut().push(40);
        assert_eq!(try_guard.borrow().as_slice(), &[10, 20, 30, 40]);
    }

    assert_eq!(outer_guard.borrow().as_slice(), &[10, 20, 30, 40]);

    let (started_tx, started_rx) = mpsc::channel();
    let (acquired_tx, acquired_rx) = mpsc::channel();

    let worker_shared = Arc::clone(&shared);
    let worker = thread::spawn(move || {
        started_tx.send(()).expect("start notification should send");

        let guard = worker_shared.lock();
        guard.borrow_mut().push(50);
        let snapshot = guard.borrow().clone();

        acquired_tx
            .send(snapshot)
            .expect("acquired notification should send");
    });

    started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("worker should start promptly");

    assert!(
        acquired_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "worker must not acquire the reentrant mutex while it is held by another thread"
    );

    drop(outer_guard);

    let worker_snapshot = acquired_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("worker should acquire after the original guard is dropped");
    assert_eq!(worker_snapshot, vec![10, 20, 30, 40, 50]);

    worker.join().expect("worker thread should not panic");

    let final_guard = shared
        .try_lock()
        .expect("mutex should be unlocked after worker exits");
    assert_eq!(final_guard.borrow().as_slice(), &[10, 20, 30, 40, 50]);
    assert_eq!(final_guard.borrow().last().copied(), Some(50));
}