#![cfg(feature = "futures")]

use loom::future::block_on;
use loom::sync::atomic::{AtomicBool, AtomicUsize};
use loom::thread;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::task::{Context, Poll, Waker};

struct WakeState {
    ready: AtomicBool,
    polls: AtomicUsize,
    wake_calls: AtomicUsize,
    completions: AtomicUsize,
    waker: Mutex<Option<Waker>>,
}

struct ReadyAfterWake {
    state: Arc<WakeState>,
}

impl Future for ReadyAfterWake {
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = &self.state;
        state.polls.fetch_add(1, Relaxed);

        if state.ready.load(Acquire) {
            state.completions.fetch_add(1, Relaxed);
            Poll::Ready(99)
        } else {
            let mut slot = state.waker.lock().expect("waker mutex poisoned");
            *slot = Some(cx.waker().clone());

            if state.ready.load(Acquire) {
                state.completions.fetch_add(1, Relaxed);
                Poll::Ready(99)
            } else {
                Poll::Pending
            }
        }
    }
}

struct YieldOnce {
    polls: Arc<AtomicUsize>,
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = usize;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let poll_number = self.polls.fetch_add(1, Relaxed) + 1;

        if self.yielded {
            Poll::Ready(poll_number)
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[test]
fn block_on_completes_future_woken_from_loom_thread() {
    loom::model(|| {
        let state = Arc::new(WakeState {
            ready: AtomicBool::new(false),
            polls: AtomicUsize::new(0),
            wake_calls: AtomicUsize::new(0),
            completions: AtomicUsize::new(0),
            waker: Mutex::new(None),
        });

        let worker_state = Arc::clone(&state);
        let worker = thread::spawn(move || {
            thread::yield_now();
            worker_state.ready.store(true, Release);

            let maybe_waker = worker_state
                .waker
                .lock()
                .expect("waker mutex poisoned")
                .take();

            if let Some(waker) = maybe_waker {
                worker_state.wake_calls.fetch_add(1, Relaxed);
                waker.wake();
            }

            7usize
        });

        let value = block_on(ReadyAfterWake {
            state: Arc::clone(&state),
        });

        let worker_value = worker.join().expect("worker thread panicked");

        assert_eq!(value, 99);
        assert_eq!(worker_value, 7);
        assert!(state.ready.load(Acquire));
        assert!(state.polls.load(Relaxed) >= 1);
        assert_eq!(state.completions.load(Relaxed), 1);
        assert!(state.wake_calls.load(Relaxed) <= 1);
    });
}

#[test]
fn block_on_drives_self_waking_futures_in_sequence() {
    loom::model(|| {
        let polls = Arc::new(AtomicUsize::new(0));

        let total = block_on({
            let polls = Arc::clone(&polls);
            async move {
                let first = YieldOnce {
                    polls: Arc::clone(&polls),
                    yielded: false,
                }
                .await;

                thread::yield_now();

                let second = YieldOnce {
                    polls: Arc::clone(&polls),
                    yielded: false,
                }
                .await;

                first + second
            }
        });

        assert_eq!(total, 6);
        assert_eq!(polls.load(Relaxed), 4);
    });
}

#[test]
fn block_on_returns_immediately_ready_async_output() {
    loom::model(|| {
        let result = block_on(async {
            let mut values = Vec::new();
            values.push(10usize);
            values.push(20usize);
            values.push(12usize);

            let sum: usize = values.iter().copied().sum();
            let max = values.iter().copied().max();

            (sum, max, values.len())
        });

        assert_eq!(result.0, 42);
        assert_eq!(result.1, Some(20));
        assert_eq!(result.2, 3);
    });
}