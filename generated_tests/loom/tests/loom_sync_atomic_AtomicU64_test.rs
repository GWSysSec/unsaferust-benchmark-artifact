#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicU64, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_u64_sequential_bitwise_and_update_workflow() {
    loom::model(|| {
        let mut atomic = AtomicU64::new(7);

        let returned = atomic.with_mut(|value| {
            assert_eq!(*value, 7);
            *value = 0b1111_0000;
            (*value).count_ones()
        });
        assert_eq!(returned, 4);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1111_0000);

        let previous = atomic.swap(100, Ordering::SeqCst);
        assert_eq!(previous, 0b1111_0000);
        assert_eq!(atomic.load(Ordering::SeqCst), 100);

        let failed = atomic.compare_exchange_weak(99, 200, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(100));
        assert_eq!(atomic.load(Ordering::SeqCst), 100);

        let previous = atomic.fetch_sub(36, Ordering::SeqCst);
        assert_eq!(previous, 100);
        assert_eq!(atomic.load(Ordering::SeqCst), 64);

        let previous = atomic.fetch_and(0b0111_0000, Ordering::SeqCst);
        assert_eq!(previous, 64);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b0100_0000);

        let previous = atomic.fetch_or(0b0000_1010, Ordering::SeqCst);
        assert_eq!(previous, 0b0100_0000);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b0100_1010);

        let before_nand = atomic.load(Ordering::SeqCst);
        let previous = atomic.fetch_nand(0b0111_1111, Ordering::SeqCst);
        assert_eq!(previous, before_nand);
        assert_eq!(atomic.load(Ordering::SeqCst), !(before_nand & 0b0111_1111));

        let update_result = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, !(before_nand & 0b0111_1111));
            Some(current & 0xff)
        });
        assert_eq!(update_result, Ok(!(before_nand & 0b0111_1111)));
        assert_eq!(
            atomic.load(Ordering::SeqCst),
            (!(before_nand & 0b0111_1111)) & 0xff
        );

        let rejected = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, 0xb5);
            None
        });
        assert_eq!(rejected, Err(0xb5));
        assert_eq!(atomic.load(Ordering::SeqCst), 0xb5);
    });
}

#[test]
fn atomic_u64_concurrent_fetch_update_and_flags_workflow() {
    loom::model(|| {
        let counter = Arc::new(AtomicU64::new(10));
        let flags = Arc::new(AtomicU64::new(0b0011));

        let counter_worker = Arc::clone(&counter);
        let flags_worker = Arc::clone(&flags);
        let worker = thread::spawn(move || {
            let old_counter =
                counter_worker.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    Some(current + 5)
                });
            assert!(old_counter == Ok(10) || old_counter == Ok(8));

            let previous_flags = flags_worker.fetch_or(0b0100, Ordering::SeqCst);
            assert_eq!(previous_flags & 0b0100, 0);
        });

        let previous_counter = counter.fetch_sub(2, Ordering::SeqCst);
        assert!(previous_counter == 10 || previous_counter == 15);

        let previous_flags = flags.fetch_and(0b0111, Ordering::SeqCst);
        assert_eq!(previous_flags & 0b0011, 0b0011);

        worker.join().expect("worker thread should complete successfully");

        let final_counter = counter.load(Ordering::SeqCst);
        assert_eq!(final_counter, 13);

        let flags_after_worker = flags.load(Ordering::SeqCst);
        assert_eq!(flags_after_worker & 0b0111, flags_after_worker);

        let previous_flags = flags.fetch_nand(0b0101, Ordering::SeqCst);
        assert_eq!(previous_flags & 0b0011, 0b0011);
        assert_eq!(flags.load(Ordering::SeqCst), !(previous_flags & 0b0101));

        let swapped = counter.swap(0, Ordering::SeqCst);
        assert_eq!(swapped, 13);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    });
}