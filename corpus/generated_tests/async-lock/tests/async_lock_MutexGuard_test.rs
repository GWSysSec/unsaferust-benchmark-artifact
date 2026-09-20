use async_lock::{Mutex, MutexGuard};
use std::ptr;
use std::sync::{mpsc, Arc};
use std::thread;

#[test]
fn mutex_guard_source_identifies_original_mutex_and_preserves_state() {
    let mutex = Mutex::new(vec![1, 2, 3]);

    let mut guard = mutex.lock_blocking();
    guard.push(4);

    let source = MutexGuard::source(&guard);
    assert!(ptr::eq(source, &mutex));
    assert_eq!(&*guard, &[1, 2, 3, 4]);
    assert!(source.try_lock().is_none());

    drop(guard);

    let mut second_guard = source
        .try_lock()
        .expect("mutex should unlock after guard is dropped");
    assert_eq!(&*second_guard, &[1, 2, 3, 4]);
    second_guard.push(5);
    assert_eq!(&*second_guard, &[1, 2, 3, 4, 5]);

    drop(second_guard);

    let final_guard = mutex.lock_blocking();
    assert_eq!(&*final_guard, &[1, 2, 3, 4, 5]);
}

#[test]
fn source_reference_can_drive_follow_up_locking_workflow() {
    let mutex = Mutex::new(10_i32);

    let mut guard = mutex.lock_blocking();
    *guard += 7;

    let source = MutexGuard::source(&guard);
    assert!(ptr::eq(source, &mutex));
    assert_eq!(*guard, 17);
    assert!(source.try_lock().is_none());

    drop(guard);

    {
        let mut follow_up_guard = source
            .try_lock()
            .expect("source should refer to the same now-unlocked mutex");
        *follow_up_guard *= 2;
        assert_eq!(*follow_up_guard, 34);
    }

    assert_eq!(*mutex.lock_blocking(), 34);
}

#[test]
fn source_works_for_guards_created_on_another_thread() {
    let mutex = Arc::new(Mutex::new(String::from("start")));
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();

    let worker_mutex = Arc::clone(&mutex);
    let worker = thread::spawn(move || {
        let mut guard = worker_mutex.lock_blocking();
        guard.push_str("-worker");

        let source = MutexGuard::source(&guard);
        assert!(ptr::eq(source, &*worker_mutex));
        assert!(source.try_lock().is_none());
        assert_eq!(guard.as_str(), "start-worker");

        ready_tx.send(()).expect("main thread should receive readiness");
        release_rx.recv().expect("main thread should release worker");

        guard.push_str("-done");
    });

    ready_rx.recv().expect("worker should acquire mutex first");
    assert!(mutex.try_lock().is_none());

    release_tx.send(()).expect("worker should be releasable");
    worker.join().expect("worker thread should not panic");

    let guard = mutex
        .try_lock()
        .expect("mutex should be available after worker thread exits");
    assert_eq!(guard.as_str(), "start-worker-done");
    assert!(ptr::eq(MutexGuard::source(&guard), &*mutex));
}