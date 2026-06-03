use std::panic;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[test]
fn catch_returns_value_and_preserves_side_effects_for_successful_closures() {
    curl::init();

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_for_first_call = Arc::clone(&counter);

    let first = panic::catch_unwind(move || {
        counter_for_first_call.fetch_add(1, Ordering::SeqCst);
        "completed"
    })
    .ok();

    assert_eq!(first, Some("completed"));
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    let mut values = vec![1, 2, 3];
    let second = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        values.push(4);
        values.iter().sum::<i32>()
    }))
    .ok();

    assert_eq!(second, Some(10));
    assert_eq!(values, vec![1, 2, 3, 4]);
}

#[test]
fn catch_converts_panics_to_none_and_allows_later_successful_calls() {
    curl::init();

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_for_panic = Arc::clone(&counter);

    let panicked = panic::catch_unwind(move || {
        counter_for_panic.fetch_add(1, Ordering::SeqCst);
        panic!("intentional panic to verify panic catching");
    })
    .ok();

    assert_eq!(panicked, None);
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    let counter_for_recovery = Arc::clone(&counter);
    let recovered = panic::catch_unwind(move || {
        counter_for_recovery.fetch_add(10, Ordering::SeqCst);
        counter_for_recovery.load(Ordering::SeqCst)
    })
    .ok();

    assert_eq!(recovered, Some(11));
    assert_eq!(counter.load(Ordering::SeqCst), 11);
}