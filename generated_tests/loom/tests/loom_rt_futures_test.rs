use loom::sync::atomic::{fence, AtomicBool, AtomicUsize, Ordering};
use loom::sync::mpsc;
use loom::sync::Arc;
use loom::thread;

fn consume_notify() -> bool {
    #[cfg(feature = "futures")]
    {
        loom::rt::futures::consume_notify()
    }

    #[cfg(not(feature = "futures"))]
    {
        false
    }
}

#[test]
fn consume_notify_is_consuming_and_does_not_disturb_message_passing() {
    loom::model(|| {
        assert!(!consume_notify());
        assert!(!consume_notify());

        let (tx, rx) = mpsc::channel::<usize>();
        let ready = Arc::new(AtomicBool::new(false));
        let sum = Arc::new(AtomicUsize::new(0));

        let producer_ready = Arc::clone(&ready);
        let producer_sum = Arc::clone(&sum);

        let producer = thread::spawn(move || {
            assert!(!producer_ready.load(Ordering::Acquire));

            tx.send(10).expect("first send should succeed");
            tx.send(32).expect("second send should succeed");

            producer_sum.fetch_add(1, Ordering::AcqRel);
            producer_ready.store(true, Ordering::Release);

            assert!(!consume_notify());
        });

        while !ready.load(Ordering::Acquire) {
            thread::yield_now();
            assert!(!consume_notify());
        }

        fence(Ordering::SeqCst);

        let first = rx.recv().expect("first value should be received");
        let second = rx.recv().expect("second value should be received");

        assert_eq!(first + second, 42);
        assert_eq!(sum.load(Ordering::Acquire), 1);
        assert!(ready.load(Ordering::Acquire));

        producer.join().expect("producer thread should not panic");

        assert!(rx.try_recv().is_err());
        assert!(!consume_notify());
    });
}

#[test]
fn consume_notify_can_be_checked_while_threads_race_on_atomic_state() {
    let mut builder = loom::model::Builder::new();

    builder.check(|| {
        assert!(!consume_notify());

        let counter = Arc::new(AtomicUsize::new(0));
        let winner_seen = Arc::new(AtomicBool::new(false));

        let left_counter = Arc::clone(&counter);
        let left_winner_seen = Arc::clone(&winner_seen);
        let left = thread::spawn(move || {
            let previous = left_counter.fetch_add(1, Ordering::AcqRel);
            assert!(previous <= 1);

            if previous == 0 {
                let old = left_winner_seen.swap(true, Ordering::AcqRel);
                assert!(!old);
            }

            assert!(!consume_notify());
        });

        let right_counter = Arc::clone(&counter);
        let right_winner_seen = Arc::clone(&winner_seen);
        let right = thread::spawn(move || {
            let previous = right_counter.fetch_add(1, Ordering::AcqRel);
            assert!(previous <= 1);

            if previous == 0 {
                let old = right_winner_seen.swap(true, Ordering::AcqRel);
                assert!(!old);
            }

            assert!(!consume_notify());
        });

        left.join().expect("left worker should not panic");
        right.join().expect("right worker should not panic");

        assert_eq!(counter.load(Ordering::Acquire), 2);
        assert!(winner_seen.load(Ordering::Acquire));
        assert!(!consume_notify());
    });
}