#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicI64, Ordering};

#[test]
fn atomic_i64_sequential_read_modify_write_workflow() {
    loom::model(|| {
        let mut atomic = AtomicI64::new(41);

        let returned = atomic.with_mut(|value| {
            assert_eq!(*value, 41);
            *value += 1;
            *value * 2
        });
        assert_eq!(returned, 84);
        assert_eq!(atomic.load(Ordering::SeqCst), 42);

        let previous = atomic.swap(0b1111, Ordering::SeqCst);
        assert_eq!(previous, 42);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1111);

        let failed = atomic.compare_exchange_weak(
            0b1110,
            100,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert_eq!(failed, Err(0b1111));
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1111);

        let previous = atomic.fetch_sub(5, Ordering::SeqCst);
        assert_eq!(previous, 0b1111);
        assert_eq!(atomic.load(Ordering::SeqCst), 10);

        let previous = atomic.fetch_and(0b0110, Ordering::SeqCst);
        assert_eq!(previous, 10);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b0010);

        let previous = atomic.fetch_or(0b1100, Ordering::SeqCst);
        assert_eq!(previous, 0b0010);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1110);

        let previous = atomic.fetch_nand(0b1011, Ordering::SeqCst);
        assert_eq!(previous, 0b1110);
        assert_eq!(atomic.load(Ordering::SeqCst), !(0b1110_i64 & 0b1011_i64));

        let before_update = atomic.load(Ordering::SeqCst);
        let updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, before_update);
            Some(current.wrapping_add(7))
        });
        assert_eq!(updated, Ok(before_update));
        assert_eq!(atomic.load(Ordering::SeqCst), before_update.wrapping_add(7));

        let current = atomic.load(Ordering::SeqCst);
        let not_updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |seen| {
            assert_eq!(seen, current);
            None
        });
        assert_eq!(not_updated, Err(current));
        assert_eq!(atomic.load(Ordering::SeqCst), current);
    });
}

#[test]
fn atomic_i64_edge_values_and_failed_update_are_stable() {
    loom::model(|| {
        let mut atomic = AtomicI64::new(i64::MIN + 10);

        let snapshot = atomic.with_mut(|value| {
            assert_eq!(*value, i64::MIN + 10);
            *value = -1;
            *value
        });
        assert_eq!(snapshot, -1);
        assert_eq!(atomic.load(Ordering::SeqCst), -1);

        let previous = atomic.swap(i64::MAX, Ordering::SeqCst);
        assert_eq!(previous, -1);
        assert_eq!(atomic.load(Ordering::SeqCst), i64::MAX);

        let failed = atomic.compare_exchange_weak(
            i64::MIN,
            0,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert_eq!(failed, Err(i64::MAX));
        assert_eq!(atomic.load(Ordering::SeqCst), i64::MAX);

        let previous = atomic.fetch_sub(i64::MAX - 3, Ordering::SeqCst);
        assert_eq!(previous, i64::MAX);
        assert_eq!(atomic.load(Ordering::SeqCst), 3);

        let previous = atomic.fetch_and(0, Ordering::SeqCst);
        assert_eq!(previous, 3);
        assert_eq!(atomic.load(Ordering::SeqCst), 0);

        let previous = atomic.fetch_or(0b1010, Ordering::SeqCst);
        assert_eq!(previous, 0);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1010);

        let previous = atomic.fetch_nand(-1, Ordering::SeqCst);
        assert_eq!(previous, 0b1010);
        assert_eq!(atomic.load(Ordering::SeqCst), !0b1010_i64);

        let observed = atomic.load(Ordering::SeqCst);
        let rejected = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, observed);
            if current >= 0 {
                Some(current + 1)
            } else {
                None
            }
        });
        assert_eq!(rejected, Err(observed));
        assert_eq!(atomic.load(Ordering::SeqCst), observed);
    });
}