use loom::sync::{Arc, Mutex};
use loom::thread;

#[test]
fn try_lock_reports_busy_while_guard_is_held_and_succeeds_after_drop() {
    loom::model(|| {
        let values = Mutex::new(Vec::<i32>::new());

        {
            let mut guard = values.try_lock().expect("first try_lock should acquire mutex");
            guard.push(10);
            guard.push(20);

            assert_eq!(guard.len(), 2);
            assert_eq!(guard[0], 10);
            assert!(
                values.try_lock().is_err(),
                "try_lock must not acquire a mutex while it is already locked"
            );
        }

        {
            let mut guard = values
                .try_lock()
                .expect("try_lock should acquire mutex after previous guard is dropped");
            assert_eq!(guard.as_slice(), &[10, 20]);

            let previous = std::mem::replace(&mut guard[1], 30);
            assert_eq!(previous, 20);
            assert_eq!(guard.as_slice(), &[10, 30]);
        }

        let guard = values
            .try_lock()
            .expect("try_lock should still work after mutation");
        assert_eq!(guard.iter().sum::<i32>(), 40);
    });
}

#[test]
fn try_lock_can_be_used_to_coordinate_concurrent_updates() {
    loom::model(|| {
        let counter = Arc::new(Mutex::new(0usize));

        let first_counter = Arc::clone(&counter);
        let first = thread::spawn(move || {
            loop {
                match first_counter.try_lock() {
                    Ok(mut guard) => {
                        let before = *guard;
                        *guard = before + 1;

                        assert_eq!(*guard, before + 1);
                        assert!(
                            first_counter.try_lock().is_err(),
                            "same mutex should be unavailable while guard is live"
                        );
                        break;
                    }
                    Err(_) => thread::yield_now(),
                }
            }
        });

        let second_counter = Arc::clone(&counter);
        let second = thread::spawn(move || {
            loop {
                match second_counter.try_lock() {
                    Ok(mut guard) => {
                        let before = *guard;
                        *guard = before + 1;

                        assert_eq!(*guard, before + 1);
                        assert!(*guard <= 2);
                        break;
                    }
                    Err(_) => thread::yield_now(),
                }
            }
        });

        first.join().expect("first worker should not panic");
        second.join().expect("second worker should not panic");

        let guard = counter
            .try_lock()
            .expect("main thread should acquire mutex after workers finish");
        assert_eq!(*guard, 2);
    });
}

#[test]
fn try_lock_preserves_nested_state_across_multiple_lock_attempts() {
    loom::model(|| {
        let state = Arc::new(Mutex::new((false, Vec::<&'static str>::new())));

        {
            let mut guard = state
                .try_lock()
                .expect("initial try_lock should acquire state mutex");
            assert!(!guard.0);
            assert!(guard.1.is_empty());

            guard.0 = true;
            guard.1.push("initialized");

            assert!(
                state.try_lock().is_err(),
                "mutex should reject a second try_lock while state is being initialized"
            );
        }

        let worker_state = Arc::clone(&state);
        let worker = thread::spawn(move || {
            loop {
                match worker_state.try_lock() {
                    Ok(mut guard) => {
                        assert!(guard.0);
                        assert_eq!(guard.1.as_slice(), &["initialized"]);

                        guard.1.push("worker");
                        assert_eq!(guard.1.len(), 2);
                        break;
                    }
                    Err(_) => thread::yield_now(),
                }
            }
        });

        worker.join().expect("worker should not panic");

        let guard = state
            .try_lock()
            .expect("final try_lock should observe worker changes");
        assert!(guard.0);
        assert_eq!(guard.1.as_slice(), &["initialized", "worker"]);
    });
}