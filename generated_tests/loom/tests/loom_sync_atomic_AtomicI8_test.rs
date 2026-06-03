#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicI8, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_i8_sequential_read_modify_write_workflow() {
    loom::model(|| {
        let mut atomic = AtomicI8::new(7);

        let returned_from_with_mut = atomic.with_mut(|value| {
            assert_eq!(*value, 7);
            *value = 0b0101;
            *value + 2
        });
        assert_eq!(returned_from_with_mut, 0b0111);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b0101);

        let previous = atomic.swap(20, Ordering::SeqCst);
        assert_eq!(previous, 0b0101);
        assert_eq!(atomic.load(Ordering::SeqCst), 20);

        let failed = atomic.compare_exchange_weak(19, 99, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(20));
        assert_eq!(atomic.load(Ordering::SeqCst), 20);

        loop {
            match atomic.compare_exchange_weak(20, 0b1111, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(previous) => {
                    assert_eq!(previous, 20);
                    break;
                }
                Err(observed) => {
                    assert_eq!(observed, 20);
                    thread::yield_now();
                }
            }
        }
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

        let previous = atomic.fetch_nand(0b0011, Ordering::SeqCst);
        assert_eq!(previous, 0b1110);
        assert_eq!(atomic.load(Ordering::SeqCst), !(0b1110 & 0b0011));

        let previous = atomic.swap(40, Ordering::SeqCst);
        assert_eq!(previous, -3);
        assert_eq!(atomic.load(Ordering::SeqCst), 40);

        let updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, 40);
            Some(current - 10)
        });
        assert_eq!(updated, Ok(40));
        assert_eq!(atomic.load(Ordering::SeqCst), 30);

        let not_updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, 30);
            None
        });
        assert_eq!(not_updated, Err(30));
        assert_eq!(atomic.load(Ordering::SeqCst), 30);
    });
}

#[test]
fn atomic_i8_concurrent_fetch_sub_and_update_workflow() {
    loom::model(|| {
        let atomic = Arc::new(AtomicI8::new(10));

        let first = {
            let atomic = Arc::clone(&atomic);
            thread::spawn(move || {
                let previous = atomic.fetch_sub(1, Ordering::SeqCst);
                assert!(previous == 10 || previous == 8);
                previous
            })
        };

        let second = {
            let atomic = Arc::clone(&atomic);
            thread::spawn(move || {
                let previous = atomic.fetch_sub(2, Ordering::SeqCst);
                assert!(previous == 10 || previous == 9);
                previous
            })
        };

        let first_previous = first.join().unwrap();
        let second_previous = second.join().unwrap();

        assert_ne!(first_previous, second_previous);
        assert_eq!(atomic.load(Ordering::SeqCst), 7);

        let marked = atomic.fetch_or(0b1000, Ordering::SeqCst);
        assert_eq!(marked, 7);
        assert_eq!(atomic.load(Ordering::SeqCst), 15);

        let masked = atomic.fetch_and(0b1011, Ordering::SeqCst);
        assert_eq!(masked, 15);
        assert_eq!(atomic.load(Ordering::SeqCst), 11);

        let capped = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current > 8 {
                Some(8)
            } else {
                None
            }
        });
        assert_eq!(capped, Ok(11));
        assert_eq!(atomic.load(Ordering::SeqCst), 8);
    });
}