use parking_lot::{Once, OnceState};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

#[test]
fn test_call_once_runs_exactly_once() {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let once = Once::new();

    assert_eq!(COUNTER.load(Ordering::SeqCst), 0);
    assert_eq!(once.state().done(), false);

    once.call_once(|| {
        COUNTER.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(once.state().done(), true);


    once.call_once(|| {
        COUNTER.fetch_add(100, Ordering::SeqCst);
    });
    once.call_once(|| {
        COUNTER.fetch_add(100, Ordering::SeqCst);
    });

    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    assert_ne!(COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(once.state().done(), true);
}

#[test]
fn test_call_once_concurrent() {
    let once = Arc::new(Once::new());
    let counter = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(once.state().done(), false);

    let mut handles = Vec::new();
    for i in 0..4 {
        let once_c = Arc::clone(&once);
        let counter_c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            once_c.call_once(|| {
                counter_c.fetch_add(1, Ordering::SeqCst);
            });
            let _ = i;
        }));
    }

    for h in handles {
        h.join().expect("thread panicked");
    }

    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_ne!(counter.load(Ordering::SeqCst), 4);
    assert_eq!(once.state().done(), true);
    assert!(once.state().done());
}

#[test]
fn test_call_once_force_after_poison() {
    let once = Arc::new(Once::new());

    assert_eq!(once.state().done(), false);
    assert_eq!(once.state().poisoned(), false);


    let once_p = Arc::clone(&once);
    let result = thread::spawn(move || {
        once_p.call_once(|| {
            panic!("intentional poison");
        });
    })
    .join();

    assert!(result.is_err());
    assert_eq!(once.state().poisoned(), true);
    assert!(once.state().poisoned());
    assert_eq!(once.state().done(), false);


    let once_q = Arc::clone(&once);
    let panicked = thread::spawn(move || {
        once_q.call_once(|| {});
    })
    .join();
    assert!(panicked.is_err());


    let observed_poison = Arc::new(AtomicUsize::new(0));
    let observed_clone = Arc::clone(&observed_poison);
    let ran = Arc::new(AtomicUsize::new(0));
    let ran_clone = Arc::clone(&ran);

    once.call_once_force(|state: OnceState| {
        if state.poisoned() {
            observed_clone.store(1, Ordering::SeqCst);
        }
        ran_clone.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(ran.load(Ordering::SeqCst), 1);
    assert_eq!(observed_poison.load(Ordering::SeqCst), 1);
    assert_eq!(once.state().done(), true);
    assert_eq!(once.state().poisoned(), false);


    once.call_once_force(|_| {
        ran.fetch_add(10, Ordering::SeqCst);
    });
    once.call_once(|| {
        ran.fetch_add(10, Ordering::SeqCst);
    });
    assert_eq!(ran.load(Ordering::SeqCst), 1);
    assert_ne!(ran.load(Ordering::SeqCst), 11);
}

#[test]
fn test_call_once_force_clean_state() {
    let once = Once::new();
    let mut observed_poisoned = true;
    let mut runs = 0;

    assert_eq!(once.state().done(), false);
    assert_eq!(once.state().poisoned(), false);

    once.call_once_force(|state: OnceState| {
        observed_poisoned = state.poisoned();
        runs += 1;
    });

    assert_eq!(observed_poisoned, false);
    assert_eq!(runs, 1);
    assert_eq!(once.state().done(), true);
    assert_eq!(once.state().poisoned(), false);


    once.call_once_force(|_| {
        runs += 1;
    });
    assert_eq!(runs, 1);
    assert_ne!(runs, 2);
    assert!(once.state().done());
}

#[test]
fn test_multi_step_initialization_workflow() {
    let once = Arc::new(Once::new());
    let value = Arc::new(AtomicUsize::new(0));

    assert_eq!(value.load(Ordering::SeqCst), 0);
    assert_eq!(once.state().done(), false);


    let mut handles = Vec::new();
    for _ in 0..3 {
        let once_c = Arc::clone(&once);
        let value_c = Arc::clone(&value);
        handles.push(thread::spawn(move || {
            once_c.call_once(|| {
                value_c.store(42, Ordering::SeqCst);
            });
            value_c.load(Ordering::SeqCst)
        }));
    }

    let mut observed = Vec::new();
    for h in handles {
        observed.push(h.join().expect("join failed"));
    }

    assert_eq!(value.load(Ordering::SeqCst), 42);
    assert_eq!(once.state().done(), true);
    assert_eq!(observed.len(), 3);
    for v in &observed {
        assert_eq!(*v, 42);
    }


    once.call_once_force(|state: OnceState| {
        assert_eq!(state.poisoned(), false);
        value.store(999, Ordering::SeqCst);
    });
    assert_eq!(value.load(Ordering::SeqCst), 42);
    assert_ne!(value.load(Ordering::SeqCst), 999);
    assert_eq!(once.state().poisoned(), false);
}