#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicI16, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_i16_sequential_read_modify_write_workflow() {
    loom::model(|| {
        let mut atomic = AtomicI16::new(0);

        let returned_from_with_mut = atomic.with_mut(|value| {
            assert_eq!(*value, 0);
            *value = 0b1100;
            *value + 1
        });
        assert_eq!(returned_from_with_mut, 0b1101);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1100);

        let previous = atomic.swap(20, Ordering::SeqCst);
        assert_eq!(previous, 0b1100);
        assert_eq!(atomic.load(Ordering::SeqCst), 20);

        let failed = atomic.compare_exchange_weak(19, 30, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed, Err(20));
        assert_eq!(atomic.load(Ordering::SeqCst), 20);

        let previous = atomic.fetch_sub(5, Ordering::SeqCst);
        assert_eq!(previous, 20);
        assert_eq!(atomic.load(Ordering::SeqCst), 15);

        let previous = atomic.fetch_and(0b1010, Ordering::SeqCst);
        assert_eq!(previous, 15);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1010);

        let previous = atomic.fetch_or(0b0101, Ordering::SeqCst);
        assert_eq!(previous, 0b1010);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1111);

        let previous = atomic.fetch_nand(0b0011, Ordering::SeqCst);
        assert_eq!(previous, 0b1111);
        assert_eq!(atomic.load(Ordering::SeqCst), !(0b1111_i16 & 0b0011_i16));

        atomic.store(7, Ordering::SeqCst);
        let updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, 7);
            Some(current * 3)
        });
        assert_eq!(updated, Ok(7));
        assert_eq!(atomic.load(Ordering::SeqCst), 21);

        let not_updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, 21);
            None
        });
        assert_eq!(not_updated, Err(21));
        assert_eq!(atomic.load(Ordering::SeqCst), 21);
    });
}

#[test]
fn atomic_i16_concurrent_claim_and_accounting_workflow() {
    loom::model(|| {
        let state = Arc::new(AtomicI16::new(0));

        let producer = {
            let state = Arc::clone(&state);
            thread::spawn(move || {
                let replaced = state.swap(10, Ordering::SeqCst);
                assert!(
                    replaced == 0 || replaced == -2,
                    "producer can replace either the initial state or a pre-publication debit"
                );

                let old = state.fetch_or(0b0100, Ordering::SeqCst);
                assert!(
                    old == 10 || old == 8,
                    "producer should set the published bit after installing the base value"
                );
            })
        };

        let consumer = {
            let state = Arc::clone(&state);
            thread::spawn(move || {
                let observed = state.fetch_sub(2, Ordering::SeqCst);
                assert!(
                    observed == 0 || observed == 10 || observed == 14,
                    "consumer can debit before publication, after publication, or after the flag is set"
                );

                let repaired = state.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    if current < 0 {
                        Some(0)
                    } else {
                        None
                    }
                });

                match repaired {
                    Ok(previous) => assert!(
                        previous < 0,
                        "a successful repair must have replaced a negative pre-publication debit"
                    ),
                    Err(current) => assert!(
                        current >= 0,
                        "repair should be skipped once the state is already non-negative"
                    ),
                }
            })
        };

        producer.join().expect("producer thread panicked");
        consumer.join().expect("consumer thread panicked");

        let final_value = state.load(Ordering::SeqCst);
        assert!(
            final_value == 12 || final_value == 14,
            "final state should be published, flagged, and include exactly one debit if it happened after publication"
        );
        assert_eq!(final_value & 0b0100, 0b0100);
    });
}

#[test]
fn atomic_i16_compare_exchange_weak_can_claim_exact_state() {
    loom::model(|| {
        let value = Arc::new(AtomicI16::new(3));

        let claimer = {
            let value = Arc::clone(&value);
            thread::spawn(move || {
                loop {
                    let current = value.load(Ordering::SeqCst);
                    if current != 3 {
                        break false;
                    }

                    match value.compare_exchange_weak(3, 9, Ordering::SeqCst, Ordering::SeqCst) {
                        Ok(previous) => {
                            assert_eq!(previous, 3);
                            break true;
                        }
                        Err(actual) => {
                            assert!(actual == 3 || actual == 9);
                            thread::yield_now();
                        }
                    }
                }
            })
        };

        let observer = {
            let value = Arc::clone(&value);
            thread::spawn(move || {
                let before = value.fetch_and(0b1111, Ordering::SeqCst);
                assert!(before == 3 || before == 9);
            })
        };

        let claimed = claimer.join().expect("claimer thread panicked");
        observer.join().expect("observer thread panicked");

        let final_value = value.load(Ordering::SeqCst);
        assert!(final_value == 3 || final_value == 9);
        if claimed {
            assert_eq!(final_value, 9);
        }
    });
}