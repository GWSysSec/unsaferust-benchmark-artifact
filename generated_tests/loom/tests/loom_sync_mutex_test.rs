use loom::sync::{Arc, Mutex};
use loom::thread;

#[test]
fn try_lock_fails_while_locked_then_observes_guard_mutations() {
    loom::model(|| {
        let values = Mutex::new(Vec::<i32>::new());

        {
            let mut guard = values
                .try_lock()
                .expect("initial try_lock should acquire the mutex");
            guard.push(10);
            guard.push(20);

            assert_eq!(guard.len(), 2);
            assert_eq!(guard.as_slice(), &[10, 20]);
            assert!(
                values.try_lock().is_err(),
                "try_lock must report that the mutex is busy while a guard is held"
            );

            guard[1] = 30;
            assert_eq!(guard.iter().copied().sum::<i32>(), 40);
        }

        {
            let mut guard = values
                .try_lock()
                .expect("try_lock should succeed after the previous guard is dropped");
            assert_eq!(guard.as_slice(), &[10, 30]);

            let old = std::mem::replace(&mut guard[0], 5);
            assert_eq!(old, 10);
            assert_eq!(guard.as_slice(), &[5, 30]);
        }

        let guard = values
            .try_lock()
            .expect("try_lock should continue to work after multiple lock/unlock cycles");
        assert_eq!(guard.len(), 2);
        assert_eq!(guard.iter().copied().sum::<i32>(), 35);
    });
}

#[test]
fn try_lock_can_be_used_for_nonblocking_concurrent_updates() {
    loom::model(|| {
        let shared = Arc::new(Mutex::new(Vec::<&'static str>::new()));

        {
            let mut guard = shared
                .try_lock()
                .expect("main thread should acquire the mutex before spawning");
            guard.push("main-start");
            assert_eq!(guard.as_slice(), &["main-start"]);

            let cloned = Arc::clone(&shared);
            let handle = thread::spawn(move || {
                assert!(
                    cloned.try_lock().is_err(),
                    "spawned thread should see the mutex as busy while main holds it"
                );
            });

            handle.join().expect("spawned thread should not panic");
            guard.push("main-end");
            assert_eq!(guard.as_slice(), &["main-start", "main-end"]);
        }

        let first_worker_shared = Arc::clone(&shared);
        let first = thread::spawn(move || {
            loop {
                match first_worker_shared.try_lock() {
                    Ok(mut guard) => {
                        guard.push("worker-a");
                        assert!(guard.contains(&"main-start"));
                        assert!(guard.contains(&"main-end"));
                        break;
                    }
                    Err(_) => thread::yield_now(),
                }
            }
        });

        let second_worker_shared = Arc::clone(&shared);
        let second = thread::spawn(move || {
            loop {
                match second_worker_shared.try_lock() {
                    Ok(mut guard) => {
                        guard.push("worker-b");
                        assert!(guard.contains(&"main-start"));
                        assert!(guard.contains(&"main-end"));
                        break;
                    }
                    Err(_) => thread::yield_now(),
                }
            }
        });

        first.join().expect("first worker should complete");
        second.join().expect("second worker should complete");

        let guard = shared
            .try_lock()
            .expect("mutex should be available after both workers join");
        assert_eq!(guard.len(), 4);
        assert!(guard.contains(&"main-start"));
        assert!(guard.contains(&"main-end"));
        assert!(guard.contains(&"worker-a"));
        assert!(guard.contains(&"worker-b"));
    });
}

#[test]
fn try_lock_supports_repeated_take_modify_replace_workflow() {
    loom::model(|| {
        let state = Mutex::new(Some(String::from("initial")));

        {
            let mut guard = state
                .try_lock()
                .expect("try_lock should acquire mutex containing an Option");
            let taken = guard.take();
            assert_eq!(taken.as_deref(), Some("initial"));
            assert!(guard.is_none());
            assert!(
                state.try_lock().is_err(),
                "nested try_lock should fail while the Option state is being modified"
            );

            *guard = taken.map(|mut value| {
                value.push_str("-updated");
                value
            });
            assert_eq!(guard.as_deref(), Some("initial-updated"));
        }

        {
            let mut guard = state
                .try_lock()
                .expect("try_lock should reacquire mutex after Option update");
            let previous = guard.replace(String::from("final"));
            assert_eq!(previous.as_deref(), Some("initial-updated"));
            assert_eq!(guard.as_deref(), Some("final"));
        }

        let guard = state
            .try_lock()
            .expect("try_lock should read the final Option value");
        assert_eq!(guard.as_deref(), Some("final"));
    });
}