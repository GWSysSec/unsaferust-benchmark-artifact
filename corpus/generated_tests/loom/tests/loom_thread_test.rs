use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn current_reports_current_thread_context() {
    loom::model(|| {
        let main_observations = {
            let current = thread::current();
            (
                std::mem::size_of_val(&current),
                format!("{:?}", current.clone()),
                format!("{:?}", current),
            )
        };

        assert_eq!(
            main_observations.0,
            std::mem::size_of::<loom::thread::Thread>()
        );
        assert_eq!(main_observations.1, main_observations.2);

        let main_try_observations: Result<_, ()> = Ok({
            let current = thread::current();
            (format!("{:?}", current.clone()), format!("{:?}", current))
        });

        let main_try_observations =
            main_try_observations.expect("current thread should be available inside a loom model");

        assert_eq!(main_try_observations.0, main_try_observations.1);

        let visits = Arc::new(AtomicUsize::new(0));
        let spawned_visits = Arc::clone(&visits);

        let handle = thread::spawn(move || {
            let spawned_with = {
                spawned_visits.fetch_add(1, Ordering::SeqCst);
                let current = thread::current();
                (format!("{:?}", current.clone()), format!("{:?}", current))
            };

            assert_eq!(spawned_with.0, spawned_with.1);

            let spawned_try_with: Result<_, ()> = Ok({
                spawned_visits.fetch_add(1, Ordering::SeqCst);
                let current = thread::current();
                (format!("{:?}", current.clone()), format!("{:?}", current))
            });

            let spawned_try_with =
                spawned_try_with.expect("current thread should be available in a spawned loom thread");

            assert_eq!(spawned_try_with.0, spawned_try_with.1);
            assert_eq!(spawned_visits.load(Ordering::SeqCst), 2);
        });

        handle.join().expect("spawned thread should finish successfully");
        assert_eq!(visits.load(Ordering::SeqCst), 2);
    });
}

#[test]
fn current_thread_context_can_drive_a_multi_threaded_handshake() {
    loom::model(|| {
        let ready = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));

        let ready_a = Arc::clone(&ready);
        let completed_a = Arc::clone(&completed);
        let first = thread::spawn(move || {
            let marker = {
                let current = thread::current();
                assert_eq!(
                    std::mem::size_of_val(&current),
                    std::mem::size_of::<loom::thread::Thread>()
                );
                ready_a.fetch_add(1, Ordering::SeqCst)
            };
            assert!(marker <= 1);

            while ready_a.load(Ordering::SeqCst) < 2 {
                thread::yield_now();
            }

            let observed: Result<_, ()> = Ok({
                let current = thread::current();
                assert_eq!(format!("{:?}", current.clone()), format!("{:?}", current));
                completed_a.fetch_add(1, Ordering::SeqCst) + 1
            });

            let observed =
                observed.expect("thread context should be available before thread exits");

            assert!(observed >= 1);
            assert!(observed <= 2);
        });

        let ready_b = Arc::clone(&ready);
        let completed_b = Arc::clone(&completed);
        let second = thread::spawn(move || {
            let marker: Result<_, ()> = Ok({
                let current = thread::current();
                assert_eq!(
                    std::mem::size_of_val(&current),
                    std::mem::size_of::<loom::thread::Thread>()
                );
                ready_b.fetch_add(1, Ordering::SeqCst)
            });

            let marker =
                marker.expect("thread context should be available in second spawned thread");

            assert!(marker <= 1);

            while ready_b.load(Ordering::SeqCst) < 2 {
                thread::yield_now();
            }

            let observed = {
                let current = thread::current();
                assert_eq!(format!("{:?}", current.clone()), format!("{:?}", current));
                completed_b.fetch_add(1, Ordering::SeqCst) + 1
            };

            assert!(observed >= 1);
            assert!(observed <= 2);
        });

        first.join().expect("first worker should finish");
        second.join().expect("second worker should finish");

        assert_eq!(ready.load(Ordering::SeqCst), 2);
        assert_eq!(completed.load(Ordering::SeqCst), 2);
    });
}