#![allow(deprecated)]

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartSnapshot {
    index: usize,
    name: Option<String>,
    current_index: Option<usize>,
    current_threads: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExitSnapshot {
    index: usize,
    name: Option<String>,
    observed_marker: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    name: Option<String>,
    seed: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedLifecycleRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AsyncLifecycleRecord {
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
struct ExitPrepRecord {
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
fn thread_pool_builder_exit_handler_observes_final_state_after_broadcast_scope_and_async_work() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let lifecycle_marker = Arc::new(AtomicUsize::new(0));
    let (start_tx, start_rx) = mpsc::channel::<StartSnapshot>();
    let (exit_tx, exit_rx) = mpsc::channel::<ExitSnapshot>();

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("exit-handler-pipeline-worker-{index}"));

    let builder = rayon_core::ThreadPoolBuilder::start_handler(builder, move |index| {
        start_tx
            .send(StartSnapshot {
                index,
                name: std::thread::current().name().map(str::to_owned),
                current_index: rayon_core::current_thread_index(),
                current_threads: rayon_core::current_num_threads(),
            })
            .expect("start handler should report worker startup");
    });

    let builder = rayon_core::ThreadPoolBuilder::exit_handler(builder, {
        let lifecycle_marker = Arc::clone(&lifecycle_marker);

        move |index| {
            exit_tx
                .send(ExitSnapshot {
                    index,
                    name: std::thread::current().name().map(str::to_owned),
                    observed_marker: lifecycle_marker.load(Ordering::SeqCst),
                })
                .expect("exit handler should report worker shutdown");
        }
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("ThreadPoolBuilder with exit_handler should build");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut starts = recv_exact(&start_rx, thread_count, "start handler");
    starts.sort_by_key(|event| event.index);

    assert_eq!(
        starts.iter().map(|event| event.index).collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &starts {
        assert_eq!(
            event.name.as_deref(),
            Some(format!("exit-handler-pipeline-worker-{}", event.index).as_str())
        );
        assert_eq!(event.current_index, Some(event.index));
        assert_eq!(event.current_threads, thread_count);
    }

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "start handler should run exactly once per worker"
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
            name: std::thread::current().name().map(str::to_owned),
            seed: (index + 1) * (num_threads + 89),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    seeds.sort_by_key(|record| record.index);

    assert_eq!(seeds.len(), thread_count);
    assert_eq!(
        seeds.iter().map(|record| record.index).collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &seeds {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(
            record.name.as_deref(),
            Some(format!("exit-handler-pipeline-worker-{}", record.index).as_str())
        );
        assert_eq!(record.seed, (record.index + 1) * (thread_count + 89));
        assert!(record.pending_status_available);
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let scoped_records = Mutex::new(Vec::<ScopedLifecycleRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for seed_record in seeds.iter().cloned() {
            let scoped_records = &scoped_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO scoped work should run on a Rayon worker");

                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || num_threads + executing_index,
                );

                scoped_records
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(ScopedLifecycleRecord {
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
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(record.pending_status_available);
    }

    let expected_scoped_sum: usize = scoped_records.iter().map(|record| record.derived).sum();

    let seed_by_index_for_async = Arc::new(seed_by_index.clone());
    let (async_tx, async_rx) = mpsc::channel::<AsyncLifecycleRecord>();

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
                rayon_core::join(move || seed * 3, move || index + num_threads);

            async_tx
                .send(AsyncLifecycleRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    value: left + right,
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                })
                .expect("spawn_broadcast worker should report async lifecycle record");
        }
    });

    let mut async_records = recv_exact(
        &async_rx,
        thread_count,
        "spawn_broadcast before exit_handler",
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
            seed_by_index[record.index] * 3 + record.index + thread_count
        );
        assert!(record.pending_status_available);
    }

    assert!(
        async_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_broadcast should run exactly once per worker"
    );

    let expected_async_sum: usize = async_records.iter().map(|record| record.value).sum();
    let final_marker = expected_seed_sum + expected_scoped_sum + expected_async_sum;
    lifecycle_marker.store(final_marker, Ordering::SeqCst);

    drop(pool);

    let mut exits = recv_exact(&exit_rx, thread_count, "ThreadPoolBuilder::exit_handler");
    exits.sort_by_key(|event| event.index);

    assert_eq!(
        exits.iter().map(|event| event.index).collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &exits {
        assert_eq!(
            event.name.as_deref(),
            Some(format!("exit-handler-pipeline-worker-{}", event.index).as_str())
        );
        assert_eq!(
            event.observed_marker, final_marker,
            "exit_handler should observe state published after all pool work completed"
        );
    }

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit_handler should run exactly once per worker"
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
        .thread_name(|index| format!("exit-handler-panic-worker-{index}"));

    let builder = rayon_core::ThreadPoolBuilder::panic_handler(builder, {
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
                .expect("exit handler should report its attempt before any panic");

            if index % 2 == 0 {
                std::panic::panic_any(ExitPanicRecord {
                    index,
                    worker_name,
                    marker,
                    checksum: marker + index + thread_count * 10,
                });
            }
        }
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
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

        (index, num_threads, (index + 5) * (num_threads + 13))
    });

    warmup.sort_by_key(|entry| entry.0);

    assert_eq!(
        warmup.iter().map(|(index, _, _)| *index).collect::<BTreeSet<_>>(),
        expected_indices
    );

    let seed_by_origin: BTreeMap<usize, usize> = warmup
        .iter()
        .map(|(index, _, seed)| (*index, *seed))
        .collect();

    let prep_records = Mutex::new(Vec::<ExitPrepRecord>::new());

    let seed_sum = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for (origin_index, num_threads, seed) in warmup.iter().copied() {
            let prep_records = &prep_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO exit-prep work should run on a Rayon worker");

                assert!(executing_index < num_threads);
                assert_eq!(rayon_core::current_num_threads(), num_threads);

                let (left, right) =
                    rayon_core::join(move || seed + origin_index, move || num_threads + executing_index);

                prep_records
                    .lock()
                    .expect("exit-prep record mutex should not be poisoned")
                    .push(ExitPrepRecord {
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

    let expected_seed_sum: usize = warmup.iter().map(|(_, _, seed)| *seed).sum();
    assert_eq!(seed_sum, expected_seed_sum);

    let mut prep_records = prep_records
        .into_inner()
        .expect("exit-prep record mutex should not be poisoned");
    prep_records.sort_by_key(|record| record.origin_index);

    assert_eq!(prep_records.len(), thread_count);
    assert_eq!(
        prep_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &prep_records {
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
        assert!(record.pending_status_available);
    }

    let prep_value_sum: usize = prep_records.iter().map(|record| record.value).sum();
    let marker = seed_sum + prep_value_sum;
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

    assert_eq!(exit_attempts.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        exit_events
            .iter()
            .map(|event| event.attempt_ordinal)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &exit_events {
        assert_eq!(event.marker, marker);
        assert_eq!(
            event.worker_name.as_deref(),
            Some(format!("exit-handler-panic-worker-{}", event.index).as_str())
        );
    }

    let expected_panic_count = (0..thread_count).filter(|index| index % 2 == 0).count();
    let panic_events = recv_exact(
        &panic_rx,
        expected_panic_count,
        "panic handler for exit_handler panics",
    );

    assert_eq!(panic_calls.load(Ordering::SeqCst), expected_panic_count);

    let mut observed_panics = BTreeSet::<ExitPanicRecord>::new();
    for event in panic_events {
        match event {
            HandledExitPanic::Record(record) => {
                assert!(
                    observed_panics.insert(record),
                    "each panicking exit handler should produce one payload"
                );
            }
            unexpected => panic!("unexpected exit-handler panic payload: {unexpected:?}"),
        }
    }

    let expected_panics: BTreeSet<_> = (0..thread_count)
        .filter(|index| index % 2 == 0)
        .map(|index| ExitPanicRecord {
            index,
            worker_name: Some(format!("exit-handler-panic-worker-{index}")),
            marker,
            checksum: marker + index + thread_count * 10,
        })
        .collect();

    assert_eq!(observed_panics, expected_panics);

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit_handler should not run more than once per worker"
    );
    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic_handler should receive only the expected exit-handler panics"
    );
}