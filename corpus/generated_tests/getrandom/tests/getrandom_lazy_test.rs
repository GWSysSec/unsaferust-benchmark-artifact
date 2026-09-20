use std::cell::Cell;

#[path = "../src/lazy.rs"]
mod lazy;

#[test]
fn unsync_init_caches_true_value_and_skips_later_initializers() {
    let lazy = lazy::LazyBool::new();
    let calls = Cell::new(0);

    let first = lazy.unsync_init(|| {
        calls.set(calls.get() + 1);
        true
    });

    assert!(first);
    assert_eq!(calls.get(), 1);

    let second = lazy.unsync_init(|| {
        calls.set(calls.get() + 1);
        false
    });

    assert!(second);
    assert_eq!(calls.get(), 1);

    let third = lazy.unsync_init(|| {
        calls.set(calls.get() + 1);
        false
    });

    assert!(third);
    assert_eq!(calls.get(), 1);
}

#[test]
fn unsync_init_caches_false_value_and_supports_independent_instances() {
    let first_lazy = lazy::LazyBool::new();
    let second_lazy = lazy::LazyBool::new();

    let first_calls = Cell::new(0);
    let second_calls = Cell::new(0);

    let first_result = first_lazy.unsync_init(|| {
        first_calls.set(first_calls.get() + 1);
        false
    });

    assert!(!first_result);
    assert_eq!(first_calls.get(), 1);

    let cached_first_result = first_lazy.unsync_init(|| {
        first_calls.set(first_calls.get() + 1);
        true
    });

    assert!(!cached_first_result);
    assert_eq!(first_calls.get(), 1);

    let second_result = second_lazy.unsync_init(|| {
        second_calls.set(second_calls.get() + 1);
        true
    });

    assert!(second_result);
    assert_eq!(second_calls.get(), 1);
    assert_eq!(first_calls.get(), 1);
}