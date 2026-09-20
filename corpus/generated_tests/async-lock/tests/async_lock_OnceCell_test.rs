use std::sync::atomic::{AtomicUsize, Ordering};

use async_lock::OnceCell;
use futures_lite::future;

#[test]
fn async_set_get_unchecked_and_take_workflow() {
    future::block_on(async {
        let mut cell = OnceCell::new();

        assert!(!cell.is_initialized());
        assert!(cell.get().is_none());

        let first = cell.set(String::from("alpha")).await;
        assert!(first.is_ok());
        assert_eq!(first.unwrap().as_str(), "alpha");
        assert!(cell.is_initialized());
        assert_eq!(cell.get().map(String::as_str), Some("alpha"));

        assert_eq!(unsafe { cell.get_unchecked() }.as_str(), "alpha");

        let duplicate = cell.set(String::from("beta")).await;
        assert_eq!(duplicate, Err(String::from("beta")));
        assert_eq!(cell.get().map(String::as_str), Some("alpha"));

        assert_eq!(cell.take(), Some(String::from("alpha")));
        assert!(!cell.is_initialized());
        assert!(cell.get().is_none());

        let second = cell.set(String::from("gamma")).await;
        assert!(second.is_ok());
        assert_eq!(cell.get().map(String::as_str), Some("gamma"));
    });
}

#[test]
fn async_get_or_try_init_retries_after_error_and_caches_success() {
    future::block_on(async {
        let cell = OnceCell::<i32>::new();
        let attempts = AtomicUsize::new(0);

        let failed: Result<&i32, &'static str> = cell
            .get_or_try_init(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err("temporary failure")
            })
            .await;

        assert_eq!(failed, Err("temporary failure"));
        assert!(!cell.is_initialized());
        assert!(cell.get().is_none());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);

        let initialized = cell
            .get_or_try_init(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, &'static str>(42)
            })
            .await;

        assert_eq!(initialized, Ok(&42));
        assert!(cell.is_initialized());
        assert_eq!(cell.get(), Some(&42));
        assert_eq!(attempts.load(Ordering::SeqCst), 2);

        let cached = cell
            .get_or_try_init(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err::<i32, &'static str>("should not run")
            })
            .await;

        assert_eq!(cached, Ok(&42));
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    });
}

#[test]
fn async_get_or_init_invokes_initializer_only_once() {
    future::block_on(async {
        let cell = OnceCell::<usize>::new();
        let calls = AtomicUsize::new(0);

        let first = cell
            .get_or_init(|| async {
                calls.fetch_add(1, Ordering::SeqCst);
                7
            })
            .await;

        assert_eq!(first, &7);
        assert!(cell.is_initialized());
        assert_eq!(cell.get(), Some(&7));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = cell
            .get_or_init(|| async {
                calls.fetch_add(1, Ordering::SeqCst);
                99
            })
            .await;

        assert_eq!(second, &7);
        assert_eq!(cell.get(), Some(&7));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn blocking_once_cell_initialization_paths_cache_values() {
    let direct = OnceCell::new();

    assert!(!direct.is_initialized());
    assert_eq!(direct.set_blocking(10), Ok(&10));
    assert!(direct.is_initialized());
    assert_eq!(direct.get(), Some(&10));
    assert_eq!(direct.set_blocking(20), Err(20));
    assert_eq!(direct.get(), Some(&10));

    let fallible = OnceCell::<i32>::new();
    let attempts = AtomicUsize::new(0);

    let failed: Result<&i32, &'static str> = fallible.get_or_try_init_blocking(|| {
        attempts.fetch_add(1, Ordering::SeqCst);
        Err("not yet")
    });

    assert_eq!(failed, Err("not yet"));
    assert!(!fallible.is_initialized());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);

    let initialized = fallible.get_or_try_init_blocking(|| {
        attempts.fetch_add(1, Ordering::SeqCst);
        Ok::<i32, &'static str>(33)
    });

    assert_eq!(initialized, Ok(&33));
    assert!(fallible.is_initialized());
    assert_eq!(fallible.get(), Some(&33));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);

    let cached = fallible.get_or_try_init_blocking(|| {
        attempts.fetch_add(1, Ordering::SeqCst);
        Err::<i32, &'static str>("should not run")
    });

    assert_eq!(cached, Ok(&33));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);

    let infallible = OnceCell::<String>::new();
    let calls = AtomicUsize::new(0);

    let first = infallible.get_or_init_blocking(|| {
        calls.fetch_add(1, Ordering::SeqCst);
        String::from("created")
    });

    assert_eq!(first.as_str(), "created");
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let second = infallible.get_or_init_blocking(|| {
        calls.fetch_add(1, Ordering::SeqCst);
        String::from("ignored")
    });

    assert_eq!(second.as_str(), "created");
    assert_eq!(infallible.get().map(String::as_str), Some("created"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}