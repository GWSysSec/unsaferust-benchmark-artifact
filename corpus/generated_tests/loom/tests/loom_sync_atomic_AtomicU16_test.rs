#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicU16, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_u16_sequential_read_modify_write_workflow() {
    loom::model(|| {
        let mut atomic = AtomicU16::new(7);

        let returned_from_with_mut = atomic.with_mut(|value| {
            assert_eq!(*value, 7);
            *value = 0b1100;
            *value + 3
        });
        assert_eq!(returned_from_with_mut, 0b1111);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1100);

        let previous = atomic.swap(25, Ordering::SeqCst);
        assert_eq!(previous, 0b1100);
        assert_eq!(atomic.load(Ordering::SeqCst), 25);

        let failed = atomic.compare_exchange_weak(24, 99, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(25));
        assert_eq!(atomic.load(Ordering::SeqCst), 25);

        let previous = atomic.fetch_sub(5, Ordering::SeqCst);
        assert_eq!(previous, 25);
        assert_eq!(atomic.load(Ordering::SeqCst), 20);

        let previous = atomic.fetch_and(0b1110, Ordering::SeqCst);
        assert_eq!(previous, 20);
        assert_eq!(atomic.load(Ordering::SeqCst), 20 & 0b1110);

        let previous = atomic.fetch_or(0b0011, Ordering::SeqCst);
        assert_eq!(previous, 20 & 0b1110);
        assert_eq!(atomic.load(Ordering::SeqCst), (20 & 0b1110) | 0b0011);

        let before_nand = atomic.load(Ordering::SeqCst);
        let previous = atomic.fetch_nand(0b0111, Ordering::SeqCst);
        assert_eq!(previous, before_nand);
        assert_eq!(atomic.load(Ordering::SeqCst), !(before_nand & 0b0111));

        let before_update = atomic.load(Ordering::SeqCst);
        let updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, before_update);
            Some(current.wrapping_add(1))
        });
        assert_eq!(updated, Ok(before_update));
        assert_eq!(atomic.load(Ordering::SeqCst), before_update.wrapping_add(1));

        let before_rejected_update = atomic.load(Ordering::SeqCst);
        let rejected = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, before_rejected_update);
            None
        });
        assert_eq!(rejected, Err(before_rejected_update));
        assert_eq!(atomic.load(Ordering::SeqCst), before_rejected_update);
    });
}

#[test]
fn atomic_u16_concurrent_updates_preserve_accounting_invariants() {
    loom::model(|| {
        const LEFT_FLAG: u16 = 0x1000;
        const RIGHT_FLAG: u16 = 0x2000;
        const FLAG_MASK: u16 = LEFT_FLAG | RIGHT_FLAG;
        const COUNTER_MASK: u16 = !FLAG_MASK;
        const INITIAL_COUNTER: u16 = 100;

        let atomic = Arc::new(AtomicU16::new(INITIAL_COUNTER));

        let left = Arc::clone(&atomic);
        let left_thread = thread::spawn(move || {
            let old = left.fetch_or(LEFT_FLAG, Ordering::SeqCst);
            assert_eq!(old & LEFT_FLAG, 0);

            let update = left.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                assert_ne!(current & LEFT_FLAG, 0);
                assert!(
                    (current & COUNTER_MASK) == INITIAL_COUNTER
                        || (current & COUNTER_MASK) == INITIAL_COUNTER - 1
                );
                Some(current + 10)
            });

            let previous = update.expect("left counter increment should always succeed");
            assert_ne!(previous & LEFT_FLAG, 0);
            assert!(
                (previous & COUNTER_MASK) == INITIAL_COUNTER
                    || (previous & COUNTER_MASK) == INITIAL_COUNTER - 1
            );
        });

        let right = Arc::clone(&atomic);
        let right_thread = thread::spawn(move || {
            let old = right.fetch_or(RIGHT_FLAG, Ordering::SeqCst);
            assert_eq!(old & RIGHT_FLAG, 0);

            let previous = right.fetch_sub(1, Ordering::SeqCst);
            assert_ne!(previous & RIGHT_FLAG, 0);
            assert!(
                (previous & COUNTER_MASK) == INITIAL_COUNTER
                    || (previous & COUNTER_MASK) == INITIAL_COUNTER + 10
            );
        });

        left_thread.join().expect("left worker thread panicked");
        right_thread.join().expect("right worker thread panicked");

        let final_value = atomic.load(Ordering::SeqCst);
        assert_eq!(final_value & FLAG_MASK, FLAG_MASK);
        assert_eq!(final_value & COUNTER_MASK, INITIAL_COUNTER + 10 - 1);
        assert_eq!(final_value, FLAG_MASK | (INITIAL_COUNTER + 10 - 1));

        let swapped_out = atomic.swap(0b1111_0000, Ordering::SeqCst);
        assert_eq!(swapped_out, final_value);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1111_0000);

        let failed = atomic.compare_exchange_weak(
            0b0000_1111,
            0,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert_eq!(failed, Err(0b1111_0000));

        let before_and = atomic.fetch_and(0b1010_1010, Ordering::SeqCst);
        assert_eq!(before_and, 0b1111_0000);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1010_0000);

        let before_nand = atomic.fetch_nand(0b1111_1111, Ordering::SeqCst);
        assert_eq!(before_nand, 0b1010_0000);
        assert_eq!(atomic.load(Ordering::SeqCst), !0b1010_0000u16);
    });
}