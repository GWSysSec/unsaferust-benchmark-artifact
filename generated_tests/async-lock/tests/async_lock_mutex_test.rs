use async_lock::{Mutex, MutexGuard};
use std::ptr;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

#[test]
fn mutex_guard_source_points_to_original_mutex_and_reflects_guarded_updates() {
    let mutex = Mutex::new(vec!["created".to_string()]);

    let mut guard = mutex.lock_blocking();
    guard.push("updated while locked".to_string());

    let source = MutexGuard::source(&guard);

    assert!(ptr::eq(source, &mutex));
    assert_eq!(guard.len(), 2);
    assert_eq!(guard[0], "created");
    assert_eq!(guard[1], "updated while locked");
    assert!(source.try_lock().is_none());

    drop(guard);

    let mut next_guard = source
        .try_lock()
        .expect("source should refer to the mutex and become lockable after guard drop");
    assert_eq!(
        &*next_guard,
        &[
            "created".to_string(),
            "updated while locked".to_string()
        ]
    );

    next_guard.push("updated through source".to_string());
    assert_eq!(next_guard.len(), 3);

    drop(next_guard);

    let final_guard = mutex.lock_blocking();
    assert_eq!(final_guard.last().map(String::as_str), Some("updated through source"));
    assert_eq!(final_guard.len(), 3);
}

#[test]
fn mutex_guard_source_remains_useful_in_arc_backed_workflow() {
    let mutex = Arc::new(Mutex::new(0_i32));

    let mut guard = mutex.lock_blocking();
    *guard = 41;

    let source = MutexGuard::source(&guard);

    assert!(ptr::eq(source, &*mutex));
    assert_eq!(*guard, 41);
    assert!(source.try_lock().is_none());

    *guard += 1;
    assert_eq!(*guard, 42);

    drop(guard);

    {
        let mut follow_up_guard = source
            .try_lock()
            .expect("mutex should be available after the original guard is dropped");
        assert_eq!(*follow_up_guard, 42);
        *follow_up_guard *= 2;
        assert_eq!(*follow_up_guard, 84);
    }

    let final_guard = mutex.lock_blocking();
    assert_eq!(*final_guard, 84);
}

#[test]
fn source_identifies_locked_mutex_while_other_threads_wait_for_release() {
    let mutex = Arc::new(Mutex::new(Vec::<usize>::new()));
    let mut guard = mutex.lock_blocking();

    guard.push(1);
    guard.push(2);

    let source = MutexGuard::source(&guard);

    assert!(ptr::eq(source, &*mutex));
    assert_eq!(&*guard, &[1, 2]);
    assert!(source.try_lock().is_none());

    let (started_tx, started_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();

    let worker_mutex = Arc::clone(&mutex);
    let worker = thread::spawn(move || {
        assert!(worker_mutex.try_lock().is_none());
        started_tx.send(()).expect("main thread should receive start notification");

        let mut worker_guard = worker_mutex.lock_blocking();
        worker_guard.push(3);
        worker_guard.push(4);

        done_tx
            .send(worker_guard.clone())
            .expect("main thread should receive final vector");
    });

    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("worker should observe the mutex while it is still locked");

    assert!(source.try_lock().is_none());
    assert_eq!(&*guard, &[1, 2]);

    drop(guard);

    let worker_result = done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("worker should complete after the mutex is released");

    worker.join().expect("worker thread should not panic");

    assert_eq!(worker_result, vec![1, 2, 3, 4]);

    let final_guard = mutex.lock_blocking();
    assert_eq!(&*final_guard, &[1, 2, 3, 4]);
}