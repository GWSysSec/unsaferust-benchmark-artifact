#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicU8, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_u8_sequential_read_modify_write_and_bitwise_workflow() {
    loom::model(|| {
        let mut atomic = AtomicU8::new(3);

        let returned = atomic.with_mut(|value| {
            assert_eq!(*value, 3);
            *value = 0b1010_1100;
            value.count_ones()
        });
        assert_eq!(returned, 4);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1010_1100);

        let previous = atomic.swap(200, Ordering::SeqCst);
        assert_eq!(previous, 0b1010_1100);
        assert_eq!(atomic.load(Ordering::SeqCst), 200);

        let failed = atomic.compare_exchange_weak(199, 10, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(200));
        assert_eq!(atomic.load(Ordering::SeqCst), 200);

        let previous = atomic.fetch_sub(72, Ordering::SeqCst);
        assert_eq!(previous, 200);
        assert_eq!(atomic.load(Ordering::SeqCst), 128);

        let previous = atomic.fetch_and(0b1111_0000, Ordering::SeqCst);
        assert_eq!(previous, 128);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1000_0000);

        let previous = atomic.fetch_or(0b0000_1011, Ordering::SeqCst);
        assert_eq!(previous, 0b1000_0000);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1000_1011);

        let before_nand = atomic.load(Ordering::SeqCst);
        let previous = atomic.fetch_nand(0b1111_0000, Ordering::SeqCst);
        assert_eq!(previous, before_nand);
        assert_eq!(
            atomic.load(Ordering::SeqCst),
            !(before_nand & 0b1111_0000)
        );

        let before_update = atomic.load(Ordering::SeqCst);
        let update = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current & 1 == 1 {
                Some(current.wrapping_sub(1))
            } else {
                None
            }
        });

        if before_update & 1 == 1 {
            assert_eq!(update, Ok(before_update));
            assert_eq!(atomic.load(Ordering::SeqCst), before_update - 1);
        } else {
            assert_eq!(update, Err(before_update));
            assert_eq!(atomic.load(Ordering::SeqCst), before_update);
        }
    });
}

#[test]
fn atomic_u8_concurrent_fetch_update_claims_unique_bits() {
    loom::model(|| {
        let flags = Arc::new(AtomicU8::new(0));

        let left_flags = Arc::clone(&flags);
        let left = thread::spawn(move || {
            let result = left_flags.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                assert_eq!(current & !0b0000_0011, 0);
                Some(current | 0b0000_0001)
            });
            assert!(result.is_ok());
        });

        let right_flags = Arc::clone(&flags);
        let right = thread::spawn(move || {
            let result = right_flags.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                assert_eq!(current & !0b0000_0011, 0);
                Some(current | 0b0000_0010)
            });
            assert!(result.is_ok());
        });

        left.join().expect("left worker panicked");
        right.join().expect("right worker panicked");

        assert_eq!(flags.load(Ordering::SeqCst), 0b0000_0011);

        let previous = flags.swap(0b1111_0000, Ordering::SeqCst);
        assert_eq!(previous, 0b0000_0011);
        assert_eq!(flags.load(Ordering::SeqCst), 0b1111_0000);
    });
}

#[test]
fn atomic_u8_compare_exchange_weak_retry_then_arithmetic_and_masks() {
    loom::model(|| {
        let atomic = AtomicU8::new(40);

        loop {
            match atomic.compare_exchange_weak(40, 64, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(previous) => {
                    assert_eq!(previous, 40);
                    break;
                }
                Err(observed) => {
                    assert_eq!(observed, 40);
                    thread::yield_now();
                }
            }
        }

        assert_eq!(atomic.load(Ordering::SeqCst), 64);

        let previous = atomic.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(previous, 64);
        assert_eq!(atomic.load(Ordering::SeqCst), 63);

        let previous = atomic.fetch_and(0b0011_1110, Ordering::SeqCst);
        assert_eq!(previous, 63);
        assert_eq!(atomic.load(Ordering::SeqCst), 62);

        let previous = atomic.fetch_or(0b1000_0001, Ordering::SeqCst);
        assert_eq!(previous, 62);
        assert_eq!(atomic.load(Ordering::SeqCst), 191);

        let previous = atomic.fetch_nand(0b1111_1111, Ordering::SeqCst);
        assert_eq!(previous, 191);
        assert_eq!(atomic.load(Ordering::SeqCst), !191u8);

        let rejected = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current == 191 {
                Some(0)
            } else {
                None
            }
        });
        assert_eq!(rejected, Err(!191u8));
        assert_eq!(atomic.load(Ordering::SeqCst), !191u8);
    });
}