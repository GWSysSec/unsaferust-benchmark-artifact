#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicU32, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_u32_sequential_read_modify_write_and_bitwise_workflow() {
    loom::model(|| {
        let mut atomic = AtomicU32::new(10);

        let returned_from_with_mut = atomic.with_mut(|value| {
            assert_eq!(*value, 10);
            *value = 0b1100;
            *value + 3
        });
        assert_eq!(returned_from_with_mut, 0b1111);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1100);

        let previous = atomic.swap(100, Ordering::SeqCst);
        assert_eq!(previous, 0b1100);
        assert_eq!(atomic.load(Ordering::SeqCst), 100);

        let failed = atomic.compare_exchange_weak(99, 200, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(100));
        assert_eq!(atomic.load(Ordering::SeqCst), 100);

        let previous = atomic.fetch_sub(37, Ordering::SeqCst);
        assert_eq!(previous, 100);
        assert_eq!(atomic.load(Ordering::SeqCst), 63);

        let previous = atomic.fetch_and(0b0011_1110, Ordering::SeqCst);
        assert_eq!(previous, 63);
        assert_eq!(atomic.load(Ordering::SeqCst), 62);

        let previous = atomic.fetch_or(0b1000_0001, Ordering::SeqCst);
        assert_eq!(previous, 62);
        assert_eq!(atomic.load(Ordering::SeqCst), 191);

        let before_nand = atomic.load(Ordering::SeqCst);
        let previous = atomic.fetch_nand(0b1111_0000, Ordering::SeqCst);
        assert_eq!(previous, before_nand);
        assert_eq!(
            atomic.load(Ordering::SeqCst),
            !(before_nand & 0b1111_0000)
        );

        let previous = atomic.swap(40, Ordering::SeqCst);
        assert_eq!(previous, !(before_nand & 0b1111_0000));
        assert_eq!(atomic.load(Ordering::SeqCst), 40);

        let updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current % 2 == 0 {
                Some(current / 2)
            } else {
                None
            }
        });
        assert_eq!(updated, Ok(40));
        assert_eq!(atomic.load(Ordering::SeqCst), 20);

        let not_updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current > 100 {
                Some(current - 100)
            } else {
                None
            }
        });
        assert_eq!(not_updated, Err(20));
        assert_eq!(atomic.load(Ordering::SeqCst), 20);
    });
}

#[test]
fn atomic_u32_concurrent_fetch_sub_or_and_update_workflow() {
    loom::model(|| {
        let counter = Arc::new(AtomicU32::new(4));
        let observed_sum = Arc::new(AtomicU32::new(0));
        let flags = Arc::new(AtomicU32::new(0));

        let counter_a = Arc::clone(&counter);
        let observed_sum_a = Arc::clone(&observed_sum);
        let flags_a = Arc::clone(&flags);
        let thread_a = thread::spawn(move || {
            let previous = counter_a.fetch_sub(1, Ordering::SeqCst);
            assert!(previous == 4 || previous == 3);
            observed_sum_a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |sum| {
                Some(sum + previous)
            }).expect("fetch_update should always add the observed value");
            let old_flags = flags_a.fetch_or(0b01, Ordering::SeqCst);
            assert_eq!(old_flags & 0b01, 0);
        });

        let counter_b = Arc::clone(&counter);
        let observed_sum_b = Arc::clone(&observed_sum);
        let flags_b = Arc::clone(&flags);
        let thread_b = thread::spawn(move || {
            let previous = counter_b.fetch_sub(1, Ordering::SeqCst);
            assert!(previous == 4 || previous == 3);
            observed_sum_b.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |sum| {
                Some(sum + previous)
            }).expect("fetch_update should always add the observed value");
            let old_flags = flags_b.fetch_or(0b10, Ordering::SeqCst);
            assert_eq!(old_flags & 0b10, 0);
        });

        thread_a.join().expect("thread A should complete");
        thread_b.join().expect("thread B should complete");

        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(observed_sum.load(Ordering::SeqCst), 7);
        assert_eq!(flags.load(Ordering::SeqCst), 0b11);

        let masked_previous = flags.fetch_and(0b01, Ordering::SeqCst);
        assert_eq!(masked_previous, 0b11);
        assert_eq!(flags.load(Ordering::SeqCst), 0b01);

        let nand_previous = flags.fetch_nand(0b01, Ordering::SeqCst);
        assert_eq!(nand_previous, 0b01);
        assert_eq!(flags.load(Ordering::SeqCst), !0b01);

        let replaced = counter.swap(8, Ordering::SeqCst);
        assert_eq!(replaced, 2);
        assert_eq!(counter.load(Ordering::SeqCst), 8);

        let result = counter.compare_exchange_weak(9, 100, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(result, Err(8));
        assert_eq!(counter.load(Ordering::SeqCst), 8);
    });
}