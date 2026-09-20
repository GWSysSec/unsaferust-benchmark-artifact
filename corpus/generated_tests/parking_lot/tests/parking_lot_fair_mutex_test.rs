use parking_lot::{const_fair_mutex, FairMutex};
use std::sync::Arc;
use std::thread;

#[test]
fn const_fair_mutex_supports_local_workflow_and_mutation() {
    const INITIAL: FairMutex<Vec<i32>> = const_fair_mutex(Vec::new());

    let mut mutex = INITIAL;

    {
        let values = mutex.get_mut();
        values.extend([3, 1, 2]);
        values.sort();
    }

    {
        let mut guard = mutex.lock();
        assert_eq!(&*guard, &[1, 2, 3]);
        guard.push(4);
        assert_eq!(guard.len(), 4);
    }

    let guard = mutex.try_lock();
    assert!(guard.is_some());
    drop(guard);

    let values = mutex.into_inner();
    assert_eq!(values, vec![1, 2, 3, 4]);
}

#[test]
fn const_fair_mutex_coordinates_multiple_threads() {
    let shared: Arc<FairMutex<Vec<usize>>> = Arc::new(const_fair_mutex(Vec::<usize>::new()));

    let mut handles = Vec::new();
    for thread_id in 0..4 {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            for offset in 0..25 {
                let mut guard = shared.lock();
                guard.push(thread_id * 25 + offset);
            }
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    let mut values = shared.lock().clone();
    assert_eq!(values.len(), 100);

    values.sort_unstable();
    assert_eq!(values.first().copied(), Some(0));
    assert_eq!(values.last().copied(), Some(99));
    assert!(values.iter().copied().eq(0..100));

    let sum: usize = values.iter().sum();
    assert_eq!(sum, (0..100).sum());
}

#[test]
fn const_fair_mutex_try_lock_reflects_lock_state() {
    let mutex = const_fair_mutex(String::from("ready"));

    let guard = mutex.lock();
    assert_eq!(guard.as_str(), "ready");
    assert!(mutex.try_lock().is_none());

    drop(guard);

    let mut second_guard = mutex
        .try_lock()
        .expect("mutex should be unlocked after guard drop");
    second_guard.push_str(" and updated");
    assert_eq!(second_guard.as_str(), "ready and updated");
}