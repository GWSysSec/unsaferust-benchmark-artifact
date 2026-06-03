use parking_lot::Once;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn once_state_done_transitions_from_incomplete_to_done_after_successful_initialization() {
    let once = Once::new();
    let calls = AtomicUsize::new(0);
    let closure_observed_not_done = AtomicBool::new(false);

    let initial = once.state();
    assert!(!initial.done());
    assert!(!initial.poisoned());

    once.call_once_force(|state| {
        calls.fetch_add(1, Ordering::SeqCst);
        closure_observed_not_done.store(!state.done(), Ordering::SeqCst);
        assert!(!state.done());
        assert!(!state.poisoned());
    });

    let completed = once.state();
    assert!(completed.done());
    assert!(!completed.poisoned());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(closure_observed_not_done.load(Ordering::SeqCst));

    once.call_once_force(|_| {
        calls.fetch_add(1, Ordering::SeqCst);
        panic!("call_once_force closure must not run after Once is done");
    });

    let still_completed = once.state();
    assert!(still_completed.done());
    assert!(!still_completed.poisoned());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn once_state_done_becomes_true_after_recovering_from_poison() {
    let once = Once::new();
    let attempts = AtomicUsize::new(0);
    let first_attempt_saw_not_done = AtomicBool::new(false);
    let recovery_saw_poisoned_not_done = AtomicBool::new(false);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        once.call_once_force(|state| {
            attempts.fetch_add(1, Ordering::SeqCst);
            first_attempt_saw_not_done.store(!state.done(), Ordering::SeqCst);
            assert!(!state.done());
            assert!(!state.poisoned());
            panic!("intentional poison");
        });
    }));

    assert!(panic_result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert!(first_attempt_saw_not_done.load(Ordering::SeqCst));

    let poisoned = once.state();
    assert!(!poisoned.done());
    assert!(poisoned.poisoned());

    once.call_once_force(|state| {
        attempts.fetch_add(1, Ordering::SeqCst);
        recovery_saw_poisoned_not_done.store(state.poisoned() && !state.done(), Ordering::SeqCst);
        assert!(!state.done());
        assert!(state.poisoned());
    });

    let recovered = once.state();
    assert!(recovered.done());
    assert!(!recovered.poisoned());
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(recovery_saw_poisoned_not_done.load(Ordering::SeqCst));
}

#[test]
fn concurrent_call_once_force_runs_initializer_once_and_all_threads_observe_done_afterwards() {
    let once = Arc::new(Once::new());
    let initializer_runs = Arc::new(AtomicUsize::new(0));
    let closure_saw_not_done = Arc::new(AtomicBool::new(false));

    let mut handles = Vec::new();

    for _ in 0..8 {
        let once = Arc::clone(&once);
        let initializer_runs = Arc::clone(&initializer_runs);
        let closure_saw_not_done = Arc::clone(&closure_saw_not_done);

        handles.push(thread::spawn(move || {
            once.call_once_force(|state| {
                initializer_runs.fetch_add(1, Ordering::SeqCst);
                closure_saw_not_done.store(!state.done(), Ordering::SeqCst);
                assert!(!state.done());
                thread::sleep(Duration::from_millis(10));
            });

            assert!(once.state().done());
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    assert_eq!(initializer_runs.load(Ordering::SeqCst), 1);
    assert!(closure_saw_not_done.load(Ordering::SeqCst));

    let final_state = once.state();
    assert!(final_state.done());
    assert!(!final_state.poisoned());
}