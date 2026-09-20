use loom::model::Builder;
use loom::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use loom::sync::Arc;
use loom::thread;

fn unique_checkpoint_path(test_name: &str) -> String {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    path.push(format!(
        "loom_{test_name}_{}_{}.checkpoint",
        std::process::id(),
        nanos
    ));

    path.to_string_lossy().into_owned()
}

#[test]
fn builder_checkpoint_file_and_check_explore_atomic_handoff_workflow() {
    let checkpoint = unique_checkpoint_path("builder_checkpoint_file_and_check");
    let _ = std::fs::remove_file(&checkpoint);

    let mut builder = Builder::new();

    builder.checkpoint_file(&checkpoint).check(|| {
        let counter = Arc::new(AtomicU32::new(0));
        let first_started = Arc::new(AtomicBool::new(false));
        let second_started = Arc::new(AtomicBool::new(false));

        let first_counter = Arc::clone(&counter);
        let first_started_for_thread = Arc::clone(&first_started);
        let first = thread::spawn(move || {
            assert!(
                first_started_for_thread
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            );

            let previous = first_counter.fetch_add(1, Ordering::AcqRel);
            assert!(previous < 2);
            previous
        });

        let second_counter = Arc::clone(&counter);
        let first_started_for_second = Arc::clone(&first_started);
        let second_started_for_thread = Arc::clone(&second_started);
        let second = thread::spawn(move || {
            while !first_started_for_second.load(Ordering::Acquire) {
                thread::yield_now();
            }

            assert!(
                second_started_for_thread
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            );

            let previous = second_counter.fetch_update(
                Ordering::AcqRel,
                Ordering::Acquire,
                |current| {
                    assert!(current < 2);
                    Some(current + 1)
                },
            );

            assert!(previous.is_ok());
            previous.unwrap()
        });

        let first_previous = first.join().unwrap();
        let second_previous = second.join().unwrap();

        assert_ne!(first_previous, second_previous);
        assert_eq!(first_previous + second_previous, 1);
        assert_eq!(counter.load(Ordering::Acquire), 2);
        assert!(first_started.load(Ordering::Acquire));
        assert!(second_started.load(Ordering::Acquire));
    });

    let _ = std::fs::remove_file(&checkpoint);
}