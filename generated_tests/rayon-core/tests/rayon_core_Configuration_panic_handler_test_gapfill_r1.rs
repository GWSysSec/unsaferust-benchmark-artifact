#![allow(deprecated)]

use std::any::Any;
use std::collections::BTreeSet;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

const STATIC_PANIC: &str = "literal panic handled by deprecated Configuration::panic_handler";
const STRING_PANIC: &str = "owned string panic handled by deprecated Configuration::panic_handler";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct WorkerPanic {
    worker_index: usize,
    thread_count: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BroadcastPanic {
    worker_index: usize,
    thread_count: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HandledPanic {
    Worker(WorkerPanic),
    Broadcast(BroadcastPanic),
    Message(String),
    Unexpected,
}

fn classify_payload(payload: &(dyn Any + Send)) -> HandledPanic {
    if let Some(record) = payload.downcast_ref::<WorkerPanic>() {
        HandledPanic::Worker(record.clone())
    } else if let Some(record) = payload.downcast_ref::<BroadcastPanic>() {
        HandledPanic::Broadcast(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledPanic::Message(message.clone())
    } else {
        HandledPanic::Unexpected
    }
}

fn recv_exact<T>(receiver: &mpsc::Receiver<T>, count: usize, label: &str) -> Vec<T> {
    (0..count)
        .map(|index| {
            receiver
                .recv_timeout(Duration::from_secs(5))
                .unwrap_or_else(|error| {
                    panic!(
                        "{label} did not produce item {} of {} in time: {error}",
                        index + 1,
                        count
                    )
                })
        })
        .collect()
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn deprecated_configuration_panic_handler_receives_detached_payloads_and_pool_recovers() {
    let thread_count = 3usize;
    let (panic_tx, panic_rx) = mpsc::channel::<HandledPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let config = rayon_core::Configuration::new().num_threads(thread_count);
    let config = rayon_core::Configuration::panic_handler(config, {
        let panic_tx = Arc::clone(&panic_tx);

        move |payload| {
            let event = classify_payload(&*payload);
            if let Ok(sender) = panic_tx.lock() {
                let _ = sender.send(event);
            }
        }
    });

    let pool = rayon_core::Configuration::build(config)
        .expect("Configuration with panic_handler should build a custom pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "the test thread should not be a worker in the custom pool"
    );

    let mut records = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let worker_index = rayon_core::BroadcastContext::index(&context);
        let thread_count = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(thread_count, 3);
        assert_eq!(rayon_core::current_thread_index(), Some(worker_index));

        WorkerPanic {
            worker_index,
            thread_count,
            value: (worker_index + 1) * (thread_count + 10),
        }
    });
    records.sort_unstable();

    let expected_records: BTreeSet<_> = (0..thread_count)
        .map(|worker_index| WorkerPanic {
            worker_index,
            thread_count,
            value: (worker_index + 1) * (thread_count + 10),
        })
        .collect();

    assert_eq!(
        records.iter().cloned().collect::<BTreeSet<_>>(),
        expected_records
    );

    for record in records.iter().cloned() {
        rayon_core::ThreadPool::spawn(&pool, move || {
            std::panic::panic_any(record);
        });
    }

    rayon_core::ThreadPool::spawn_fifo(&pool, || {
        std::panic::panic_any(STATIC_PANIC);
    });

    rayon_core::ThreadPool::spawn(&pool, || {
        std::panic::panic_any(String::from(STRING_PANIC));
    });

    let events = recv_exact(&panic_rx, thread_count + 2, "panic handler");

    let mut handled_records = BTreeSet::new();
    let mut handled_messages = BTreeSet::new();

    for event in events {
        match event {
            HandledPanic::Worker(record) => {
                assert!(
                    handled_records.insert(record),
                    "each worker panic payload should be handled once"
                );
            }
            HandledPanic::Message(message) => {
                assert!(
                    handled_messages.insert(message),
                    "each message panic payload should be handled once"
                );
            }
            unexpected => panic!("unexpected panic payload classification: {unexpected:?}"),
        }
    }

    assert_eq!(handled_records, expected_records);

    let expected_messages: BTreeSet<_> = [STATIC_PANIC.to_owned(), STRING_PANIC.to_owned()]
        .into_iter()
        .collect();
    assert_eq!(handled_messages, expected_messages);

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should run exactly once for each detached panic"
    );

    let post_panic_results = Arc::new(Mutex::new(Vec::<usize>::new()));
    let records_for_scope = records.clone();

    let scope_result = rayon_core::ThreadPool::scope(&pool, {
        let post_panic_results = Arc::clone(&post_panic_results);

        move |scope| {
            for record in records_for_scope {
                let post_panic_results = Arc::clone(&post_panic_results);

                rayon_core::Scope::spawn(scope, move |_| {
                    let value = record.value;
                    let worker_index = record.worker_index;
                    let (left, right) =
                        rayon_core::join(move || value, move || worker_index);

                    post_panic_results
                        .lock()
                        .expect("post-panic result mutex should not be poisoned")
                        .push(left + right);
                });
            }

            "scoped work completed after handled panics"
        }
    });

    assert_eq!(scope_result, "scoped work completed after handled panics");

    let mut observed = Arc::try_unwrap(post_panic_results)
        .expect("all scoped tasks should release their Arc clones")
        .into_inner()
        .expect("post-panic result mutex should not be poisoned");
    observed.sort_unstable();

    let mut expected: Vec<_> = records
        .iter()
        .map(|record| record.value + record.worker_index)
        .collect();
    expected.sort_unstable();

    assert_eq!(observed, expected);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn deprecated_configuration_panic_handler_handles_spawn_broadcast_panics_and_follow_up_work() {
    let thread_count = 2usize;
    let (panic_tx, panic_rx) = mpsc::channel::<HandledPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let config = rayon_core::Configuration::new().num_threads(thread_count);
    let config = rayon_core::Configuration::panic_handler(config, {
        let panic_tx = Arc::clone(&panic_tx);

        move |payload| {
            let event = classify_payload(&*payload);
            if let Ok(sender) = panic_tx.lock() {
                let _ = sender.send(event);
            }
        }
    });

    let pool = rayon_core::ThreadPool::new(config)
        .expect("ThreadPool::new should build from Configuration with panic_handler");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );

    let warmup_values = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        index + 1
    });

    let checksum_base: usize = warmup_values.iter().sum();
    assert_eq!(checksum_base, (1..=thread_count).sum::<usize>());

    rayon_core::ThreadPool::spawn_broadcast(&pool, move |context| {
        let worker_index = rayon_core::BroadcastContext::index(&context);
        let thread_count = rayon_core::BroadcastContext::num_threads(&context);

        std::panic::panic_any(BroadcastPanic {
            worker_index,
            thread_count,
            checksum: checksum_base * (worker_index + 1),
        });
    });

    let events = recv_exact(&panic_rx, thread_count, "broadcast panic handler");

    let mut observed_broadcast_panics = Vec::new();
    for event in events {
        match event {
            HandledPanic::Broadcast(record) => observed_broadcast_panics.push(record),
            unexpected => panic!("unexpected broadcast panic payload: {unexpected:?}"),
        }
    }
    observed_broadcast_panics.sort_unstable();

    let expected_broadcast_panics: Vec<_> = (0..thread_count)
        .map(|worker_index| BroadcastPanic {
            worker_index,
            thread_count,
            checksum: checksum_base * (worker_index + 1),
        })
        .collect();

    assert_eq!(observed_broadcast_panics, expected_broadcast_panics);

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_broadcast should produce exactly one handled panic per worker"
    );

    let (index_sum, checksum_sum) = rayon_core::ThreadPool::join(
        &pool,
        || {
            observed_broadcast_panics
                .iter()
                .map(|record| record.worker_index)
                .sum::<usize>()
        },
        || {
            observed_broadcast_panics
                .iter()
                .map(|record| record.checksum)
                .sum::<usize>()
        },
    );

    assert_eq!(index_sum, thread_count * (thread_count - 1) / 2);
    assert_eq!(
        checksum_sum,
        expected_broadcast_panics
            .iter()
            .map(|record| record.checksum)
            .sum::<usize>()
    );

    let scoped_outputs = Mutex::new(Vec::<usize>::new());

    let scope_len = rayon_core::ThreadPool::in_place_scope(&pool, |scope| {
        for record in observed_broadcast_panics.iter().cloned() {
            let scoped_outputs = &scoped_outputs;

            rayon_core::Scope::spawn(scope, move |_| {
                scoped_outputs
                    .lock()
                    .expect("scoped output mutex should not be poisoned")
                    .push(record.checksum / (record.worker_index + 1));
            });
        }

        observed_broadcast_panics.len()
    });

    assert_eq!(scope_len, thread_count);

    let mut scoped_outputs = scoped_outputs
        .into_inner()
        .expect("scoped output mutex should not be poisoned");
    scoped_outputs.sort_unstable();

    assert_eq!(scoped_outputs, vec![checksum_base; thread_count]);
}