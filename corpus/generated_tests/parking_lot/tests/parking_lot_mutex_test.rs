use parking_lot::{const_mutex, Mutex};
use std::sync::{Arc, Barrier};
use std::thread;

#[derive(Debug, PartialEq, Eq)]
struct SharedState {
    counter: usize,
    events: Vec<&'static str>,
}

#[test]
fn module_const_mutex_initializes_and_protects_shared_state() {
    const INITIAL: Mutex<SharedState> = const_mutex(SharedState {
        counter: 0,
        events: Vec::new(),
    });

    let shared = Arc::new(INITIAL);

    {
        let mut guard = shared.lock();
        assert_eq!(guard.counter, 0);
        assert!(guard.events.is_empty());

        guard.counter += 1;
        guard.events.push("main-started");

        assert_eq!(guard.counter, 1);
        assert_eq!(guard.events.as_slice(), &["main-started"]);
    }

    {
        let _held = shared.lock();
        assert!(
            shared.try_lock().is_none(),
            "try_lock should fail while another guard is held"
        );
    }

    {
        let mut guard = shared
            .try_lock()
            .expect("try_lock should succeed after the previous guard is dropped");
        guard.counter += 1;
        guard.events.push("main-relocked");

        assert_eq!(guard.counter, 2);
        assert_eq!(guard.events.len(), 2);
    }

    let barrier = Arc::new(Barrier::new(5));
    let mut handles = Vec::new();

    for event in ["worker-a", "worker-b", "worker-c", "worker-d"] {
        let shared = Arc::clone(&shared);
        let barrier = Arc::clone(&barrier);

        handles.push(thread::spawn(move || {
            barrier.wait();

            let mut guard = shared.lock();
            let previous = guard.counter;
            guard.counter = previous + 1;
            guard.events.push(event);
        }));
    }

    barrier.wait();

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    let mutex = match Arc::try_unwrap(shared) {
        Ok(mutex) => mutex,
        Err(_) => panic!("all Arc clones should be dropped after workers join"),
    };

    let mut final_state = mutex.into_inner();
    final_state.events.sort_unstable();

    assert_eq!(final_state.counter, 6);
    assert_eq!(final_state.events.len(), 6);
    assert_eq!(
        final_state.events,
        vec![
            "main-relocked",
            "main-started",
            "worker-a",
            "worker-b",
            "worker-c",
            "worker-d"
        ]
    );
}

#[test]
fn module_const_mutex_supports_mutating_inner_value_before_sharing() {
    const INITIAL: Mutex<Vec<i32>> = const_mutex(Vec::new());

    let mut mutex = INITIAL;

    {
        let inner = mutex.get_mut();
        inner.extend([30, 10, 20]);
        assert_eq!(inner.as_slice(), &[30, 10, 20]);
    }

    {
        let mut guard = mutex.lock();
        guard.sort_unstable();
        guard.dedup();
        guard.push(40);

        assert_eq!(guard.as_slice(), &[10, 20, 30, 40]);
    }

    let shared = Arc::new(mutex);
    let mut handles = Vec::new();

    for value in [50, 60, 70] {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let mut guard = shared.lock();
            guard.push(value);
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    let mutex = match Arc::try_unwrap(shared) {
        Ok(mutex) => mutex,
        Err(_) => panic!("all Arc clones should be dropped after workers join"),
    };

    let mut values = mutex.into_inner();
    values.sort_unstable();

    assert_eq!(values, vec![10, 20, 30, 40, 50, 60, 70]);
}