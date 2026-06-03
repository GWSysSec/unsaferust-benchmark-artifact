use loom::sync::atomic::{self, AtomicBool, AtomicU32, Ordering};
use loom::thread;
use std::sync::Arc;

#[test]
fn spin_loop_hint_can_be_used_in_busy_wait_until_release_store_is_observed() {
    loom::model(|| {
        let ready = Arc::new(AtomicBool::new(false));
        let data = Arc::new(AtomicU32::new(0));
        let observed_spins = Arc::new(AtomicU32::new(0));

        let writer_ready = Arc::clone(&ready);
        let writer_data = Arc::clone(&data);
        let writer = thread::spawn(move || {
            assert_eq!(writer_data.load(Ordering::Relaxed), 0);

            writer_data.store(41, Ordering::Relaxed);
            atomic::fence(Ordering::Release);
            writer_ready.store(true, Ordering::Release);

            assert!(writer_ready.load(Ordering::Relaxed));
            41_u32
        });

        let reader_ready = Arc::clone(&ready);
        let reader_data = Arc::clone(&data);
        let reader_spins = Arc::clone(&observed_spins);
        let reader = thread::spawn(move || {
            while !reader_ready.load(Ordering::Acquire) {
                let previous = reader_spins.fetch_add(1, Ordering::Relaxed);
                assert!(previous < 10_000);

                atomic::spin_loop_hint();
                thread::yield_now();
            }

            atomic::fence(Ordering::Acquire);

            let value = reader_data.load(Ordering::Relaxed);
            assert_eq!(value, 41);
            assert!(reader_ready.load(Ordering::Relaxed));

            value + 1
        });

        let written = writer.join().expect("writer thread should not panic");
        let read = reader.join().expect("reader thread should not panic");

        assert_eq!(written, 41);
        assert_eq!(read, 42);
        assert_eq!(data.load(Ordering::Relaxed), 41);
        assert!(ready.load(Ordering::Relaxed));
    });
}

#[test]
fn spin_loop_hint_fits_in_compare_exchange_retry_workflow() {
    loom::model(|| {
        let claimed = Arc::new(AtomicBool::new(false));
        let successes = Arc::new(AtomicU32::new(0));
        let attempts = Arc::new(AtomicU32::new(0));

        let mut handles = Vec::new();

        for _ in 0..2 {
            let claimed = Arc::clone(&claimed);
            let successes = Arc::clone(&successes);
            let attempts = Arc::clone(&attempts);

            handles.push(thread::spawn(move || {
                loop {
                    attempts.fetch_add(1, Ordering::Relaxed);

                    match claimed.compare_exchange(
                        false,
                        true,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(previous) => {
                            assert!(!previous);
                            let old_successes = successes.fetch_add(1, Ordering::Relaxed);
                            assert_eq!(old_successes, 0);
                            return true;
                        }
                        Err(current) => {
                            assert!(current);
                            atomic::spin_loop_hint();
                            return false;
                        }
                    }
                }
            }));
        }

        let first = handles.remove(0).join().expect("first contender should not panic");
        let second = handles.remove(0).join().expect("second contender should not panic");

        assert_ne!(first, second);
        assert!(claimed.load(Ordering::Acquire));
        assert_eq!(successes.load(Ordering::Relaxed), 1);
        assert_eq!(attempts.load(Ordering::Relaxed), 2);
    });
}