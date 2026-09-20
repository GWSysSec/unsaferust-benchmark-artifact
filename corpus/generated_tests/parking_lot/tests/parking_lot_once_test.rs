use parking_lot::Once;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn call_once_initializes_exactly_once_across_competing_threads() {
    let once = Arc::new(Once::new());
    let initializer_runs = Arc::new(AtomicUsize::new(0));
    let threads_that_returned = Arc::new(AtomicUsize::new(0));
    let start = Arc::new(Barrier::new(9));

    let initial_state = once.state();
    assert!(!initial_state.done());
    assert!(!initial_state.poisoned());

    let mut handles = Vec::new();
    for _ in 0..8 {
        let once = Arc::clone(&once);
        let initializer_runs = Arc::clone(&initializer_runs);
        let threads_that_returned = Arc::clone(&threads_that_returned);
        let start = Arc::clone(&start);

        handles.push(thread::spawn(move || {
            start.wait();
            once.call_once(|| {
                let previous = initializer_runs.fetch_add(1, Ordering::SeqCst);
                assert_eq!(previous, 0);
            });
            threads_that_returned.fetch_add(1, Ordering::SeqCst);
        }));
    }

    start.wait();

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    assert_eq!(initializer_runs.load(Ordering::SeqCst), 1);
    assert_eq!(threads_that_returned.load(Ordering::SeqCst), 8);

    let completed_state = once.state();
    assert!(completed_state.done());
    assert!(!completed_state.poisoned());

    once.call_once(|| {
        initializer_runs.fetch_add(1, Ordering::SeqCst);
        panic!("call_once closure must not run after successful initialization");
    });

    assert_eq!(initializer_runs.load(Ordering::SeqCst), 1);
    assert!(once.state().done());
}

#[test]
fn call_once_force_observes_poison_and_recovers_once() {
    let once = Once::new();
    let attempts = AtomicUsize::new(0);
    let ordinary_retry_ran = AtomicBool::new(false);
    let force_saw_poison = AtomicBool::new(false);
    let force_saw_not_done = AtomicBool::new(false);

    let first = catch_unwind(AssertUnwindSafe(|| {
        once.call_once(|| {
            attempts.fetch_add(1, Ordering::SeqCst);
            panic!("intentional initialization failure");
        });
    }));

    assert!(first.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);

    let poisoned = once.state();
    assert!(!poisoned.done());
    assert!(poisoned.poisoned());

    let ordinary_retry = catch_unwind(AssertUnwindSafe(|| {
        once.call_once(|| {
            ordinary_retry_ran.store(true, Ordering::SeqCst);
            attempts.fetch_add(1, Ordering::SeqCst);
        });
    }));

    assert!(ordinary_retry.is_err());
    assert!(!ordinary_retry_ran.load(Ordering::SeqCst));
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert!(once.state().poisoned());

    once.call_once_force(|state| {
        attempts.fetch_add(1, Ordering::SeqCst);
        force_saw_poison.store(state.poisoned(), Ordering::SeqCst);
        force_saw_not_done.store(!state.done(), Ordering::SeqCst);
    });

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(force_saw_poison.load(Ordering::SeqCst));
    assert!(force_saw_not_done.load(Ordering::SeqCst));

    let recovered = once.state();
    assert!(recovered.done());
    assert!(!recovered.poisoned());

    once.call_once_force(|_| {
        attempts.fetch_add(1, Ordering::SeqCst);
        panic!("call_once_force closure must not run once the Once is done");
    });

    once.call_once(|| {
        attempts.fetch_add(1, Ordering::SeqCst);
        panic!("call_once closure must not run once the Once is done");
    });

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(once.state().done());
    assert!(!once.state().poisoned());
}