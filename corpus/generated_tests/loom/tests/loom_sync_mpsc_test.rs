use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use loom::model;
use loom::sync::mpsc::channel;
use loom::thread;

#[test]
fn try_recv_observes_empty_buffered_messages_and_closed_channel() {
    model(|| {
        let (tx, rx) = channel::<u32>();

        let initial = rx.try_recv();
        assert!(
            matches!(initial, Err(TryRecvError::Empty)),
            "new channel with a live sender should be empty, got {:?}",
            initial
        );

        let tx_a = tx.clone();
        let tx_b = tx.clone();
        drop(tx);

        let sender_a = thread::spawn(move || {
            tx_a.send(10).expect("receiver is still alive");
            thread::yield_now();
            tx_a.send(11).expect("receiver is still alive");
        });

        let sender_b = thread::spawn(move || {
            thread::yield_now();
            tx_b.send(20).expect("receiver is still alive");
        });

        sender_a.join().expect("sender_a should not panic");
        sender_b.join().expect("sender_b should not panic");

        let first = rx.try_recv();
        let second = rx.try_recv();
        let third = rx.try_recv();

        assert!(first.is_ok(), "first buffered receive failed: {:?}", first);
        assert!(second.is_ok(), "second buffered receive failed: {:?}", second);
        assert!(third.is_ok(), "third buffered receive failed: {:?}", third);

        let mut received = vec![first.unwrap(), second.unwrap(), third.unwrap()];
        received.sort_unstable();
        assert_eq!(received, vec![10, 11, 20]);

        let drained = rx.try_recv();
        assert!(
            matches!(drained, Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected)),
            "drained channel should report no further messages, got {:?}",
            drained
        );
    });
}

#[test]
fn try_recv_does_not_consume_message_after_failed_empty_poll() {
    model(|| {
        let (tx, rx) = channel::<&'static str>();

        let before_send = rx.try_recv();
        assert!(
            matches!(before_send, Err(TryRecvError::Empty)),
            "empty channel with live sender should report Empty, got {:?}",
            before_send
        );

        tx.send("ready").expect("send should succeed");
        drop(tx);

        assert_eq!(rx.try_recv(), Ok("ready"));

        let after_drain = rx.try_recv();
        assert!(
            matches!(after_drain, Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected)),
            "after draining the only message there should be no value left, got {:?}",
            after_drain
        );
    });
}

#[test]
fn recv_timeout_panics_without_draining_buffered_message() {
    model(|| {
        let (tx, rx) = channel::<u32>();

        tx.send(7).expect("buffered send should succeed before timeout call");

        let timeout_result = catch_unwind(AssertUnwindSafe(|| {
            let _ = rx.recv_timeout(Duration::from_millis(1));
        }));

        assert!(
            timeout_result.is_err(),
            "loom mpsc recv_timeout is expected to be unsupported and panic"
        );

        assert_eq!(
            rx.try_recv(),
            Ok(7),
            "unsupported recv_timeout should not consume an already buffered message"
        );

        drop(tx);

        let after_message = rx.try_recv();
        assert!(
            matches!(after_message, Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected)),
            "only one message was sent, got {:?}",
            after_message
        );
    });
}

#[test]
fn recv_timeout_panics_on_empty_channel_with_live_sender() {
    model(|| {
        let (tx, rx) = channel::<u8>();

        let timeout_result = catch_unwind(AssertUnwindSafe(|| {
            let _ = rx.recv_timeout(Duration::from_nanos(0));
        }));

        assert!(
            timeout_result.is_err(),
            "recv_timeout should panic even when the channel is merely empty"
        );

        tx.send(99).expect("channel should still be usable after caught panic");
        drop(tx);

        assert_eq!(rx.try_recv(), Ok(99));
    });
}