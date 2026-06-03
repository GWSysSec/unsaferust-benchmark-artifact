use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

const STATIC_MESSAGE: &str = "static detached panic handled by ThreadPoolBuilder::panic_handler";
const STRING_MESSAGE: &str = "owned detached panic handled by ThreadPoolBuilder::panic_handler";
const LATER_DETACHED_MESSAGE: &str =
    "later detached panic reaches ThreadPoolBuilder::panic_handler";

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
struct FifoPanicRecord {
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
enum HandledBuilderPanic {
    Detached(DetachedPanicRecord),
    Fifo(FifoPanicRecord),
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

fn classify_payload(payload: &(dyn Any + Send)) -> HandledBuilderPanic {
    if let Some(record) = payload.downcast_ref::<DetachedPanicRecord>() {
        HandledBuilderPanic::Detached(record.clone())
    } else if let Some(record) = payload.downcast_ref::<FifoPanicRecord>() {
        HandledBuilderPanic::Fifo(record.clone())
    } else if let Some(record) = payload.downcast_ref::<BroadcastPanicRecord>() {
        HandledBuilderPanic::Broadcast(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledBuilderPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledBuilderPanic::Message(message.clone())
    } else {
        HandledBuilderPanic::Unexpected
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

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_panic_handler_receives_spawn_fifo_and_broadcast_payloads_then_pool_recovers()
{
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (panic_tx, panic_rx) = mpsc::channel::<HandledBuilderPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("builder-panic-handler-worker-{index}"));

    let builder = rayon_core::ThreadPoolBuilder::panic_handler(builder, {
        let panic_tx = Arc::clone(&panic_tx);

        move |payload| {
            let event = catch_unwind(AssertUnwindSafe(|| classify_payload(&*payload)))
                .unwrap_or(HandledBuilderPanic::Unexpected);

            if let Ok(sender) = panic_tx.lock() {
                let _ = sender.send(event);
            }
        }
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("ThreadPoolBuilder with panic_handler should build a custom pool");
    let pool_ref = &pool;

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(pool_ref),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "the integration-test thread should not be a worker in the custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 113),
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 113));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();

    for seed_record in seeds.iter().cloned() {
        rayon_core::ThreadPool::spawn(pool_ref, move || {
            let executing_index = rayon_core::current_thread_index()
                .expect("detached spawn panic should run on a Rayon worker");

            assert!(executing_index < seed_record.num_threads);
            assert_eq!(rayon_core::current_num_threads(), seed_record.num_threads);
            assert!(
                rayon_core::current_thread_has_pending_tasks().is_some(),
                "detached spawn work should be able to query worker-local pending-task status"
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

    let fifo_seed = seeds
        .first()
        .cloned()
        .expect("broadcast should produce a seed for the FIFO panic");
    rayon_core::ThreadPool::spawn_fifo(pool_ref, move || {
        let executing_index = rayon_core::current_thread_index()
            .expect("detached FIFO panic should run on a Rayon worker");

        assert!(executing_index < fifo_seed.num_threads);
        assert_eq!(rayon_core::current_num_threads(), fifo_seed.num_threads);

        let origin_index = fifo_seed.index;
        let seed = fifo_seed.seed;
        let num_threads = fifo_seed.num_threads;

        let (left, right) =
            rayon_core::join(move || seed * 2, move || executing_index + num_threads);

        std::panic::panic_any(FifoPanicRecord {
            origin_index,
            seed,
            executing_index,
            num_threads,
            checksum: left + right,
        });
    });

    rayon_core::ThreadPool::spawn(pool_ref, || {
        std::panic::panic_any(STATIC_MESSAGE);
    });
    rayon_core::ThreadPool::spawn_fifo(pool_ref, || {
        std::panic::panic_any(String::from(STRING_MESSAGE));
    });

    let first_events = recv_exact(
        &panic_rx,
        thread_count + 3,
        "ThreadPoolBuilder::panic_handler first detached batch",
    );

    let mut detached_records = Vec::<DetachedPanicRecord>::new();
    let mut fifo_records = Vec::<FifoPanicRecord>::new();
    let mut message_payloads = BTreeSet::<String>::new();

    for event in first_events {
        match event {
            HandledBuilderPanic::Detached(record) => detached_records.push(record),
            HandledBuilderPanic::Fifo(record) => fifo_records.push(record),
            HandledBuilderPanic::Message(message) => {
                assert!(
                    message_payloads.insert(message),
                    "each message payload should be handled at most once"
                );
            }
            unexpected => panic!("unexpected first-batch panic handler event: {unexpected:?}"),
        }
    }

    detached_records.sort_unstable();
    fifo_records.sort_unstable();

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

    assert_eq!(fifo_records.len(), 1);
    let fifo_record = &fifo_records[0];
    assert_eq!(fifo_record.origin_index, 0);
    assert!(fifo_record.executing_index < thread_count);
    assert_eq!(fifo_record.num_threads, thread_count);
    assert_eq!(fifo_record.seed, seed_by_index[0]);
    assert_eq!(
        fifo_record.checksum,
        fifo_record.seed * 2 + fifo_record.executing_index + thread_count
    );

    let mut expected_messages = BTreeSet::new();
    expected_messages.insert(STATIC_MESSAGE.to_owned());
    expected_messages.insert(STRING_MESSAGE.to_owned());
    assert_eq!(message_payloads, expected_messages);

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should receive exactly the scheduled first-batch detached panics"
    );

    let seed_by_index_for_broadcast = Arc::new(seed_by_index.clone());
    rayon_core::ThreadPool::spawn_broadcast(pool_ref, {
        let seed_by_index_for_broadcast = Arc::clone(&seed_by_index_for_broadcast);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            let seed = seed_by_index_for_broadcast[index];
            let (left, right) =
                rayon_core::join(move || seed + index, move || num_threads * 100);

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
            HandledBuilderPanic::Broadcast(record) => broadcast_records.push(record),
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
            record.seed + record.index + thread_count * 100
        );
    }

    let mut recovery_inputs = Vec::<(usize, usize, usize)>::new();

    recovery_inputs.extend(
        detached_records
            .iter()
            .map(|record| (0usize, record.origin_index, record.checksum)),
    );
    recovery_inputs.extend(
        fifo_records
            .iter()
            .map(|record| (1usize, record.origin_index, record.checksum)),
    );
    recovery_inputs.extend(
        broadcast_records
            .iter()
            .map(|record| (2usize, record.index, record.checksum)),
    );

    assert_eq!(recovery_inputs.len(), thread_count * 2 + 1);

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

    let scope_return = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        for (stage, origin_index, source_checksum) in recovery_inputs.iter().copied() {
            let recovery_records = &recovery_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery scoped work should run inside the custom Rayon pool");

                assert!(executing_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                let pending_status_available =
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();

                let (left, right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || source_checksum + origin_index,
                    move || thread_count + executing_index + stage,
                );

                recovery_records
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
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
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| (record.stage, record.origin_index));

    assert_eq!(recovery_records.len(), recovery_inputs.len());
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
        assert!(record.stage <= 2);
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
                + record.origin_index
                + thread_count
                + record.executing_index
                + record.stage
        );
    }

    let (observed_recovery_sum, recomputed_recovery_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || recovery_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            recovery_records
                .iter()
                .map(|record| {
                    expected_recovery_sources[&(record.stage, record.origin_index)]
                        + record.origin_index
                        + thread_count
                        + record.executing_index
                        + record.stage
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_recovery_sum, recomputed_recovery_sum);

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "non-panicking recovery work should not invoke the panic handler"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_panic_handler_is_not_used_for_scoped_panics_but_handles_later_detached_work()
{
    let thread_count = 2usize;
    let (panic_tx, panic_rx) = mpsc::channel::<HandledBuilderPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("builder-panic-handler-propagation-worker-{index}"));

    let builder = rayon_core::ThreadPoolBuilder::panic_handler(builder, {
        let panic_tx = Arc::clone(&panic_tx);

        move |payload| {
            let event = catch_unwind(AssertUnwindSafe(|| classify_payload(&*payload)))
                .unwrap_or(HandledBuilderPanic::Unexpected);

            if let Ok(sender) = panic_tx.lock() {
                let _ = sender.send(event);
            }
        }
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("ThreadPoolBuilder with panic_handler should build");
    let pool_ref = &pool;

    let scoped_panic = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::ThreadPool::scope(pool_ref, |scope| {
            rayon_core::Scope::spawn(scope, move |_| {
                let worker_index = rayon_core::current_thread_index()
                    .expect("scoped panicking work should run on a Rayon worker");
                assert!(worker_index < thread_count);
                panic!("scoped panic should propagate to the scope caller");
            });
        });
    }));

    let payload = scoped_panic
        .expect_err("panic in scoped work should propagate to the ThreadPool::scope caller");
    let message = panic_payload_to_string(&*payload);
    assert!(
        message.contains("scoped panic should propagate"),
        "unexpected scoped panic payload: {message:?}"
    );

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "propagated scoped panic should not be routed through ThreadPoolBuilder::panic_handler"
    );

    rayon_core::ThreadPool::spawn(pool_ref, move || {
        let worker_index = rayon_core::current_thread_index()
            .expect("detached panicking work should run on a Rayon worker");
        assert!(worker_index < thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        std::panic::panic_any(LATER_DETACHED_MESSAGE);
    });

    let event = panic_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("later detached panic should reach the configured panic handler");

    match event {
        HandledBuilderPanic::Message(message) => {
            assert_eq!(message, LATER_DETACHED_MESSAGE);
        }
        unexpected => panic!("unexpected later detached panic event: {unexpected:?}"),
    }

    let ((sum, left_index), (product, right_index)) = rayon_core::ThreadPool::join(
        pool_ref,
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

    let mut contexts = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, num_threads, index + num_threads)
    });

    contexts.sort_by_key(|record| record.0);

    assert_eq!(contexts.len(), thread_count);
    assert_eq!(
        contexts
            .iter()
            .map(|(index, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(thread_count)
    );

    for (index, num_threads, value) in contexts {
        assert_eq!(num_threads, thread_count);
        assert_eq!(value, index + thread_count);
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "successful follow-up join and broadcast work should not invoke the panic handler"
    );
}