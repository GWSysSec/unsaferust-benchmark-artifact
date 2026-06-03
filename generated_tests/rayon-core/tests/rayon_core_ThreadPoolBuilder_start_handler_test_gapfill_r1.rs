use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartEvent {
    index: usize,
    ordinal: usize,
    name: Option<String>,
    current_index: Option<usize>,
    current_threads: usize,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartBackedObservation {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    name: Option<String>,
    seed: usize,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedFromStartRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    combined: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AsyncFromStartRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExitEvent {
    index: usize,
    name: Option<String>,
    marker: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartAttempt {
    index: usize,
    name: Option<String>,
    current_index: Option<usize>,
    current_threads: usize,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StartPanicRecord {
    index: usize,
    name: Option<String>,
    current_index: Option<usize>,
    current_threads: usize,
    seed: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HandledStartPanic {
    Record(StartPanicRecord),
    Message(String),
    Unexpected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoAfterStartPanicRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    panic_checksum: usize,
    value: usize,
    pending_status_available: bool,
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

fn classify_start_payload(payload: &(dyn Any + Send)) -> HandledStartPanic {
    if let Some(record) = payload.downcast_ref::<StartPanicRecord>() {
        HandledStartPanic::Record(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledStartPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledStartPanic::Message(message.clone())
    } else {
        HandledStartPanic::Unexpected
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_start_handler_seeds_named_workers_for_broadcast_scope_and_async_work() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let start_count = Arc::new(AtomicUsize::new(0));
    let start_seeds = Arc::new(Mutex::new(vec![0usize; thread_count]));
    let lifecycle_marker = Arc::new(AtomicUsize::new(0));
    let start_barrier = Arc::new(Barrier::new(thread_count));

    let (start_tx, start_rx) = mpsc::channel::<StartEvent>();
    let (exit_tx, exit_rx) = mpsc::channel::<ExitEvent>();

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("builder-start-seeded-worker-{index}"))
        .exit_handler({
            let lifecycle_marker = Arc::clone(&lifecycle_marker);

            move |index| {
                exit_tx
                    .send(ExitEvent {
                        index,
                        name: std::thread::current().name().map(str::to_owned),
                        marker: lifecycle_marker.load(Ordering::SeqCst),
                    })
                    .expect("exit handler should report worker shutdown");
            }
        });

    let builder = rayon_core::ThreadPoolBuilder::start_handler(builder, {
        let start_count = Arc::clone(&start_count);
        let start_seeds = Arc::clone(&start_seeds);
        let start_barrier = Arc::clone(&start_barrier);

        move |index| {
            let ordinal = start_count.fetch_add(1, Ordering::SeqCst);
            let seed = (index + 1) * (thread_count + 31);

            {
                let mut seeds = start_seeds
                    .lock()
                    .expect("start seed mutex should not be poisoned");
                seeds[index] = seed;
            }

            start_tx
                .send(StartEvent {
                    index,
                    ordinal,
                    name: std::thread::current().name().map(str::to_owned),
                    current_index: rayon_core::current_thread_index(),
                    current_threads: rayon_core::current_num_threads(),
                    seed,
                })
                .expect("start handler should report worker startup");

            start_barrier.wait();
        }
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("ThreadPoolBuilder with start_handler should build a custom pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut start_events = recv_exact(
        &start_rx,
        thread_count,
        "ThreadPoolBuilder::start_handler",
    );
    start_events.sort_by_key(|event| event.index);

    assert_eq!(start_count.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        start_events
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );
    assert_eq!(
        start_events
            .iter()
            .map(|event| event.ordinal)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &start_events {
        let expected_name = format!("builder-start-seeded-worker-{}", event.index);
        assert_eq!(event.name.as_deref(), Some(expected_name.as_str()));
        assert_eq!(event.current_index, Some(event.index));
        assert_eq!(event.current_threads, thread_count);
        assert_eq!(event.seed, (event.index + 1) * (thread_count + 31));
    }

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "start_handler should run exactly once per worker"
    );

    let seed_by_index = start_seeds
        .lock()
        .expect("start seed mutex should not be poisoned")
        .clone();

    for (index, seed) in seed_by_index.iter().copied().enumerate() {
        assert_eq!(seed, (index + 1) * (thread_count + 31));
    }

    let mut observations = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let seed = seed_by_index[index];

        StartBackedObservation {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            name: std::thread::current().name().map(str::to_owned),
            seed,
            derived: seed + index + num_threads * 10,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    observations.sort_by_key(|record| record.index);

    assert_eq!(observations.len(), thread_count);
    assert_eq!(
        observations
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for observation in &observations {
        let expected_name = format!("builder-start-seeded-worker-{}", observation.index);
        assert_eq!(observation.num_threads, thread_count);
        assert_eq!(observation.current_index, Some(observation.index));
        assert_eq!(observation.name.as_deref(), Some(expected_name.as_str()));
        assert_eq!(
            observation.seed,
            (observation.index + 1) * (thread_count + 31)
        );
        assert_eq!(
            observation.derived,
            observation.seed + observation.index + thread_count * 10
        );
        assert!(
            observation.pending_status_available,
            "broadcast work should observe worker-local pending-task status"
        );
    }

    let observation_by_index: BTreeMap<usize, StartBackedObservation> = observations
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();

    assert_eq!(observation_by_index.len(), thread_count);

    let expected_observation_sum: usize =
        observations.iter().map(|record| record.derived).sum();

    let scoped_records = Mutex::new(Vec::<ScopedFromStartRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for observation in observations.iter().cloned() {
            let scoped_records = &scoped_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped work should run inside the custom pool");

                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let origin_index = observation.index;
                let seed = observation.seed;
                let derived = observation.derived;
                let num_threads = observation.num_threads;

                let (left, right) = rayon_core::join(
                    move || derived + seed,
                    move || origin_index + executing_index + num_threads,
                );

                scoped_records
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(ScopedFromStartRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        combined: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        expected_observation_sum
    });

    assert_eq!(scope_return, expected_observation_sum);

    let mut scoped_records = scoped_records
        .into_inner()
        .expect("scoped record mutex should not be poisoned");
    scoped_records.sort_by_key(|record| record.origin_index);

    assert_eq!(scoped_records.len(), thread_count);
    assert_eq!(
        scoped_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &scoped_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert!(record.pending_status_available);

        let observation = observation_by_index
            .get(&record.origin_index)
            .expect("scoped record should correspond to a broadcast observation");

        assert_eq!(record.seed, observation.seed);
        assert_eq!(
            record.combined,
            observation.derived
                + observation.seed
                + observation.index
                + record.executing_index
                + observation.num_threads
        );
    }

    let scoped_total: usize = scoped_records.iter().map(|record| record.combined).sum();

    let seed_by_index_for_async = Arc::new(seed_by_index.clone());
    let (async_tx, async_rx) = mpsc::channel::<AsyncFromStartRecord>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let seed_by_index_for_async = Arc::clone(&seed_by_index_for_async);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let seed = seed_by_index_for_async[index];
            let (left, right) =
                rayon_core::join(move || seed + scoped_total, move || index + num_threads);

            async_tx
                .send(AsyncFromStartRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    value: left + right,
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                })
                .expect("spawn_broadcast worker should report its record");
        }
    });

    let mut async_records = recv_exact(
        &async_rx,
        thread_count,
        "ThreadPool::spawn_broadcast after start_handler",
    );
    async_records.sort_by_key(|record| record.index);

    assert_eq!(
        async_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &async_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.value,
            seed_by_index[record.index] + scoped_total + record.index + thread_count
        );
        assert!(record.pending_status_available);
    }

    assert!(
        async_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_broadcast should run exactly once per worker"
    );

    let async_total: usize = async_records.iter().map(|record| record.value).sum();
    let final_marker = expected_observation_sum + scoped_total + async_total;
    lifecycle_marker.store(final_marker, Ordering::SeqCst);

    drop(pool);

    let mut exit_events = recv_exact(&exit_rx, thread_count, "ThreadPoolBuilder::exit_handler");
    exit_events.sort_by_key(|event| event.index);

    assert_eq!(
        exit_events
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &exit_events {
        let expected_name = format!("builder-start-seeded-worker-{}", event.index);
        assert_eq!(event.name.as_deref(), Some(expected_name.as_str()));
        assert_eq!(
            event.marker, final_marker,
            "exit handler should observe state published after start-seeded work completed"
        );
    }

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit handler should run exactly once per worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_start_handler_panics_are_handled_and_pool_recovers() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let start_seeds = Arc::new(Mutex::new(vec![0usize; thread_count]));
    let panic_calls = Arc::new(AtomicUsize::new(0));

    let (start_tx, start_rx) = mpsc::channel::<StartAttempt>();
    let (panic_tx, panic_rx) = mpsc::channel::<HandledStartPanic>();
    let (exit_tx, exit_rx) = mpsc::channel::<usize>();

    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("builder-start-panic-worker-{index}"))
        .panic_handler({
            let panic_tx = Arc::clone(&panic_tx);
            let panic_calls = Arc::clone(&panic_calls);

            move |payload| {
                panic_calls.fetch_add(1, Ordering::SeqCst);

                let event = classify_start_payload(&*payload);
                if let Ok(sender) = panic_tx.lock() {
                    let _ = sender.send(event);
                }
            }
        })
        .exit_handler(move |index| {
            exit_tx
                .send(index)
                .expect("exit handler should report shutdown after start panic recovery");
        });

    let builder = rayon_core::ThreadPoolBuilder::start_handler(builder, {
        let start_seeds = Arc::clone(&start_seeds);

        move |index| {
            let seed = (index + 2) * (thread_count + 17);

            {
                let mut seeds = start_seeds
                    .lock()
                    .expect("start seed mutex should not be poisoned");
                seeds[index] = seed;
            }

            let name = std::thread::current().name().map(str::to_owned);
            let current_index = rayon_core::current_thread_index();
            let current_threads = rayon_core::current_num_threads();

            start_tx
                .send(StartAttempt {
                    index,
                    name: name.clone(),
                    current_index,
                    current_threads,
                    seed,
                })
                .expect("start handler should report before any intentional panic");

            if index % 2 == 0 {
                std::panic::panic_any(StartPanicRecord {
                    index,
                    name,
                    current_index,
                    current_threads,
                    seed,
                    checksum: seed + index + thread_count * 100,
                });
            }
        }
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("start_handler panics should be forwarded to panic_handler and startup should continue");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut start_attempts = recv_exact(
        &start_rx,
        thread_count,
        "ThreadPoolBuilder::start_handler attempts",
    );
    start_attempts.sort_by_key(|attempt| attempt.index);

    assert_eq!(
        start_attempts
            .iter()
            .map(|attempt| attempt.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for attempt in &start_attempts {
        let expected_name = format!("builder-start-panic-worker-{}", attempt.index);
        assert_eq!(attempt.name.as_deref(), Some(expected_name.as_str()));
        assert_eq!(attempt.current_index, Some(attempt.index));
        assert_eq!(attempt.current_threads, thread_count);
        assert_eq!(attempt.seed, (attempt.index + 2) * (thread_count + 17));
    }

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "start_handler should be invoked exactly once per worker even when some invocations panic"
    );

    let expected_panic_count = (0..thread_count).filter(|index| index % 2 == 0).count();
    let panic_events = recv_exact(
        &panic_rx,
        expected_panic_count,
        "panic_handler for ThreadPoolBuilder::start_handler",
    );

    assert_eq!(panic_calls.load(Ordering::SeqCst), expected_panic_count);

    let mut panic_records = Vec::<StartPanicRecord>::new();
    for event in panic_events {
        match event {
            HandledStartPanic::Record(record) => panic_records.push(record),
            unexpected => panic!("unexpected start_handler panic payload: {unexpected:?}"),
        }
    }

    panic_records.sort_by_key(|record| record.index);

    let expected_panic_records: BTreeSet<_> = start_attempts
        .iter()
        .filter(|attempt| attempt.index % 2 == 0)
        .map(|attempt| StartPanicRecord {
            index: attempt.index,
            name: attempt.name.clone(),
            current_index: attempt.current_index,
            current_threads: attempt.current_threads,
            seed: attempt.seed,
            checksum: attempt.seed + attempt.index + thread_count * 100,
        })
        .collect();

    assert_eq!(
        panic_records.iter().cloned().collect::<BTreeSet<_>>(),
        expected_panic_records
    );

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic_handler should receive exactly the intentional start_handler panics"
    );

    let seed_by_index = start_seeds
        .lock()
        .expect("start seed mutex should not be poisoned")
        .clone();

    for (index, seed) in seed_by_index.iter().copied().enumerate() {
        assert_eq!(seed, (index + 2) * (thread_count + 17));
    }

    let mut recovery_broadcast = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let seed = seed_by_index[index];

        RecoveryBroadcastRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed,
            derived: seed * 2 + index + num_threads,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    recovery_broadcast.sort_by_key(|record| record.index);

    assert_eq!(
        recovery_broadcast
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &recovery_broadcast {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.derived,
            seed_by_index[record.index] * 2 + record.index + thread_count
        );
        assert!(
            record.pending_status_available,
            "pool should accept broadcast work after start_handler panics"
        );
    }

    let recovery_by_index: BTreeMap<usize, RecoveryBroadcastRecord> = recovery_broadcast
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();

    assert_eq!(recovery_by_index.len(), thread_count);

    let panic_checksum_by_index: BTreeMap<usize, usize> = panic_records
        .iter()
        .map(|record| (record.index, record.checksum))
        .collect();

    let recovery_sum: usize = recovery_broadcast
        .iter()
        .map(|record| record.derived)
        .sum();
    let panic_checksum_sum: usize = panic_checksum_by_index.values().copied().sum();

    let fifo_records = Mutex::new(Vec::<FifoAfterStartPanicRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for record in recovery_broadcast.iter().cloned() {
            let panic_checksum = panic_checksum_by_index
                .get(&record.index)
                .copied()
                .unwrap_or(0);
            let fifo_records = &fifo_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO recovery work should run on a Rayon worker");

                assert!(executing_index < record.num_threads);
                assert_eq!(rayon_core::current_num_threads(), record.num_threads);

                let origin_index = record.index;
                let seed = record.seed;
                let derived = record.derived;

                let (left, right) =
                    rayon_core::join(move || derived + panic_checksum, move || seed + executing_index);

                fifo_records
                    .lock()
                    .expect("FIFO record mutex should not be poisoned")
                    .push(FifoAfterStartPanicRecord {
                        origin_index,
                        seed,
                        executing_index,
                        panic_checksum,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        recovery_sum + panic_checksum_sum
    });

    assert_eq!(scope_return, recovery_sum + panic_checksum_sum);

    let mut fifo_records = fifo_records
        .into_inner()
        .expect("FIFO record mutex should not be poisoned");
    fifo_records.sort_by_key(|record| record.origin_index);

    assert_eq!(fifo_records.len(), thread_count);
    assert_eq!(
        fifo_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &fifo_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.panic_checksum,
            panic_checksum_by_index
                .get(&record.origin_index)
                .copied()
                .unwrap_or(0)
        );
        assert!(record.pending_status_available);

        let broadcast_record = recovery_by_index
            .get(&record.origin_index)
            .expect("FIFO record should correspond to a recovery broadcast record");

        assert_eq!(
            record.value,
            broadcast_record.derived
                + record.panic_checksum
                + broadcast_record.seed
                + record.executing_index
        );
    }

    let (observed_fifo_sum, recomputed_fifo_sum) = rayon_core::ThreadPool::join(
        &pool,
        || fifo_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            fifo_records
                .iter()
                .map(|record| {
                    let broadcast_record = recovery_by_index
                        .get(&record.origin_index)
                        .expect("broadcast record should exist during recomputation");

                    broadcast_record.derived
                        + record.panic_checksum
                        + broadcast_record.seed
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_fifo_sum, recomputed_fifo_sum);

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "successful recovery work should not invoke the panic handler"
    );

    drop(pool);

    let exited: BTreeSet<_> = recv_exact(
        &exit_rx,
        thread_count,
        "exit handler after start_handler panic recovery",
    )
    .into_iter()
    .collect();

    assert_eq!(exited, expected_indices);
    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "all workers should shut down exactly once"
    );
}