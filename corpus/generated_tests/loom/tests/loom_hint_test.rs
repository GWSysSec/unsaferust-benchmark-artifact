use loom::hint;
use loom::sync::atomic::{fence, AtomicBool, AtomicU32, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn spin_loop_can_be_used_in_a_busy_wait_until_release_acquire_flag_is_seen() {
    loom::model(|| {
        let ready = Arc::new(AtomicBool::new(false));
        let payload = Arc::new(AtomicU32::new(0));

        let producer_ready = Arc::clone(&ready);
        let producer_payload = Arc::clone(&payload);
        let producer = thread::spawn(move || {
            assert!(!producer_ready.load(Ordering::Relaxed));
            assert_eq!(producer_payload.load(Ordering::Relaxed), 0);

            producer_payload.store(42, Ordering::Relaxed);
            fence(Ordering::Release);
            producer_ready.store(true, Ordering::Release);

            assert!(producer_ready.load(Ordering::Relaxed));
        });

        let consumer_ready = Arc::clone(&ready);
        let consumer_payload = Arc::clone(&payload);
        let consumer = thread::spawn(move || {
            let mut observed_spins = 0_u32;

            while !consumer_ready.load(Ordering::Acquire) {
                hint::spin_loop();
                thread::yield_now();
                observed_spins = observed_spins.saturating_add(1);
            }

            fence(Ordering::Acquire);
            let value = consumer_payload.load(Ordering::Relaxed);
            assert_eq!(value, 42);
            assert!(observed_spins <= observed_spins.saturating_add(1));
            value
        });

        producer.join().expect("producer thread should complete");
        let consumed = consumer.join().expect("consumer thread should complete");

        assert_eq!(consumed, 42);
        assert!(ready.load(Ordering::SeqCst));
        assert_eq!(payload.load(Ordering::SeqCst), 42);
    });
}

#[test]
fn spin_loop_does_not_change_atomic_state_by_itself() {
    loom::model(|| {
        let counter = AtomicU32::new(7);

        hint::spin_loop();
        assert_eq!(counter.load(Ordering::Relaxed), 7);

        assert_eq!(counter.fetch_add(5, Ordering::SeqCst), 7);
        hint::spin_loop();
        assert_eq!(counter.load(Ordering::SeqCst), 12);

        assert_eq!(counter.fetch_sub(2, Ordering::SeqCst), 12);
        hint::spin_loop();
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    });
}

#[test]
fn unreachable_unchecked_wrapper_panics_instead_of_returning() {
    let result = std::panic::catch_unwind(|| unsafe {
        hint::unreachable_unchecked();
    });

    assert!(
        result.is_err(),
        "loom::hint::unreachable_unchecked should panic instead of returning"
    );

    loom::model(|| {
        let mut still_usable_after_catching_panic =
            loom::alloc::Track::new(String::from("loom"));
        assert_eq!(still_usable_after_catching_panic.get_ref(), "loom");

        still_usable_after_catching_panic.get_mut().push_str("-hint");
        assert_eq!(still_usable_after_catching_panic.get_ref(), "loom-hint");

        let inner = still_usable_after_catching_panic.into_inner();
        assert_eq!(inner, "loom-hint");
    });
}