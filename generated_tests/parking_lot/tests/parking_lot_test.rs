use parking_lot::{const_fair_mutex, const_mutex, const_reentrant_mutex, const_rwlock};
use parking_lot::{FairMutex, Mutex, ReentrantMutex, RwLock};
use std::cell::RefCell;
use std::sync::Arc;
use std::thread;

#[test]
fn const_mutex_supports_mutation_try_lock_and_threaded_updates() {
    const INITIAL: Mutex<Vec<i32>> = const_mutex(Vec::new());

    let shared = Arc::new(INITIAL);

    {
        let mut guard = shared.lock();
        guard.extend([10, 20]);
        assert_eq!(guard.as_slice(), &[10, 20]);
    }

    let mut handles = Vec::new();
    for value in [1, 2, 3, 4] {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let mut guard = shared.lock();
            guard.push(value);
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    {
        let mut guard = shared.try_lock().expect("mutex should be unlocked");
        guard.sort();
        assert_eq!(guard.len(), 6);
        assert_eq!(guard.as_slice(), &[1, 2, 3, 4, 10, 20]);
    }

    let mutex = Arc::try_unwrap(shared).expect("all Arc clones should be dropped");
    assert_eq!(mutex.into_inner(), vec![1, 2, 3, 4, 10, 20]);
}

#[test]
fn const_fair_mutex_allows_local_and_shared_workflows() {
    const INITIAL: FairMutex<Vec<String>> = const_fair_mutex(Vec::new());

    let mut local = INITIAL;

    {
        let values = local.get_mut();
        values.push("created".to_string());
        values.push("before-lock".to_string());
        assert_eq!(values.len(), 2);
    }

    {
        let mut guard = local.lock();
        guard.push("during-lock".to_string());
        assert!(guard.iter().any(|entry| entry == "created"));
    }

    assert_eq!(
        local.into_inner(),
        vec![
            "created".to_string(),
            "before-lock".to_string(),
            "during-lock".to_string()
        ]
    );

    let shared: Arc<FairMutex<usize>> = Arc::new(const_fair_mutex(0usize));
    let mut handles = Vec::new();

    for _ in 0..8 {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            for _ in 0..50 {
                *shared.lock() += 1;
            }
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    assert_eq!(*shared.lock(), 400);
    assert!(shared.try_lock().is_some());
}

#[test]
fn const_reentrant_mutex_supports_nested_locking_on_same_thread() {
    const INITIAL: ReentrantMutex<RefCell<Vec<&'static str>>> =
        const_reentrant_mutex(RefCell::new(Vec::new()));

    let lock = INITIAL;

    {
        let outer = lock.lock();
        outer.borrow_mut().push("outer");

        {
            let inner = lock.lock();
            inner.borrow_mut().push("inner");
            assert_eq!(inner.borrow().as_slice(), &["outer", "inner"]);
        }

        outer.borrow_mut().push("outer-again");
        assert_eq!(outer.borrow().len(), 3);
    }

    {
        let guard = lock.try_lock().expect("reentrant mutex should be unlocked");
        assert_eq!(
            guard.borrow().as_slice(),
            &["outer", "inner", "outer-again"]
        );
    }
}

#[test]
fn const_rwlock_allows_multiple_readers_and_exclusive_writer() {
    const INITIAL: RwLock<Vec<usize>> = const_rwlock(Vec::new());

    let shared = Arc::new(INITIAL);

    {
        let mut writer = shared.write();
        writer.extend(0..5);
        assert_eq!(writer.as_slice(), &[0, 1, 2, 3, 4]);
    }

    {
        let reader_one = shared.read();
        let reader_two = shared.read();
        assert_eq!(reader_one.iter().sum::<usize>(), 10);
        assert_eq!(reader_two.len(), 5);
    }

    let mut handles = Vec::new();
    for multiplier in [2usize, 3, 4] {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let snapshot = shared.read();
            snapshot.iter().map(|value| value * multiplier).sum::<usize>()
        }));
    }

    let mut sums = Vec::new();
    for handle in handles {
        sums.push(handle.join().expect("reader thread should not panic"));
    }
    sums.sort();

    assert_eq!(sums, vec![20, 30, 40]);

    {
        let mut writer = shared.write();
        writer.retain(|value| value % 2 == 0);
        writer.push(10);
    }

    let final_values = shared.read();
    assert_eq!(final_values.as_slice(), &[0, 2, 4, 10]);
    assert!(shared.try_read().is_some());
}