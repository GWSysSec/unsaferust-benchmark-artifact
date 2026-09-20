use loom::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_bool_read_modify_write_workflow() {
    loom::model(|| {
        let flag = AtomicBool::new(false);

        let previous = flag.swap(true, Ordering::SeqCst);
        assert!(!previous);
        assert!(flag.load(Ordering::SeqCst));

        let previous = flag.fetch_and(false, Ordering::SeqCst);
        assert!(previous);
        assert!(!flag.load(Ordering::SeqCst));

        let previous = flag.fetch_or(true, Ordering::SeqCst);
        assert!(!previous);
        assert!(flag.load(Ordering::SeqCst));

        let previous = flag.fetch_nand(true, Ordering::SeqCst);
        assert!(previous);
        assert!(!flag.load(Ordering::SeqCst));

        let previous = flag.fetch_nand(false, Ordering::SeqCst);
        assert!(!previous);
        assert!(flag.load(Ordering::SeqCst));

        let updated = flag.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert!(current);
            Some(false)
        });
        assert_eq!(updated, Ok(true));
        assert!(!flag.load(Ordering::SeqCst));

        let not_updated = flag.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert!(!current);
            None
        });
        assert_eq!(not_updated, Err(false));
        assert!(!flag.load(Ordering::SeqCst));
    });
}

#[test]
fn compare_exchange_weak_claims_flag_once_under_all_interleavings() {
    loom::model(|| {
        let claimed = Arc::new(AtomicBool::new(false));
        let winners = Arc::new(AtomicU32::new(0));

        let claimed_a = Arc::clone(&claimed);
        let winners_a = Arc::clone(&winners);
        let first = thread::spawn(move || {
            let mut observed_failure = None;

            loop {
                match claimed_a.compare_exchange_weak(
                    false,
                    true,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(previous) => {
                        assert!(!previous);
                        let old_winners = winners_a.fetch_add(1, Ordering::AcqRel);
                        assert_eq!(old_winners, 0);
                        return true;
                    }
                    Err(actual) => {
                        observed_failure = Some(actual);
                        if actual {
                            return false;
                        }
                        thread::yield_now();
                    }
                }

                if let Some(false) = observed_failure {
                    assert!(!claimed_a.load(Ordering::Acquire) || winners_a.load(Ordering::Acquire) <= 1);
                }
            }
        });

        let claimed_b = Arc::clone(&claimed);
        let winners_b = Arc::clone(&winners);
        let second = thread::spawn(move || {
            loop {
                match claimed_b.compare_exchange_weak(
                    false,
                    true,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(previous) => {
                        assert!(!previous);
                        let old_winners = winners_b.fetch_add(1, Ordering::AcqRel);
                        assert_eq!(old_winners, 0);
                        return true;
                    }
                    Err(actual) => {
                        if actual {
                            return false;
                        }
                        thread::yield_now();
                    }
                }
            }
        });

        let first_won = first.join().unwrap();
        let second_won = second.join().unwrap();

        assert_ne!(first_won, second_won);
        assert_eq!(winners.load(Ordering::Acquire), 1);
        assert!(claimed.load(Ordering::Acquire));

        let released_previous = claimed.swap(false, Ordering::AcqRel);
        assert!(released_previous);
        assert!(!claimed.load(Ordering::Acquire));
    });
}

#[test]
fn fetch_update_can_conditionally_publish_and_then_clear() {
    loom::model(|| {
        let ready = Arc::new(AtomicBool::new(false));
        let attempts = Arc::new(AtomicU32::new(0));

        let producer_ready = Arc::clone(&ready);
        let producer_attempts = Arc::clone(&attempts);
        let producer = thread::spawn(move || {
            let result = producer_ready.fetch_update(
                Ordering::AcqRel,
                Ordering::Acquire,
                |current| {
                    producer_attempts.fetch_add(1, Ordering::AcqRel);
                    if current {
                        None
                    } else {
                        Some(true)
                    }
                },
            );

            assert!(result == Ok(false) || result == Err(true));
            result.is_ok()
        });

        let consumer_ready = Arc::clone(&ready);
        let consumer = thread::spawn(move || {
            loop {
                if consumer_ready.load(Ordering::Acquire) {
                    let was_ready = consumer_ready.fetch_and(false, Ordering::AcqRel);
                    if was_ready {
                        return true;
                    }
                } else {
                    thread::yield_now();
                    if consumer_ready.fetch_or(false, Ordering::AcqRel) {
                        continue;
                    }
                }
            }
        });

        let producer_published = producer.join().unwrap();
        let consumer_observed = consumer.join().unwrap();

        assert!(producer_published);
        assert!(consumer_observed);
        assert!(attempts.load(Ordering::Acquire) >= 1);
        assert!(!ready.load(Ordering::Acquire));

        let clear_again = ready.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            if current {
                Some(false)
            } else {
                None
            }
        });
        assert_eq!(clear_again, Err(false));
    });
}