use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

const STATIC_DETACHED_MESSAGE: &str =
    "static payload handled by ThreadPoolBuilder::panic_handler";
const STRING_DETACHED_MESSAGE: &str =
    "string payload handled by ThreadPoolBuilder::panic_handler";
const LATE_DETACHED_MESSAGE: &str =
    "late detached payload handled after scoped panic propagation";

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DetachedPanicRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BroadcastPanicRecord {
    index: usize,
    seed: usize,
    num_threads: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HandledPanic {
    Detached(DetachedPanicRecord),
    Broadcast(BroadcastPanicRecord),
    Message(String),
    Unexpected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
    stage: usize,
    origin_index: usize,
    source_checksum: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
}

fn classify_payload(payload: &(dyn Any + Send)) -> HandledPanic {
    if let Some(record) = payload.downcast_ref::<DetachedPanicRecord>() {
        HandledPanic::Detached(record.clone())
    } else if let Some(record) = payload.downcast_ref::<BroadcastPanicRecord>() {
        HandledPanic::Broadcast(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledPanic::Message(message.clone())
    } else {
        HandledPanic::Unexpected
    }
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_owned()
    }
}

fn recv_exact<T>(receiver: &mpsc::Receiver<T>, count: usize, label: &str) -> Vec<T> {
    (0..count)
        .map(|index| {
            receiver.recv_timeout(Duration::from_secs(5)).unwrap_or_else(|error| {
                panic!(
                    "{label} did not produce item {} of {} in time: {error}",
                    index + 1,
                    count
                )
            })
        })
        .collect()
}

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_panic_handler_collects_detached_and_broadcast_panics_then_pool_recovers() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (panic_tx, panic_rx) = mpsc::channel::<HandledPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));
    let panic_calls = Arc::new(AtomicUsize::new(0));

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("target-panic-handler-worker-{index}"));

    let builder = rayon_core::ThreadPoolBuilder::panic_handler(builder, {
        let panic_tx = Arc::clone(&panic_tx);
        let panic_calls = Arc::clone(&panic_calls);

        move |payload| {
            panic_calls.fetch_add(1, Ordering::SeqCst);

            let event = catch_unwind(AssertUnwindSafe(|| classify_payload(&*payload)))
                .unwrap_or(HandledPanic::Unexpected);

            if let Ok(sender) = panic_tx.lock() {
                let _ = sender.send(event);
            }
        }
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("ThreadPoolBuilder with panic_handler should build a custom pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 127),
        }
    });

    seeds.sort_by_key(|record| record.index);

    assert_eq!(seeds.len(), thread_count);
    assert_eq!(
        seeds
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (expected_index, record) in seeds.iter().enumerate() {
        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(expected_index));
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 127));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();

    for seed_record in seeds.iter().cloned() {
        rayon_core::ThreadPool::spawn(&pool, move || {
            let executing_index = rayon_core::current_thread_index()
                .expect("detached panicking work should run on a Rayon worker");

            assert!(executing_index < seed_record.num_threads);
            assert_eq!(rayon_core::current_num_threads(), seed_record.num_threads);
            assert!(
                rayon_core::current_thread_has_pending_tasks().is_some(),
                "detached work should be able to query worker-local pending-task status"
            );

            let origin_index = seed_record.index;
            let seed = seed_record.seed;
            let num_threads = seed_record.num_threads;

            let (left, right) = rayon_core::join(
                move || seed + origin_index,
                move || num_threads * 10 + executing_index,
            );

            std::panic::panic_any(DetachedPanicRecord {
                origin_index,
                seed,
                executing_index,
                num_threads,
                checksum: left + right,
            });
        });
    }

    rayon_core::ThreadPool::spawn(&pool, || {
        std::panic::panic_any(STATIC_DETACHED_MESSAGE);
    });

    rayon_core::ThreadPool::spawn_fifo(&pool, || {
        std::panic::panic_any(String::from(STRING_DETACHED_MESSAGE));
    });

    let first_events = recv_exact(
        &panic_rx,
        thread_count + 2,
        "ThreadPoolBuilder::panic_handler first detached batch",
    );

    let mut detached_records = Vec::<DetachedPanicRecord>::new();
    let mut message_payloads = BTreeSet::<String>::new();

    for event in first_events {
        match event {
            HandledPanic::Detached(record) => detached_records.push(record),
            HandledPanic::Message(message) => {
                assert!(
                    message_payloads.insert(message),
                    "each message payload should be handled at most once"
                );
            }
            unexpected => panic!("unexpected first-batch panic handler event: {unexpected:?}"),
        }
    }

    detached_records.sort_unstable();

    assert_eq!(detached_records.len(), thread_count);
    assert_eq!(
        detached_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &detached_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.checksum,
            record.seed + record.origin_index + thread_count * 10 + record.executing_index
        );
    }

    assert_eq!(
        message_payloads,
        [STATIC_DETACHED_MESSAGE.to_owned(), STRING_DETACHED_MESSAGE.to_owned()]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should receive exactly the scheduled first-batch panics"
    );

    let seed_by_index_for_broadcast = Arc::new(seed_by_index.clone());
    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let seed_by_index_for_broadcast = Arc::clone(&seed_by_index_for_broadcast);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            let seed = seed_by_index_for_broadcast[index];
            let (left, right) =
                rayon_core::join(move || seed * 2 + index, move || num_threads * 100);

            std::panic::panic_any(BroadcastPanicRecord {
                index,
                seed,
                num_threads,
                checksum: left + right,
            });
        }
    });

    let broadcast_events = recv_exact(
        &panic_rx,
        thread_count,
        "ThreadPoolBuilder::panic_handler spawn_broadcast batch",
    );

    let mut broadcast_records = Vec::<BroadcastPanicRecord>::new();

    for event in broadcast_events {
        match event {
            HandledPanic::Broadcast(record) => broadcast_records.push(record),
            unexpected => panic!("unexpected broadcast panic handler event: {unexpected:?}"),
        }
    }

    broadcast_records.sort_unstable();

    assert_eq!(broadcast_records.len(), thread_count);
    assert_eq!(
        broadcast_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &broadcast_records {
        assert!(record.index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.checksum,
            record.seed * 2 + record.index + thread_count * 100
        );
    }

    assert_eq!(panic_calls.load(Ordering::SeqCst), thread_count * 2 + 2);

    let mut recovery_inputs = Vec::<(usize, usize, usize)>::new();
    recovery_inputs.extend(
        detached_records
            .iter()
            .map(|record| (0usize, record.origin_index, record.checksum)),
    );
    recovery_inputs.extend(
        broadcast_records
            .iter()
            .map(|record| (1usize, record.index, record.checksum)),
    );

    let expected_recovery_sources: BTreeMap<(usize, usize), usize> = recovery_inputs
        .iter()
        .copied()
        .map(|(stage, origin_index, checksum)| ((stage, origin_index), checksum))
        .collect();

    assert_eq!(expected_recovery_sources.len(), recovery_inputs.len());

    let expected_source_sum: usize = recovery_inputs
        .iter()
        .map(|(_, _, checksum)| *checksum)
        .sum();

    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for (stage, origin_index, source_checksum) in recovery_inputs.iter().copied() {
            let recovery_records = &recovery_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery scoped work should run inside the custom pool");

                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let pending_status_available =
                    rayon_core::current_thread_has_pending_tasks().is_some();

                let (left, right) = rayon_core::join(
                    move || source_checksum + stage,
                    move || origin_index + executing_index + thread_count,
                );

                recovery_records
                    .lock()
                    .expect("recovery mutex should not be poisoned")
                    .push(RecoveryRecord {
                        stage,
                        origin_index,
                        source_checksum,
                        executing_index,
                        value: left + right,
                        pending_status_available,
                    });
            });
        }

        expected_source_sum
    });

    assert_eq!(scope_return, expected_source_sum);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery mutex should not be poisoned");
    recovery_records.sort_by_key(|record| (record.stage, record.origin_index));

    assert_eq!(recovery_records.len(), thread_count * 2);
    assert_eq!(
        recovery_records
            .iter()
            .map(|record| (record.stage, record.origin_index))
            .collect::<BTreeSet<_>>(),
        expected_recovery_sources
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
    );

    for record in &recovery_records {
        assert!(record.stage <= 1);
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert!(
            record.pending_status_available,
            "recovery scoped work should observe worker-local pending-task status"
        );

        let expected_checksum = expected_recovery_sources[&(record.stage, record.origin_index)];
        assert_eq!(record.source_checksum, expected_checksum);
        assert_eq!(
            record.value,
            expected_checksum
                + record.stage
                + record.origin_index
                + record.executing_index
                + thread_count
        );
    }

    let (observed_recovery_sum, recomputed_recovery_sum) = rayon_core::ThreadPool::join(
        &pool,
        || recovery_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            recovery_records
                .iter()
                .map(|record| {
                    expected_recovery_sources[&(record.stage, record.origin_index)]
                        + record.stage
                        + record.origin_index
                        + record.executing_index
                        + thread_count
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_recovery_sum, recomputed_recovery_sum);

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "successful recovery work should not invoke the panic handler"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_panic_handler_is_not_used_for_scoped_panic_but_handles_later_detached_work()
{
    let thread_count = 2usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (panic_tx, panic_rx) = mpsc::channel::<HandledPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));
    let panic_calls = Arc::new(AtomicUsize::new(0));

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("target-panic-handler-propagation-worker-{index}"));

    let builder = rayon_core::ThreadPoolBuilder::panic_handler(builder, {
        let panic_tx = Arc::clone(&panic_tx);
        let panic_calls = Arc::clone(&panic_calls);

        move |payload| {
            panic_calls.fetch_add(1, Ordering::SeqCst);
            let event = classify_payload(&*payload);

            if let Ok(sender) = panic_tx.lock() {
                let _ = sender.send(event);
            }
        }
    });

    let pool =
        rayon_core::ThreadPoolBuilder::build(builder).expect("custom pool should build");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let scoped_panic = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::scope(&pool, |scope| {
            rayon_core::Scope::spawn(scope, |_| {
                let worker_index = rayon_core::current_thread_index()
                    .expect("scoped panicking work should run on a Rayon worker");
                assert!(worker_index < thread_count);
                panic!(
                    "scoped panic should propagate directly instead of using panic_handler"
                );
            });

            11usize
        });
    }));

    let payload = scoped_panic
        .expect_err("panic in scoped work should propagate to the scope caller");
    let message = panic_payload_to_string(&*payload);

    assert!(
        message.contains("scoped panic should propagate directly"),
        "unexpected scoped panic payload: {message:?}"
    );
    assert_eq!(panic_calls.load(Ordering::SeqCst), 0);
    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "propagated scoped panic should not be sent to panic_handler"
    );

    rayon_core::ThreadPool::spawn(&pool, move || {
        let worker_index = rayon_core::current_thread_index()
            .expect("late detached panicking work should run on a Rayon worker");

        assert!(worker_index < thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        std::panic::panic_any(String::from(LATE_DETACHED_MESSAGE));
    });

    let event = panic_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("late detached panic should reach ThreadPoolBuilder::panic_handler");

    match event {
        HandledPanic::Message(message) => assert_eq!(message, LATE_DETACHED_MESSAGE),
        unexpected => panic!("unexpected late detached panic event: {unexpected:?}"),
    }

    assert_eq!(panic_calls.load(Ordering::SeqCst), 1);

    let ((sum, left_index), (product, right_index)) = rayon_core::ThreadPool::join(
        &pool,
        || ((1usize..=5).sum::<usize>(), rayon_core::current_thread_index()),
        || {
            (
                (1usize..=5).product::<usize>(),
                rayon_core::current_thread_index(),
            )
        },
    );

    assert_eq!(sum, 15);
    assert_eq!(product, 120);

    for worker_index in [left_index, right_index] {
        let worker_index = worker_index.expect("join branch should run inside the custom pool");
        assert!(worker_index < thread_count);
    }

    let mut contexts = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (
            index,
            num_threads,
            rayon_core::current_thread_index(),
            index + num_threads,
        )
    });

    contexts.sort_by_key(|record| record.0);

    assert_eq!(contexts.len(), thread_count);
    assert_eq!(
        contexts
            .iter()
            .map(|(index, _, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, num_threads, current_index, value) in contexts {
        assert_eq!(num_threads, thread_count);
        assert_eq!(current_index, Some(index));
        assert_eq!(value, index + thread_count);
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "successful follow-up join and broadcast work should not invoke panic_handler"
    );
}