#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicI32, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_i32_sequential_read_modify_write_workflow() {
    loom::model(|| {
        let mut atomic = AtomicI32::new(10);

        let derived = atomic.with_mut(|value| {
            assert_eq!(*value, 10);
            *value = 0b1100;
            *value * 2
        });
        assert_eq!(derived, 0b11000);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1100);

        let previous = atomic.swap(25, Ordering::SeqCst);
        assert_eq!(previous, 0b1100);
        assert_eq!(atomic.load(Ordering::SeqCst), 25);

        let failed = atomic.compare_exchange_weak(24, 100, Ordering::SeqCst, Ordering::SeqCst);
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

        atomic.store(40, Ordering::SeqCst);

        let updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, 40);
            Some(current - 7)
        });
        assert_eq!(updated, Ok(40));
        assert_eq!(atomic.load(Ordering::SeqCst), 33);

        let not_updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, 33);
            None
        });
        assert_eq!(not_updated, Err(33));
        assert_eq!(atomic.load(Ordering::SeqCst), 33);
    });
}

#[test]
fn atomic_i32_concurrent_claim_then_accounting_workflow() {
    loom::model(|| {
        let owner = Arc::new(AtomicI32::new(0));
        let balance = Arc::new(AtomicI32::new(100));

        let first = {
            let owner = Arc::clone(&owner);
            let balance = Arc::clone(&balance);
            thread::spawn(move || loop {
                match owner.compare_exchange_weak(0, 1, Ordering::SeqCst, Ordering::SeqCst) {
                    Ok(previous) => {
                        assert_eq!(previous, 0);
                        let old_balance = balance.fetch_sub(30, Ordering::SeqCst);
                        assert!(old_balance == 100 || old_balance == 70);
                        break true;
                    }
                    Err(observed) if observed != 0 => {
                        assert!(observed == 1 || observed == 2);
                        break false;
                    }
                    Err(0) => thread::yield_now(),
                    Err(other) => panic!("unexpected owner value: {}", other),
                }
            })
        };

        let second = {
            let owner = Arc::clone(&owner);
            let balance = Arc::clone(&balance);
            thread::spawn(move || loop {
                match owner.compare_exchange_weak(0, 2, Ordering::SeqCst, Ordering::SeqCst) {
                    Ok(previous) => {
                        assert_eq!(previous, 0);
                        let old_balance = balance.fetch_sub(30, Ordering::SeqCst);
                        assert!(old_balance == 100 || old_balance == 70);
                        break true;
                    }
                    Err(observed) if observed != 0 => {
                        assert!(observed == 1 || observed == 2);
                        break false;
                    }
                    Err(0) => thread::yield_now(),
                    Err(other) => panic!("unexpected owner value: {}", other),
                }
            })
        };

        let first_claimed = first.join().expect("first claimant thread panicked");
        let second_claimed = second.join().expect("second claimant thread panicked");

        assert_ne!(first_claimed, second_claimed);

        let winning_owner = owner.load(Ordering::SeqCst);
        assert!(winning_owner == 1 || winning_owner == 2);
        assert_eq!(balance.load(Ordering::SeqCst), 70);

        let previous = balance.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current == 70 {
                Some(current + winning_owner)
            } else {
                None
            }
        });
        assert_eq!(previous, Ok(70));
        assert_eq!(balance.load(Ordering::SeqCst), 70 + winning_owner);
    });
}

#[test]
fn atomic_i32_bitwise_edges_and_with_mut_return_value() {
    loom::model(|| {
        let mut atomic = AtomicI32::new(-1);

        let old_inner = atomic.with_mut(|value| {
            let old = *value;
            *value = 0;
            old
        });
        assert_eq!(old_inner, -1);
        assert_eq!(atomic.load(Ordering::SeqCst), 0);

        let previous = atomic.fetch_and(0b1010, Ordering::SeqCst);
        assert_eq!(previous, 0);
        assert_eq!(atomic.load(Ordering::SeqCst), 0);

        let previous = atomic.fetch_or(0b1010, Ordering::SeqCst);
        assert_eq!(previous, 0);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1010);

        let previous = atomic.fetch_nand(0, Ordering::SeqCst);
        assert_eq!(previous, 0b1010);
        assert_eq!(atomic.load(Ordering::SeqCst), !0);

        let previous = atomic.swap(i32::MIN, Ordering::SeqCst);
        assert_eq!(previous, !0);
        assert_eq!(atomic.load(Ordering::SeqCst), i32::MIN);

        let previous = atomic.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(previous, i32::MIN);
        assert_eq!(atomic.load(Ordering::SeqCst), i32::MAX);
    });
}