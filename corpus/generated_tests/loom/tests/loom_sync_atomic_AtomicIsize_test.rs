#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicIsize, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_isize_sequential_read_modify_write_workflow() {
    loom::model(|| {
        let mut atomic = AtomicIsize::new(7);

        let returned = atomic.with_mut(|value| {
            assert_eq!(*value, 7);
            *value = 0b1100;
            *value + 5
        });
        assert_eq!(returned, 0b1100 + 5);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1100);

        let previous = atomic.swap(25, Ordering::SeqCst);
        assert_eq!(previous, 0b1100);
        assert_eq!(atomic.load(Ordering::SeqCst), 25);

        let failed = atomic.compare_exchange_weak(24, 100, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(25));
        assert_eq!(atomic.load(Ordering::SeqCst), 25);

        let mut exchanged = false;
        for _ in 0..8 {
            match atomic.compare_exchange_weak(25, 30, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(previous) => {
                    assert_eq!(previous, 25);
                    exchanged = true;
                    break;
                }
                Err(current) => {
                    assert_eq!(current, 25);
                    thread::yield_now();
                }
            }
        }
        assert!(exchanged);
        assert_eq!(atomic.load(Ordering::SeqCst), 30);

        let previous = atomic.fetch_sub(5, Ordering::SeqCst);
        assert_eq!(previous, 30);
        assert_eq!(atomic.load(Ordering::SeqCst), 25);

        let previous = atomic.fetch_and(0b1110, Ordering::SeqCst);
        assert_eq!(previous, 25);
        assert_eq!(atomic.load(Ordering::SeqCst), 25 & 0b1110);

        let previous = atomic.fetch_or(0b0011, Ordering::SeqCst);
        assert_eq!(previous, 25 & 0b1110);
        assert_eq!(atomic.load(Ordering::SeqCst), (25 & 0b1110) | 0b0011);

        let before_nand = atomic.load(Ordering::SeqCst);
        let previous = atomic.fetch_nand(0b0111, Ordering::SeqCst);
        assert_eq!(previous, before_nand);
        assert_eq!(atomic.load(Ordering::SeqCst), !(before_nand & 0b0111));

        let update_result = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
            assert_eq!(value, !(before_nand & 0b0111));
            Some(value.wrapping_add(10))
        });
        assert_eq!(update_result, Ok(!(before_nand & 0b0111)));
        assert_eq!(
            atomic.load(Ordering::SeqCst),
            (!(before_nand & 0b0111)).wrapping_add(10)
        );

        let unchanged = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
            assert_eq!(value, (!(before_nand & 0b0111)).wrapping_add(10));
            None
        });
        assert_eq!(unchanged, Err((!(before_nand & 0b0111)).wrapping_add(10)));
        assert_eq!(
            atomic.load(Ordering::SeqCst),
            (!(before_nand & 0b0111)).wrapping_add(10)
        );
    });
}

#[test]
fn atomic_isize_concurrent_fetch_update_and_bit_operations() {
    loom::model(|| {
        let value = Arc::new(AtomicIsize::new(0b1111));

        let subtractor = {
            let value = Arc::clone(&value);
            thread::spawn(move || {
                let previous = value.fetch_sub(1, Ordering::SeqCst);
                assert!((0b1110..=0b1111).contains(&previous));
                previous
            })
        };

        let updater = {
            let value = Arc::clone(&value);
            thread::spawn(move || {
                let result = value.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    if current & 1 == 0 {
                        Some(current | 0b1000)
                    } else {
                        Some(current - 1)
                    }
                });
                assert!(result.is_ok());
                let previous = result.unwrap_or_else(|current| current);
                assert!((0b1110..=0b1111).contains(&previous));
                previous
            })
        };

        let first_seen = subtractor.join().expect("subtractor thread panicked");
        let second_seen = updater.join().expect("updater thread panicked");
        assert_ne!(first_seen, second_seen);

        let after_threads = value.load(Ordering::SeqCst);
        let expected_after_threads = if first_seen == 0b1111 {
            0b1110
        } else {
            0b1101
        };
        assert_eq!(after_threads, expected_after_threads);

        let previous = value.fetch_and(0b1010, Ordering::SeqCst);
        assert_eq!(previous, expected_after_threads);
        let after_and = expected_after_threads & 0b1010;
        assert_eq!(value.load(Ordering::SeqCst), after_and);

        let previous = value.fetch_or(0b0101, Ordering::SeqCst);
        assert_eq!(previous, after_and);
        let after_or = after_and | 0b0101;
        assert_eq!(value.load(Ordering::SeqCst), after_or);

        let previous = value.fetch_nand(0b0011, Ordering::SeqCst);
        assert_eq!(previous, after_or);
        let after_nand = !(after_or & 0b0011);
        assert_eq!(value.load(Ordering::SeqCst), after_nand);

        let old = value.swap(-4, Ordering::SeqCst);
        assert_eq!(old, after_nand);
        assert_eq!(value.load(Ordering::SeqCst), -4);
    });
}