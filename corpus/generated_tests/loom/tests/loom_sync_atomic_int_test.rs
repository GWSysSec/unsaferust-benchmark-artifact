#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_i32_integer_operations_chain_values_correctly() {
    loom::model(|| {
        let mut atomic = AtomicI32::new(12);

        let callback_result = atomic.with_mut(|value| {
            assert_eq!(*value, 12);
            *value = 0b1010;
            *value + 5
        });
        assert_eq!(callback_result, 15);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1010);

        let old = atomic.swap(0b1111, Ordering::SeqCst);
        assert_eq!(old, 0b1010);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1111);

        let failed = atomic.compare_exchange_weak(
            0b1110,
            99,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert_eq!(failed, Err(0b1111));
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1111);

        let old = atomic.fetch_sub(3, Ordering::SeqCst);
        assert_eq!(old, 0b1111);
        assert_eq!(atomic.load(Ordering::SeqCst), 12);

        let old = atomic.fetch_and(0b1010, Ordering::SeqCst);
        assert_eq!(old, 12);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1000);

        let old = atomic.fetch_or(0b0011, Ordering::SeqCst);
        assert_eq!(old, 0b1000);
        assert_eq!(atomic.load(Ordering::SeqCst), 0b1011);

        let before_nand = atomic.load(Ordering::SeqCst);
        let old = atomic.fetch_nand(0b0110, Ordering::SeqCst);
        assert_eq!(old, before_nand);
        assert_eq!(atomic.load(Ordering::SeqCst), !(before_nand & 0b0110));

        atomic.store(40, Ordering::SeqCst);
        let updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current >= 40 {
                Some(current - 10)
            } else {
                None
            }
        });
        assert_eq!(updated, Ok(40));
        assert_eq!(atomic.load(Ordering::SeqCst), 30);

        let not_updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            if current < 10 {
                Some(current + 1)
            } else {
                None
            }
        });
        assert_eq!(not_updated, Err(30));
        assert_eq!(atomic.load(Ordering::SeqCst), 30);
    });
}

#[test]
fn atomic_u32_concurrent_fetch_update_counts_exactly_once_per_thread() {
    loom::model(|| {
        let counter = Arc::new(AtomicU32::new(0));
        let flags = Arc::new(AtomicU32::new(0));

        let counter_a = Arc::clone(&counter);
        let flags_a = Arc::clone(&flags);
        let thread_a = thread::spawn(move || {
            let old_counter = counter_a.fetch_update(
                Ordering::SeqCst,
                Ordering::SeqCst,
                |value| Some(value + 1),
            );
            assert!(old_counter.is_ok());

            let previous_flags = flags_a.fetch_or(0b0001, Ordering::SeqCst);
            assert_eq!(previous_flags & 0b0001, 0);
        });

        let counter_b = Arc::clone(&counter);
        let flags_b = Arc::clone(&flags);
        let thread_b = thread::spawn(move || {
            let old_counter = counter_b.fetch_update(
                Ordering::SeqCst,
                Ordering::SeqCst,
                |value| Some(value + 1),
            );
            assert!(old_counter.is_ok());

            let previous_flags = flags_b.fetch_or(0b0010, Ordering::SeqCst);
            assert_eq!(previous_flags & 0b0010, 0);
        });

        thread_a.join().expect("thread A should not panic");
        thread_b.join().expect("thread B should not panic");

        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(flags.load(Ordering::SeqCst), 0b0011);

        let previous = flags.fetch_and(0b0001, Ordering::SeqCst);
        assert_eq!(previous, 0b0011);
        assert_eq!(flags.load(Ordering::SeqCst), 0b0001);

        let previous = counter.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(previous, 2);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn atomic_u32_compare_exchange_weak_can_claim_single_winner() {
    loom::model(|| {
        let winner = Arc::new(AtomicU32::new(0));
        let attempts = Arc::new(AtomicU32::new(0));

        let winner_a = Arc::clone(&winner);
        let attempts_a = Arc::clone(&attempts);
        let thread_a = thread::spawn(move || {
            attempts_a.fetch_or(0b0001, Ordering::SeqCst);
            let claimed = winner_a.compare_exchange_weak(
                0,
                1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
            claimed.is_ok()
        });

        let winner_b = Arc::clone(&winner);
        let attempts_b = Arc::clone(&attempts);
        let thread_b = thread::spawn(move || {
            attempts_b.fetch_or(0b0010, Ordering::SeqCst);
            let claimed = winner_b.compare_exchange_weak(
                0,
                2,
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
            claimed.is_ok()
        });

        let a_won = thread_a.join().expect("thread A should not panic");
        let b_won = thread_b.join().expect("thread B should not panic");

        assert!(a_won || b_won);
        assert!(!(a_won && b_won));
        assert_eq!(attempts.load(Ordering::SeqCst), 0b0011);

        let final_winner = winner.load(Ordering::SeqCst);
        assert!(final_winner == 1 || final_winner == 2);

        if a_won {
            assert_eq!(final_winner, 1);
        }
        if b_won {
            assert_eq!(final_winner, 2);
        }

        let previous = attempts.fetch_nand(0b0011, Ordering::SeqCst);
        assert_eq!(previous, 0b0011);
        assert_eq!(attempts.load(Ordering::SeqCst), !0b0011_u32);
    });
}