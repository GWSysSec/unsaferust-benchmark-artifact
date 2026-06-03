use async_lock::{Mutex, MutexGuardArc};
use std::sync::{mpsc, Arc};
use std::thread;

#[test]
fn mutex_guard_arc_source_identifies_original_arc_and_preserves_updates() {
    let mutex = Arc::new(Mutex::new(vec!["alpha".to_string(), "beta".to_string()]));

    let mut guard = mutex.lock_arc_blocking();
    guard.push("gamma".to_string());

    let source = MutexGuardArc::source(&guard);
    assert!(Arc::ptr_eq(source, &mutex));
    assert_eq!(source.try_lock_arc().is_none(), true);
    assert_eq!(guard.len(), 3);
    assert_eq!(guard[2], "gamma");

    let cloned_source = Arc::clone(source);
    drop(guard);

    let mut second_guard = cloned_source
        .try_lock_arc()
        .expect("mutex should be available after the first arc guard is dropped");
    assert_eq!(
        &*second_guard,
        &[
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string()
        ]
    );

    second_guard.push("delta".to_string());
    assert_eq!(second_guard.len(), 4);
    drop(second_guard);

    let final_guard = mutex.lock_arc_blocking();
    assert_eq!(final_guard.last().map(String::as_str), Some("delta"));
    assert_eq!(final_guard.len(), 4);
}

#[test]
fn source_from_arc_guard_supports_cross_thread_locking_workflow() {
    let mutex = Arc::new(Mutex::new(0_i32));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();

    let worker_mutex = Arc::clone(&mutex);
    let worker = thread::spawn(move || {
        let mut guard = worker_mutex.lock_arc_blocking();
        *guard += 10;

        let source = MutexGuardArc::source(&guard);
        assert!(Arc::ptr_eq(source, &worker_mutex));
        assert!(source.try_lock_arc().is_none());

        started_tx
            .send(*guard)
            .expect("main thread should receive initial guarded value");
        release_rx
            .recv()
            .expect("main thread should signal guard release");

        *guard += 5;
        done_tx
            .send(*guard)
            .expect("main thread should receive final guarded value");
    });

    assert_eq!(started_rx.recv().unwrap(), 10);
    assert!(mutex.try_lock_arc().is_none());

    release_tx.send(()).unwrap();
    assert_eq!(done_rx.recv().unwrap(), 15);

    worker.join().expect("worker thread should finish cleanly");

    let mut final_guard = mutex
        .try_lock_arc()
        .expect("mutex should be unlocked after worker guard is dropped");
    assert_eq!(*final_guard, 15);

    let source = MutexGuardArc::source(&final_guard);
    assert!(Arc::ptr_eq(source, &mutex));
    *final_guard *= 2;
    assert_eq!(*final_guard, 30);

    drop(final_guard);
    assert_eq!(*mutex.lock_arc_blocking(), 30);
}