use loom::model;
use loom::sync::atomic::{fence, AtomicBool, AtomicU32, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn model_explores_atomic_fetch_add_workflow() {
    loom::model::model(|| {
        let counter = Arc::new(AtomicU32::new(0));
        let started = Arc::new(AtomicBool::new(false));

        let first_counter = Arc::clone(&counter);
        let first_started = Arc::clone(&started);
        let first = thread::spawn(move || {
            assert!(!first_started.swap(true, Ordering::AcqRel));
            let previous = first_counter.fetch_add(1, Ordering::AcqRel);
            assert!(previous <= 1);
            previous
        });

        let second_counter = Arc::clone(&counter);
        let second_started = Arc::clone(&started);
        let second = thread::spawn(move || {
            while !second_started.load(Ordering::Acquire) {
                thread::yield_now();
            }

            let previous = second_counter.fetch_add(1, Ordering::AcqRel);
            assert!(previous <= 1);
            previous
        });

        let first_previous = first.join().unwrap();
        let second_previous = second.join().unwrap();

        assert_ne!(first_previous, second_previous);
        assert_eq!(first_previous + second_previous, 1);
        assert_eq!(counter.load(Ordering::Acquire), 2);
        assert!(started.load(Ordering::Acquire));
    });
}

#[test]
fn builder_checkpoint_file_and_check_validate_release_acquire_handoff() {
    let checkpoint_path = std::env::temp_dir().join(format!(
        "loom-builder-checkpoint-{}-{}.json",
        std::process::id(),
        "release_acquire_handoff"
    ));
    let checkpoint_path = checkpoint_path.to_string_lossy().into_owned();

    let mut builder = model::Builder::new();
    builder.checkpoint_file(&checkpoint_path);

    builder.check(|| {
        let ready = Arc::new(AtomicBool::new(false));
        let payload = Arc::new(AtomicU32::new(0));
        let acknowledgements = Arc::new(AtomicU32::new(0));

        let producer_ready = Arc::clone(&ready);
        let producer_payload = Arc::clone(&payload);
        let producer_acknowledgements = Arc::clone(&acknowledgements);
        let producer = thread::spawn(move || {
            assert_eq!(producer_payload.load(Ordering::Relaxed), 0);
            producer_payload.store(7, Ordering::Relaxed);
            fence(Ordering::Release);
            producer_ready.store(true, Ordering::Release);

            while producer_acknowledgements.load(Ordering::Acquire) == 0 {
                thread::yield_now();
            }

            assert_eq!(producer_acknowledgements.load(Ordering::Acquire), 1);
        });

        let consumer_ready = Arc::clone(&ready);
        let consumer_payload = Arc::clone(&payload);
        let consumer_acknowledgements = Arc::clone(&acknowledgements);
        let consumer = thread::spawn(move || {
            while !consumer_ready.load(Ordering::Acquire) {
                thread::yield_now();
            }

            fence(Ordering::Acquire);
            let observed = consumer_payload.load(Ordering::Relaxed);
            assert_eq!(observed, 7);

            let previous = consumer_acknowledgements.fetch_add(1, Ordering::AcqRel);
            assert_eq!(previous, 0);

            observed
        });

        producer.join().unwrap();
        let observed = consumer.join().unwrap();

        assert_eq!(observed, 7);
        assert!(ready.load(Ordering::Acquire));
        assert_eq!(payload.load(Ordering::Acquire), 7);
        assert_eq!(acknowledgements.load(Ordering::Acquire), 1);
    });
}