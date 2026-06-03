#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicBool, AtomicU32};
use loom::thread;

use std::cell::RefCell;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::Arc;

loom::lazy_static! {
    static ref LAZY_NUMBER: usize = {
        loom::thread::yield_now();
        41
    };

    static ref LAZY_LABEL: String = String::from("loom-lazy");
}

loom::thread_local! {
    static THREAD_LOCAL_COUNTER: RefCell<usize> = RefCell::new(0);
}

fn exercise_thread_local(expected_initial: usize, new_value: usize) {
    THREAD_LOCAL_COUNTER.with(|counter| {
        assert_eq!(*counter.borrow(), expected_initial);
    });

    THREAD_LOCAL_COUNTER.with(|counter| {
        *counter.borrow_mut() = new_value;
        assert_eq!(*counter.borrow(), new_value);
    });

    THREAD_LOCAL_COUNTER.with(|counter| {
        assert_eq!(*counter.borrow(), new_value);
    });
}

#[test]
fn model_exercises_lazy_static_thread_local_and_pruning_controls() {
    loom::model(|| {
        assert_eq!(*LAZY_NUMBER, 41);
        assert_eq!(LAZY_LABEL.as_str(), "loom-lazy");

        exercise_thread_local(0, 10);

        let ready = Arc::new(AtomicBool::new(false));
        let total = Arc::new(AtomicU32::new(0));

        let worker_ready = Arc::clone(&ready);
        let worker_total = Arc::clone(&total);
        let worker = thread::spawn(move || {
            assert_eq!(*LAZY_NUMBER + 1, 42);
            assert_eq!(LAZY_LABEL.len(), 9);

            exercise_thread_local(0, 20);

            let previous = worker_total.fetch_add(2, Relaxed);
            assert!(previous == 0 || previous == 1);

            worker_ready.store(true, Release);
        });

        let observer_ready = Arc::clone(&ready);
        let observer_total = Arc::clone(&total);
        let observer = thread::spawn(move || {
            exercise_thread_local(0, 30);

            if !observer_ready.load(Acquire) {
                loom::skip_branch();
                return;
            }

            let observed = observer_total.fetch_add(1, Relaxed);
            assert!(observed >= 2);
        });

        let previous = total.fetch_add(1, Relaxed);
        assert!(previous <= 3);

        worker.join().unwrap();
        observer.join().unwrap();

        let final_total = total.load(Relaxed);
        assert!(final_total == 3 || final_total == 4);
        assert!(ready.load(Acquire));

        THREAD_LOCAL_COUNTER.with(|counter| {
            assert_eq!(*counter.borrow(), 10);
        });

        loom::stop_exploring();

        assert_eq!(*LAZY_NUMBER, 41);
        assert_eq!(LAZY_LABEL.as_bytes()[0], b'l');
    });
}

#[test]
fn stop_exploring_after_successful_cross_thread_handshake() {
    loom::model(|| {
        let flag = Arc::new(AtomicBool::new(false));
        let value = Arc::new(AtomicU32::new(0));

        let writer_flag = Arc::clone(&flag);
        let writer_value = Arc::clone(&value);
        let writer = thread::spawn(move || {
            writer_value.store(7, Relaxed);
            writer_flag.store(true, Release);
        });

        let reader_flag = Arc::clone(&flag);
        let reader_value = Arc::clone(&value);
        let reader = thread::spawn(move || {
            if !reader_flag.load(Acquire) {
                loom::skip_branch();
                return;
            }

            assert_eq!(reader_value.load(Relaxed), 7);
        });

        writer.join().unwrap();
        reader.join().unwrap();

        assert!(flag.load(Acquire));
        assert_eq!(value.load(Relaxed), 7);

        loom::stop_exploring();

        assert_eq!(value.fetch_add(1, Relaxed), 7);
        assert_eq!(value.load(Relaxed), 8);
    });
}