#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_usize_sequential_read_modify_write_and_bitwise_workflow() {
    loom::model(|| {
        let mut atomic = AtomicUsize::new(9);

        let returned = atomic.with_mut(|value| {
            assert_eq!(*value, 9);
            *value = 0b1010_1100;
            value.count_ones() as usize
        });
        assert_eq!(returned, 4);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1010_1100);

        let previous = atomic.swap(64, Ordering::SeqCst);
        assert_eq!(previous, 0b1010_1100);
        assert_eq!(atomic.load(Ordering::SeqCst), 64);

        let failed = atomic.compare_exchange_weak(63, 128, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(64));
        assert_eq!(atomic.load(Ordering::SeqCst), 64);

        let previous = atomic.fetch_sub(0, Ordering::SeqCst);
        assert_eq!(previous, 64);
        assert_eq!(atomic.load(Ordering::SeqCst), 64);

        let previous = atomic.fetch_sub(22, Ordering::SeqCst);
        assert_eq!(previous, 64);
        assert_eq!(atomic.load(Ordering::SeqCst), 42);

        let previous = atomic.fetch_and(0b0011_1110, Ordering::SeqCst);
        assert_eq!(previous, 42);
        assert_eq!(atomic.load(Ordering::SeqCst), 42 & 0b0011_1110);

        let previous = atomic.fetch_or(0b1000_0001, Ordering::SeqCst);
        let after_and = 42 & 0b0011_1110;
        assert_eq!(previous, after_and);
        assert_eq!(atomic.load(Ordering::SeqCst), after_and | 0b1000_0001);

        let before_nand = atomic.load(Ordering::SeqCst);
        let previous = atomic.fetch_nand(0b1111_0000, Ordering::SeqCst);
        assert_eq!(previous, before_nand);
        assert_eq!(
            atomic.load(Ordering::SeqCst),
            !(before_nand & 0b1111_0000)
        );

        let update_previous = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            Some(current & 0b1111)
        });
        assert_eq!(update_previous, Ok(!(before_nand & 0b1111_0000)));
        assert_eq!(atomic.load(Ordering::SeqCst), (!(before_nand & 0b1111_0000)) & 0b1111);

        let rejected = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, atomic.load(Ordering::SeqCst));
            None
        });
        assert_eq!(rejected, Err(atomic.load(Ordering::SeqCst)));
    });
}

#[test]
fn atomic_usize_concurrent_decrement_then_single_fetch_update() {
    loom::model(|| {
        let counter = Arc::new(AtomicUsize::new(2));

        let left = {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                let previous = counter.fetch_sub(1, Ordering::SeqCst);
                assert!((1..=2).contains(&previous));
            })
        };

        let right = {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                let previous = counter.fetch_sub(1, Ordering::SeqCst);
                assert!((1..=2).contains(&previous));
            })
        };

        left.join().unwrap();
        right.join().unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 0);

        let marked = counter.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current == 0 {
                Some(usize::MAX)
            } else {
                None
            }
        });
        assert_eq!(marked, Ok(0));
        assert_eq!(counter.load(Ordering::SeqCst), usize::MAX);

        let previous = counter.swap(0b1111, Ordering::SeqCst);
        assert_eq!(previous, usize::MAX);
        assert_eq!(counter.load(Ordering::SeqCst), 0b1111);

        let previous = counter.fetch_and(0b0110, Ordering::SeqCst);
        assert_eq!(previous, 0b1111);
        assert_eq!(counter.load(Ordering::SeqCst), 0b0110);

        let previous = counter.fetch_or(0b1000, Ordering::SeqCst);
        assert_eq!(previous, 0b0110);
        assert_eq!(counter.load(Ordering::SeqCst), 0b1110);

        let previous = counter.fetch_nand(0b1100, Ordering::SeqCst);
        assert_eq!(previous, 0b1110);
        assert_eq!(counter.load(Ordering::SeqCst), !(0b1110 & 0b1100));
    });
}