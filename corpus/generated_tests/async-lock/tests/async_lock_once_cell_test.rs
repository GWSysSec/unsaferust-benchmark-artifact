use std::sync::atomic::{AtomicUsize, Ordering};

use async_lock::OnceCell;
use futures_lite::future;

#[test]
fn async_once_cell_set_get_unchecked_take_and_reinitialize() {
    future::block_on(async {
        let mut cell = OnceCell::new();

        assert!(!cell.is_initialized());
        assert!(cell.get().is_none());

        let initialized = cell.set(String::from("first")).await;
        assert!(initialized.is_ok());
        assert_eq!(initialized.unwrap().as_str(), "first");
        assert!(cell.is_initialized());
        assert_eq!(cell.get().map(String::as_str), Some("first"));

        let unchecked = unsafe { cell.get_unchecked() };
        assert_eq!(unchecked.as_str(), "first");

        let duplicate = cell.set(String::from("second")).await;
        assert_eq!(duplicate, Err(String::from("second")));
        assert_eq!(cell.get().map(String::as_str), Some("first"));

        assert_eq!(cell.take(), Some(String::from("first")));
        assert!(!cell.is_initialized());
        assert!(cell.get().is_none());

        let reinitialized = cell.set(String::from("third")).await;
        assert!(reinitialized.is_ok());
        assert_eq!(cell.get().map(String::as_str), Some("third"));
    });
}

#[test]
fn get_or_try_init_retries_after_async_error_and_caches_success() {
    future::block_on(async {
        let cell = OnceCell::<usize>::new();
        let attempts = AtomicUsize::new(0);

        let first: Result<&usize, &'static str> = cell
            .get_or_try_init(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err("not ready")
            })
            .await;

        assert_eq!(first, Err("not ready"));
        assert!(!cell.is_initialized());
        assert!(cell.get().is_none());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);

        let second: Result<&usize, &'static str> = cell
            .get_or_try_init(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            })
            .await;

        assert_eq!(second.copied(), Ok(42));
        assert!(cell.is_initialized());
        assert_eq!(cell.get().copied(), Some(42));
        assert_eq!(attempts.load(Ordering::SeqCst), 2);

        let third: Result<&usize, &'static str> = cell
            .get_or_try_init(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Ok(100)
            })
            .await;

        assert_eq!(third.copied(), Ok(42));
        assert_eq!(cell.get().copied(), Some(42));
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    });
}

#[test]
fn get_or_init_async_runs_initializer_only_once() {
    future::block_on(async {
        let cell = OnceCell::<Vec<&'static str>>::new();
        let calls = AtomicUsize::new(0);

        let first = cell
            .get_or_init(|| async {
                calls.fetch_add(1, Ordering::SeqCst);
                vec!["alpha", "beta"]
            })
            .await;

        assert_eq!(first.as_slice(), ["alpha", "beta"]);
        assert!(cell.is_initialized());

        let second = cell
            .get_or_init(|| async {
                calls.fetch_add(1, Ordering::SeqCst);
                vec!["unexpected"]
            })
            .await;

        assert_eq!(second.as_slice(), ["alpha", "beta"]);
        assert_eq!(cell.get().map(Vec::len), Some(2));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn blocking_try_init_then_get_or_init_uses_cached_value() {
    let cell = OnceCell::<String>::new();
    let attempts = AtomicUsize::new(0);

    let failed: Result<&String, &'static str> = cell.get_or_try_init_blocking(|| {
        attempts.fetch_add(1, Ordering::SeqCst);
        Err("missing configuration")
    });

    assert_eq!(failed, Err("missing configuration"));
    assert!(!cell.is_initialized());
    assert!(cell.get().is_none());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);

    let initialized: Result<&String, &'static str> = cell.get_or_try_init_blocking(|| {
        attempts.fetch_add(1, Ordering::SeqCst);
        Ok(String::from("configured"))
    });

    assert_eq!(initialized.map(String::as_str), Ok("configured"));
    assert!(cell.is_initialized());
    assert_eq!(cell.get().map(String::as_str), Some("configured"));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);

    let cached = cell.get_or_init_blocking(|| {
        attempts.fetch_add(1, Ordering::SeqCst);
        String::from("replacement")
    });

    assert_eq!(cached.as_str(), "configured");
    assert_eq!(cell.get().map(String::as_str), Some("configured"));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[test]
fn set_blocking_rejects_second_value_and_take_allows_reuse() {
    let mut cell = OnceCell::new();

    let first = cell.set_blocking(7);
    assert_eq!(first.copied(), Ok(7));
    assert!(cell.is_initialized());
    assert_eq!(cell.get().copied(), Some(7));

    let second = cell.set_blocking(8);
    assert_eq!(second, Err(8));
    assert_eq!(cell.get().copied(), Some(7));

    assert_eq!(cell.take(), Some(7));
    assert!(!cell.is_initialized());
    assert!(cell.get().is_none());

    let reused = cell.set_blocking(9);
    assert_eq!(reused.copied(), Ok(9));
    assert_eq!(unsafe { *cell.get_unchecked() }, 9);
}