use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartEvent {
    index: usize,
    name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExitEvent {
    index: usize,
    name: Option<String>,
    observed_marker: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AsyncRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExitAttempt {
    index: usize,
    attempt_ordinal: usize,
    worker_name: Option<String>,
    marker: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoExitPrepRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ExitPanicRecord {
    index: usize,
    worker_name: Option<String>,
    marker: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HandledExitPanic {
    Record(ExitPanicRecord),
    Message(String),
    Unexpected,
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

fn classify_exit_payload(payload: &(dyn Any + Send)) -> HandledExitPanic {
    if let Some(record) = payload.downcast_ref::<ExitPanicRecord>() {
        HandledExitPanic::Record(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledExitPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledExitPanic::Message(message.clone())
    } else {
        HandledExitPanic::Unexpected
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_exit_handler_reports_named_workers_after_broadcast_scope_and_async_work() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let lifecycle_marker = Arc::new(AtomicUsize::new(0));
    let (start_tx, start_rx) = mpsc::channel::<StartEvent>();
    let (exit_tx, exit_rx) = mpsc::channel::<ExitEvent>();

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("builder-exit-lifecycle-worker-{index}"))
        .start_handler(move |index| {
            start_tx
                .send(StartEvent {
                    index,
                    name: std::thread::current().name().map(str::to_owned),
                })
                .expect("start handler should be able to report worker startup");
        });

    let builder = rayon_core::ThreadPoolBuilder::exit_handler(builder, {
        let lifecycle_marker = Arc::clone(&lifecycle_marker);

        move |index| {
            exit_tx
                .send(ExitEvent {
                    index,
                    name: std::thread::current().name().map(str::to_owned),
                    observed_marker: lifecycle_marker.load(Ordering::SeqCst),
                })
                .expect("exit handler should be able to report worker shutdown");
        }
    });

    let pool = builder
        .build()
        .expect("ThreadPoolBuilder with exit_handler should build a custom pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "the integration-test thread should not be a worker in this custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut start_events = recv_exact(&start_rx, thread_count, "start handler");
    start_events.sort_by_key(|event| event.index);

    assert_eq!(
        start_events
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &start_events {
        assert_eq!(
            event.name.as_deref(),
            Some(format!("builder-exit-lifecycle-worker-{}", event.index).as_str())
        );
    }

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "start handler should run exactly once for each worker"
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 97),
            name: std::thread::current().name().map(str::to_owned),
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

    for record in &seeds {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, (record.index + 1) * (thread_count + 97));
        assert_eq!(
            record.name.as_deref(),
            Some(format!("builder-exit-lifecycle-worker-{}", record.index).as_str())
        );
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();
    let expected_scoped_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed + index + thread_count)
        .sum();

    let scoped_records = Mutex::new(Vec::<ScopedRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for seed_record in seeds.iter().cloned() {
            let scoped_records = &scoped_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped work should run inside the custom Rayon pool");
                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (left, right) =
                    rayon_core::join(move || seed + origin_index, move || num_threads);

                scoped_records
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(ScopedRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        derived: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        expected_seed_sum
    });

    assert_eq!(scope_return, expected_seed_sum);

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
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.derived,
            record.seed + record.origin_index + thread_count
        );
        assert!(
            record.pending_status_available,
            "scoped worker should be able to query worker-local pending-task status"
        );
    }

    assert_eq!(
        scoped_records
            .iter()
            .map(|record| record.derived)
            .sum::<usize>(),
        expected_scoped_sum
    );

    let seed_by_index_for_async = Arc::new(seed_by_index.clone());
    let (async_tx, async_rx) = mpsc::channel::<AsyncRecord>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let seed_by_index_for_async = Arc::clone(&seed_by_index_for_async);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            let seed = seed_by_index_for_async[index];
            let (left, right) =
                rayon_core::join(move || seed * 2, move || index + num_threads);

            async_tx
                .send(AsyncRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    value: left + right,
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                })
                .expect("spawn_broadcast worker should report its async record");
        }
    });

    let mut async_records = recv_exact(
        &async_rx,
        thread_count,
        "ThreadPool::spawn_broadcast before shutdown",
    );
    async_records.sort_by_key(|record| record.index);

    assert_eq!(
        async_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let expected_async_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed * 2 + index + thread_count)
        .sum();

    for record in &async_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.value,
            seed_by_index[record.index] * 2 + record.index + thread_count
        );
        assert!(
            record.pending_status_available,
            "spawn_broadcast worker should observe pending-task status"
        );
    }

    assert_eq!(
        async_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>(),
        expected_async_sum
    );

    let final_marker = expected_seed_sum + expected_scoped_sum + expected_async_sum;
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
        assert_eq!(
            event.name.as_deref(),
            Some(format!("builder-exit-lifecycle-worker-{}", event.index).as_str())
        );
        assert_eq!(
            event.observed_marker, final_marker,
            "exit handler should observe state published after all pool work completed"
        );
    }

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit handler should run exactly once per worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_exit_handler_panics_are_forwarded_to_panic_handler_after_fifo_work() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let final_marker = Arc::new(AtomicUsize::new(0));
    let exit_attempts = Arc::new(AtomicUsize::new(0));
    let panic_calls = Arc::new(AtomicUsize::new(0));

    let (exit_tx, exit_rx) = mpsc::channel::<ExitAttempt>();
    let (panic_tx, panic_rx) = mpsc::channel::<HandledExitPanic>();

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("builder-exit-panic-worker-{index}"))
        .panic_handler({
            let panic_calls = Arc::clone(&panic_calls);

            move |payload| {
                panic_calls.fetch_add(1, Ordering::SeqCst);
                panic_tx
                    .send(classify_exit_payload(&*payload))
                    .expect("panic handler should report exit-handler panic payload");
            }
        });

    let builder = rayon_core::ThreadPoolBuilder::exit_handler(builder, {
        let final_marker = Arc::clone(&final_marker);
        let exit_attempts = Arc::clone(&exit_attempts);

        move |index| {
            let marker = final_marker.load(Ordering::SeqCst);
            let worker_name = std::thread::current().name().map(str::to_owned);
            let attempt_ordinal = exit_attempts.fetch_add(1, Ordering::SeqCst);

            exit_tx
                .send(ExitAttempt {
                    index,
                    attempt_ordinal,
                    worker_name: worker_name.clone(),
                    marker,
                })
                .expect("exit handler should report its attempt before any intentional panic");

            if index % 2 == 0 {
                std::panic::panic_any(ExitPanicRecord {
                    index,
                    worker_name,
                    marker,
                    checksum: marker + index + thread_count,
                });
            }
        }
    });

    let pool = builder
        .build()
        .expect("ThreadPoolBuilder should build with both panic_handler and exit_handler");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );

    let mut warmup = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        (index, num_threads, (index + 1) * (num_threads + 5))
    });

    warmup.sort_by_key(|entry| entry.0);

    assert_eq!(
        warmup
            .iter()
            .map(|(index, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let seed_by_origin: BTreeMap<usize, usize> = warmup
        .iter()
        .map(|(index, _, seed)| (*index, *seed))
        .collect();
    assert_eq!(seed_by_origin.len(), thread_count);

    let fifo_records = Mutex::new(Vec::<FifoExitPrepRecord>::new());

    let fifo_seed_sum = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for (origin_index, num_threads, seed) in warmup.iter().copied() {
            let fifo_records = &fifo_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO work should run on a Rayon worker");
                assert!(executing_index < num_threads);
                assert_eq!(rayon_core::current_num_threads(), num_threads);

                let (left, right) =
                    rayon_core::join(move || seed + origin_index, move || num_threads + executing_index);

                fifo_records
                    .lock()
                    .expect("FIFO prep record mutex should not be poisoned")
                    .push(FifoExitPrepRecord {
                        origin_index,
                        seed,
                        executing_index,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        warmup.iter().map(|(_, _, seed)| *seed).sum::<usize>()
    });

    let expected_fifo_seed_sum: usize = warmup.iter().map(|(_, _, seed)| *seed).sum();
    assert_eq!(fifo_seed_sum, expected_fifo_seed_sum);

    let mut fifo_records = fifo_records
        .into_inner()
        .expect("FIFO prep record mutex should not be poisoned");
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
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "FIFO work should be able to query worker-local pending-task status"
        );
    }

    let fifo_value_sum: usize = fifo_records.iter().map(|record| record.value).sum();
    let marker = fifo_seed_sum + fifo_value_sum;
    final_marker.store(marker, Ordering::SeqCst);

    drop(pool);

    let mut exit_events = recv_exact(&exit_rx, thread_count, "panicking exit handler attempts");
    exit_events.sort_by_key(|event| event.index);

    assert_eq!(
        exit_events
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let attempt_ordinals: BTreeSet<_> = exit_events
        .iter()
        .map(|event| event.attempt_ordinal)
        .collect();
    assert_eq!(attempt_ordinals, expected_worker_indices(thread_count));

    for event in &exit_events {
        assert_eq!(event.marker, marker);
        assert_eq!(
            event.worker_name.as_deref(),
            Some(format!("builder-exit-panic-worker-{}", event.index).as_str())
        );
    }

    assert_eq!(exit_attempts.load(Ordering::SeqCst), thread_count);

    let expected_panic_count = (0..thread_count).filter(|index| index % 2 == 0).count();
    let panic_events = recv_exact(
        &panic_rx,
        expected_panic_count,
        "panic handler for ThreadPoolBuilder::exit_handler",
    );

    assert_eq!(panic_calls.load(Ordering::SeqCst), expected_panic_count);

    let mut observed_panics = BTreeSet::new();

    for event in panic_events {
        match event {
            HandledExitPanic::Record(record) => {
                assert!(
                    observed_panics.insert(record),
                    "each panicking exit handler should produce exactly one panic payload"
                );
            }
            unexpected => panic!("unexpected exit-handler panic payload: {unexpected:?}"),
        }
    }

    let expected_panics: BTreeSet<_> = (0..thread_count)
        .filter(|index| index % 2 == 0)
        .map(|index| ExitPanicRecord {
            index,
            worker_name: Some(format!("builder-exit-panic-worker-{index}")),
            marker,
            checksum: marker + index + thread_count,
        })
        .collect();

    assert_eq!(observed_panics, expected_panics);

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit handler should not run more than once per worker"
    );
    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should receive only the expected exit-handler panics"
    );
}