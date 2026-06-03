use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use loom::model;
use loom::sync::mpsc::channel;
use loom::thread;

#[test]
fn try_recv_reports_empty_then_receives_then_empty_after_sender_drops() {
    model(|| {
        let (tx, rx) = channel::<u32>();

        let first_attempt = rx.try_recv();
        assert!(
            matches!(first_attempt, Err(TryRecvError::Empty)),
            "new channel with a live sender should be empty, got {:?}",
            first_attempt
        );

        let sender = thread::spawn(move || {
            tx.send(10).expect("send should succeed while receiver is alive");
        });

        sender.join().expect("sender thread should not panic");

        let received = rx.try_recv();
        assert_eq!(received, Ok(10));

        let after_message = rx.try_recv();
        assert!(
            matches!(after_message, Err(TryRecvError::Empty)),
            "loom mpsc try_recv reports an empty drained queue, got {:?}",
            after_message
        );
    });
}

#[test]
fn recv_timeout_is_unsupported_and_does_not_drain_buffered_messages() {
    model(|| {
        let (tx, rx) = channel::<&'static str>();

        tx.send("first").expect("first send should succeed");
        tx.send("second").expect("second send should succeed");
        drop(tx);

        let timeout_result = catch_unwind(AssertUnwindSafe(|| {
            let _ = rx.recv_timeout(Duration::from_millis(1));
        }));

        assert!(
            timeout_result.is_err(),
            "loom currently documents recv_timeout as unsupported and should panic"
        );

        let first = rx.try_recv();
        assert_eq!(first, Ok("first"));

        let second = rx.try_recv();
        assert_eq!(second, Ok("second"));

        let after_drain = rx.try_recv();
        assert!(
            matches!(after_drain, Err(TryRecvError::Empty)),
            "drained loom channel should report an empty queue, got {:?}",
            after_drain
        );
    });
}

#[test]
fn try_recv_state_is_preserved_when_recv_timeout_panics() {
    model(|| {
        let (tx, rx) = channel::<u8>();

        assert!(
            matches!(rx.try_recv(), Err(TryRecvError::Empty)),
            "channel should begin empty while sender is still alive"
        );

        let sender = thread::spawn(move || {
            tx.send(1).expect("first send should succeed");
            thread::yield_now();
            tx.send(2).expect("second send should succeed");
        });

        sender.join().expect("sender thread should not panic");

        let first = rx.try_recv();
        assert_eq!(first, Ok(1));

        let timeout_result = catch_unwind(AssertUnwindSafe(|| {
            let _ = rx.recv_timeout(Duration::from_secs(1));
        }));

        assert!(
            timeout_result.is_err(),
            "loom currently documents recv_timeout as unsupported and should panic"
        );

        let second = rx.try_recv();
        assert_eq!(
            second,
            Ok(2),
            "a failed recv_timeout call must not consume the queued message"
        );

        let final_try = rx.try_recv();
        assert!(
            matches!(final_try, Err(TryRecvError::Empty)),
            "all values were consumed and the queue is empty, got {:?}",
            final_try
        );
    });
}