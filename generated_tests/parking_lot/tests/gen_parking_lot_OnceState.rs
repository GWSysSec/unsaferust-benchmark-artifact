use parking_lot::{Once, OnceState};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;

#[test]
fn once_state_done_initial_and_after_completion() {
    let once = Once::new();

    let initial = once.state();
    assert_eq!(initial.done(), false);
    assert_eq!(initial.poisoned(), false);

    let mut count: u32 = 0;
    once.call_once(|| {
        count += 1;
    });
    assert_eq!(count, 1);

    let after = once.state();
    assert_eq!(after.done(), true);
    assert_eq!(after.poisoned(), false);
    assert_ne!(after.done(), initial.done());


    once.call_once(|| {
        count = count.wrapping_add(100);
    });
    assert_eq!(count, 1);
    assert_eq!(once.state().done(), true);
    assert_eq!(once.state().poisoned(), false);
}

#[test]
fn once_state_done_false_when_poisoned_then_recovered() {
    let once = Arc::new(Once::new());

    assert_eq!(once.state().done(), false);
    assert_eq!(once.state().poisoned(), false);

    let once_clone = Arc::clone(&once);
    let join_result = thread::spawn(move || {
        once_clone.call_once(|| {
            panic!("intentional poisoning");
        });
    })
    .join();
    assert_eq!(join_result.is_err(), true);


    let poisoned_state = once.state();
    assert_eq!(poisoned_state.done(), false);
    assert_eq!(poisoned_state.poisoned(), true);
    assert_ne!(poisoned_state.done(), poisoned_state.poisoned());


    let invoked = AtomicBool::new(false);
    let saw_poisoned_inside = AtomicBool::new(false);
    let saw_not_done_inside = AtomicBool::new(false);
    once.call_once_force(|state: OnceState| {
        invoked.store(true, Ordering::SeqCst);
        saw_poisoned_inside.store(state.poisoned(), Ordering::SeqCst);
        saw_not_done_inside.store(!state.done(), Ordering::SeqCst);
    });
    assert_eq!(invoked.load(Ordering::SeqCst), true);
    assert_eq!(saw_poisoned_inside.load(Ordering::SeqCst), true);
    assert_eq!(saw_not_done_inside.load(Ordering::SeqCst), true);

    let recovered = once.state();
    assert_eq!(recovered.done(), true);
    assert_eq!(recovered.poisoned(), false);
}

#[test]
fn once_state_done_visible_across_threads() {
    let once = Arc::new(Once::new());
    let init_counter = Arc::new(AtomicU32::new(0));

    assert_eq!(once.state().done(), false);
    assert_eq!(once.state().poisoned(), false);
    assert_eq!(init_counter.load(Ordering::SeqCst), 0);

    let mut handles = Vec::with_capacity(4);
    for _ in 0..4 {
        let once = Arc::clone(&once);
        let counter = Arc::clone(&init_counter);
        handles.push(thread::spawn(move || -> (bool, bool) {
            once.call_once(|| {
                counter.fetch_add(1, Ordering::SeqCst);
            });
            let s = once.state();
            (s.done(), s.poisoned())
        }));
    }

    let mut done_true_count = 0u32;
    let mut poisoned_true_count = 0u32;
    for h in handles {
        let (done, poisoned) = h.join().expect("worker thread panicked");
        if done {
            done_true_count += 1;
        }
        if poisoned {
            poisoned_true_count += 1;
        }
    }
    assert_eq!(done_true_count, 4);
    assert_eq!(poisoned_true_count, 0);
    assert_eq!(init_counter.load(Ordering::SeqCst), 1);
    assert_eq!(once.state().done(), true);
    assert_eq!(once.state().poisoned(), false);
}

#[test]
fn once_state_done_receives_false_inside_first_force_call() {
    let once = Once::new();
    assert_eq!(once.state().done(), false);
    assert_eq!(once.state().poisoned(), false);

    let inside_done = AtomicBool::new(true);
    let inside_poisoned = AtomicBool::new(true);
    let call_count = AtomicU32::new(0);

    once.call_once_force(|state: OnceState| {
        call_count.fetch_add(1, Ordering::SeqCst);
        inside_done.store(state.done(), Ordering::SeqCst);
        inside_poisoned.store(state.poisoned(), Ordering::SeqCst);
    });
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    assert_eq!(inside_done.load(Ordering::SeqCst), false);
    assert_eq!(inside_poisoned.load(Ordering::SeqCst), false);

    let after_first = once.state();
    assert_eq!(after_first.done(), true);
    assert_eq!(after_first.poisoned(), false);


    once.call_once_force(|_state: OnceState| {
        call_count.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    assert_eq!(once.state().done(), true);
    assert_eq!(once.state().poisoned(), false);
}

#[test]
fn once_state_done_stable_after_many_redundant_calls() {
    let once = Once::new();
    assert_eq!(once.state().done(), false);

    let exec = AtomicU32::new(0);
    once.call_once(|| {
        exec.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(exec.load(Ordering::SeqCst), 1);
    assert_eq!(once.state().done(), true);
    assert_eq!(once.state().poisoned(), false);

    for _ in 0..32 {
        once.call_once(|| {
            exec.fetch_add(1000, Ordering::SeqCst);
        });
    }
    assert_eq!(exec.load(Ordering::SeqCst), 1);


    let s_a = once.state();
    let s_b = once.state();
    let s_c = once.state();
    assert_eq!(s_a.done(), true);
    assert_eq!(s_b.done(), true);
    assert_eq!(s_c.done(), true);
    assert_eq!(s_a.done(), s_b.done());
    assert_eq!(s_b.done(), s_c.done());
    assert_eq!(s_a.poisoned(), false);
}