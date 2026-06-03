use parking_lot::Once;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

#[test]
fn call_once_force_recovers_from_poison_and_runs_only_once_successfully() {
    let once = Once::new();
    let attempts = AtomicUsize::new(0);
    let saw_unpoisoned_initial_state = AtomicBool::new(false);
    let saw_poisoned_retry_state = AtomicBool::new(false);

    let initial_state = once.state();
    assert!(!initial_state.done());
    assert!(!initial_state.poisoned());

    let first_attempt = catch_unwind(AssertUnwindSafe(|| {
        once.call_once_force(|state| {
            attempts.fetch_add(1, Ordering::SeqCst);
            saw_unpoisoned_initial_state.store(!state.poisoned() && !state.done(), Ordering::SeqCst);
            panic!("intentional initialization failure");
        });
    }));

    assert!(first_attempt.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert!(saw_unpoisoned_initial_state.load(Ordering::SeqCst));

    let poisoned_state = once.state();
    assert!(!poisoned_state.done());
    assert!(poisoned_state.poisoned());

    once.call_once_force(|state| {
        attempts.fetch_add(1, Ordering::SeqCst);
        saw_poisoned_retry_state.store(state.poisoned() && !state.done(), Ordering::SeqCst);
    });

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(saw_poisoned_retry_state.load(Ordering::SeqCst));

    let completed_state = once.state();
    assert!(completed_state.done());
    assert!(!completed_state.poisoned());

    once.call_once_force(|_| {
        attempts.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[test]
fn call_once_force_allows_one_successful_initializer_after_thread_panic() {
    let once = Arc::new(Once::new());
    let successful_initializers = Arc::new(AtomicUsize::new(0));
    let poisoned_states_observed = Arc::new(AtomicUsize::new(0));

    let panicking_once = Arc::clone(&once);
    let panicking_thread = thread::spawn(move || {
        panicking_once.call_once_force(|state| {
            assert!(!state.done());
            assert!(!state.poisoned());
            panic!("intentional thread initializer panic");
        });
    });

    assert!(panicking_thread.join().is_err());

    let after_panic = once.state();
    assert!(!after_panic.done());
    assert!(after_panic.poisoned());

    let mut workers = Vec::new();
    for _ in 0..8 {
        let worker_once = Arc::clone(&once);
        let worker_successes = Arc::clone(&successful_initializers);
        let worker_poisoned_observed = Arc::clone(&poisoned_states_observed);

        workers.push(thread::spawn(move || {
            worker_once.call_once_force(|state| {
                if state.poisoned() {
                    worker_poisoned_observed.fetch_add(1, Ordering::SeqCst);
                }
                assert!(!state.done());
                worker_successes.fetch_add(1, Ordering::SeqCst);
            });
        }));
    }

    for worker in workers {
        assert!(worker.join().is_ok());
    }

    assert_eq!(successful_initializers.load(Ordering::SeqCst), 1);
    assert_eq!(poisoned_states_observed.load(Ordering::SeqCst), 1);

    let final_state = once.state();
    assert!(final_state.done());
    assert!(!final_state.poisoned());
}