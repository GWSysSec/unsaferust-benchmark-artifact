#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicBool, AtomicU32};
use loom::sync::Arc;
use loom::thread;

use std::sync::atomic::Ordering::{Acquire, AcqRel, Relaxed, Release, SeqCst};

#[test]
fn skip_branch_prunes_until_worker_has_published_state() {
    loom::model(|| {
        let ready = Arc::new(AtomicBool::new(false));
        let value = Arc::new(AtomicU32::new(0));

        let worker_ready = Arc::clone(&ready);
        let worker_value = Arc::clone(&value);

        let worker = thread::spawn(move || {
            assert!(!worker_ready.load(Relaxed));
            assert_eq!(worker_value.swap(7, Release), 0);
            worker_ready.store(true, Release);
        });

        thread::yield_now();

        if !ready.load(Acquire) {
            worker.join().expect("worker thread should complete successfully");
            loom::skip_branch();
            return;
        }

        assert!(ready.load(Acquire));
        assert_eq!(value.load(Acquire), 7);

        worker.join().expect("worker thread should complete successfully");

        assert!(ready.load(SeqCst));
        assert_eq!(value.load(SeqCst), 7);
    });
}

#[test]
fn stop_exploring_after_successful_multi_thread_handoff() {
    loom::model(|| {
        let state = Arc::new(AtomicU32::new(0));
        let observed_by_worker = Arc::new(AtomicBool::new(false));

        let worker_state = Arc::clone(&state);
        let worker_observed = Arc::clone(&observed_by_worker);

        let worker = thread::spawn(move || {
            loop {
                let current = worker_state.load(Acquire);

                if current == 1 {
                    assert_eq!(
                        worker_state.compare_exchange(1, 2, AcqRel, Acquire),
                        Ok(1)
                    );
                    worker_observed.store(true, Release);
                    break;
                }

                assert_eq!(current, 0);
                thread::yield_now();
            }

            assert_eq!(worker_state.load(Acquire), 2);
        });

        assert_eq!(state.compare_exchange(0, 1, AcqRel, Acquire), Ok(0));

        worker.join().expect("worker thread should observe and advance state");

        assert!(observed_by_worker.load(Acquire));
        assert_eq!(state.load(Acquire), 2);

        loom::stop_exploring();

        assert_eq!(state.fetch_add(1, AcqRel), 2);
        assert_eq!(state.load(Acquire), 3);
    });
}

#[test]
fn skip_branch_and_stop_exploring_can_be_combined_to_focus_on_winning_cas_path() {
    loom::model(|| {
        let winner = Arc::new(AtomicU32::new(0));
        let completed = Arc::new(AtomicU32::new(0));

        let left_winner = Arc::clone(&winner);
        let left_completed = Arc::clone(&completed);
        let left = thread::spawn(move || {
            if left_winner.compare_exchange(0, 1, AcqRel, Acquire).is_ok() {
                assert_eq!(left_completed.fetch_add(1, AcqRel), 0);
            }
        });

        let right_winner = Arc::clone(&winner);
        let right_completed = Arc::clone(&completed);
        let right = thread::spawn(move || {
            if right_winner.compare_exchange(0, 2, AcqRel, Acquire).is_ok() {
                assert_eq!(right_completed.fetch_add(1, AcqRel), 0);
            }
        });

        left.join().expect("left contender should not panic");
        right.join().expect("right contender should not panic");

        let chosen = winner.load(Acquire);
        if chosen != 1 {
            loom::skip_branch();
            return;
        }

        assert_eq!(chosen, 1);
        assert_eq!(completed.load(Acquire), 1);

        loom::stop_exploring();

        assert_eq!(winner.swap(3, AcqRel), 1);
        assert_eq!(winner.load(Acquire), 3);
    });
}