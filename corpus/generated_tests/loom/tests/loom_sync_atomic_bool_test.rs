#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicBool, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_bool_sequential_bitwise_and_update_workflow() {
    loom::model(|| {
        let flag = AtomicBool::new(false);

        assert!(!flag.load(Ordering::SeqCst));

        let previous = flag.swap(true, Ordering::SeqCst);
        assert!(!previous);
        assert!(flag.load(Ordering::SeqCst));

        let failed =
            flag.compare_exchange_weak(false, false, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(true));
        assert!(flag.load(Ordering::SeqCst));

        let previous = flag.fetch_and(true, Ordering::SeqCst);
        assert!(previous);
        assert!(flag.load(Ordering::SeqCst));

        let previous = flag.fetch_and(false, Ordering::SeqCst);
        assert!(previous);
        assert!(!flag.load(Ordering::SeqCst));

        let previous = flag.fetch_or(false, Ordering::SeqCst);
        assert!(!previous);
        assert!(!flag.load(Ordering::SeqCst));

        let previous = flag.fetch_or(true, Ordering::SeqCst);
        assert!(!previous);
        assert!(flag.load(Ordering::SeqCst));

        let previous = flag.fetch_nand(true, Ordering::SeqCst);
        assert!(previous);
        assert!(!flag.load(Ordering::SeqCst));

        let previous = flag.fetch_nand(false, Ordering::SeqCst);
        assert!(!previous);
        assert!(flag.load(Ordering::SeqCst));

        let updated = flag.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert!(current);
            Some(false)
        });
        assert_eq!(updated, Ok(true));
        assert!(!flag.load(Ordering::SeqCst));

        let not_updated = flag.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert!(!current);
            None
        });
        assert_eq!(not_updated, Err(false));
        assert!(!flag.load(Ordering::SeqCst));

        assert!(!flag.into_inner());
    });
}

#[test]
fn atomic_bool_concurrent_publish_and_observe_workflow() {
    loom::model(|| {
        let published = Arc::new(AtomicBool::new(false));
        let observed = Arc::new(AtomicBool::new(false));
        let observer_ready = Arc::new(AtomicBool::new(false));

        let publisher_flag = Arc::clone(&published);
        let publisher_observed = Arc::clone(&observed);
        let publisher_ready = Arc::clone(&observer_ready);
        let publisher = thread::spawn(move || {
            while !publisher_ready.load(Ordering::SeqCst) {
                thread::yield_now();
            }

            assert!(!publisher_flag.swap(true, Ordering::SeqCst));
            assert!(publisher_flag.load(Ordering::SeqCst));

            let was_observed = publisher_observed.fetch_or(true, Ordering::SeqCst);
            assert!(was_observed);
            assert!(publisher_observed.load(Ordering::SeqCst));
        });

        let observer_flag = Arc::clone(&published);
        let observer_observed = Arc::clone(&observed);
        let observer_ready_for_thread = Arc::clone(&observer_ready);
        let observer = thread::spawn(move || {
            loop {
                match observer_flag.compare_exchange_weak(
                    false,
                    false,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(false) => break,
                    Err(false) => thread::yield_now(),
                    Ok(true) | Err(true) => {
                        panic!("publisher must not publish before observer is ready")
                    }
                }
            }

            let previous = observer_observed.fetch_nand(false, Ordering::SeqCst);
            assert!(!previous);
            assert!(observer_observed.load(Ordering::SeqCst));

            let previous = observer_observed.fetch_and(true, Ordering::SeqCst);
            assert!(previous);
            assert!(observer_observed.load(Ordering::SeqCst));

            observer_ready_for_thread.store(true, Ordering::SeqCst);
        });

        publisher.join().expect("publisher thread panicked");
        observer.join().expect("observer thread panicked");

        assert!(published.load(Ordering::SeqCst));
        assert!(observed.load(Ordering::SeqCst));
        assert!(observer_ready.load(Ordering::SeqCst));

        let final_update = observed.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current {
                Some(false)
            } else {
                None
            }
        });
        assert_eq!(final_update, Ok(true));
        assert!(!observed.load(Ordering::SeqCst));
    });
}