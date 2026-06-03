use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleWorkerFifoEvent {
    queue_position: usize,
    value: usize,
    worker_index: Option<usize>,
    num_threads: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpawnFifoRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedFollowupRecord {
    origin_index: usize,
    fifo_executing_index: usize,
    executing_index: usize,
    combined: usize,
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
enum HandledFifoPanic {
    Record(FifoPanicRecord),
    Message(String),
    Unexpected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoRecoveryRecord {
    origin_index: usize,
    seed: usize,
    panic_checksum: usize,
    executing_index: usize,
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

fn classify_payload(payload: &(dyn Any + Send)) -> HandledFifoPanic {
    if let Some(record) = payload.downcast_ref::<FifoPanicRecord>() {
        HandledFifoPanic::Record(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledFifoPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledFifoPanic::Message(message.clone())
    } else {
        HandledFifoPanic::Unexpected
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_spawn_fifo_is_detached_and_preserves_fifo_order_when_single_worker_released() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("spawn-fifo-single-worker-{index}"))
        .build()
        .expect("single-worker thread pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let release_worker = Arc::new((Mutex::new(false), Condvar::new()));
    let (started_tx, started_rx) = mpsc::channel::<(Option<usize>, usize)>();

    rayon_core::ThreadPool::spawn(&pool, {
        let release_worker = Arc::clone(&release_worker);

        move || {
            assert_eq!(rayon_core::current_thread_index(), Some(0));
            assert_eq!(rayon_core::current_num_threads(), 1);

            started_tx
                .send((
                    rayon_core::current_thread_index(),
                    rayon_core::current_num_threads(),
                ))
                .expect("test thread should wait for the blocking worker task");

            let (lock, condvar) = &*release_worker;
            let mut released = lock
                .lock()
                .expect("release mutex should not be poisoned while worker is blocked");

            while !*released {
                released = condvar
                    .wait(released)
                    .expect("release mutex should not be poisoned while waiting");
            }
        }
    });

    let (blocking_index, blocking_threads) = started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("blocking worker task should start before FIFO tasks are queued");

    assert_eq!(blocking_index, Some(0));
    assert_eq!(blocking_threads, 1);

    let (event_tx, event_rx) = mpsc::channel::<SingleWorkerFifoEvent>();

    for queue_position in 0usize..8 {
        let event_tx = event_tx.clone();

        rayon_core::ThreadPool::spawn_fifo(&pool, move || {
            assert_eq!(rayon_core::current_num_threads(), 1);
            assert_eq!(rayon_core::current_thread_index(), Some(0));

            let pending_status_available =
                rayon_core::current_thread_has_pending_tasks().is_some();

            let (left, right) =
                rayon_core::join(move || queue_position, move || queue_position * 10);

            event_tx
                .send(SingleWorkerFifoEvent {
                    queue_position,
                    value: left + right,
                    worker_index: rayon_core::current_thread_index(),
                    num_threads: rayon_core::current_num_threads(),
                    pending_status_available,
                })
                .expect("single-worker FIFO task should report its result");
        });
    }

    drop(event_tx);

    assert!(
        event_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "detached FIFO work should not run while the only worker is blocked"
    );

    {
        let (lock, condvar) = &*release_worker;
        let mut released = lock
            .lock()
            .expect("release mutex should not be poisoned when releasing worker");
        *released = true;
        condvar.notify_one();
    }

    let events = recv_exact(
        &event_rx,
        8,
        "single-worker ThreadPool::spawn_fifo ordered work",
    );

    assert!(
        event_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each queued FIFO task should report exactly once"
    );

    for (expected_position, event) in events.iter().enumerate() {
        assert_eq!(event.queue_position, expected_position);
        assert_eq!(event.value, expected_position + expected_position * 10);
        assert_eq!(event.worker_index, Some(0));
        assert_eq!(event.num_threads, 1);
        assert!(
            event.pending_status_available,
            "FIFO task should be able to query worker-local pending-task status"
        );
    }

    let observed_values: Vec<_> = events.iter().map(|event| event.value).collect();
    let expected_values: Vec<_> = (0usize..8).map(|value| value + value * 10).collect();

    assert_eq!(
        observed_values, expected_values,
        "with one blocked worker, ThreadPool::spawn_fifo jobs should run in queue order"
    );

    let final_context = rayon_core::ThreadPool::broadcast(&pool, |context| {
        (
            rayon_core::BroadcastContext::index(&context),
            rayon_core::BroadcastContext::num_threads(&context),
            rayon_core::current_thread_index(),
        )
    });

    assert_eq!(final_context, vec![(0, 1, Some(0))]);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_spawn_fifo_consumes_broadcast_seeds_and_feeds_scoped_followup_work() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("spawn-fifo-pipeline-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

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
            seed: (index + 1) * (num_threads + 71),
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 71));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let (fifo_tx, fifo_rx) = mpsc::channel::<SpawnFifoRecord>();
    let fifo_started = Arc::new(AtomicUsize::new(0));

    for seed_record in seeds.iter().cloned() {
        let fifo_tx = fifo_tx.clone();
        let fifo_started = Arc::clone(&fifo_started);

        rayon_core::ThreadPool::spawn_fifo(&pool, move || {
            fifo_started.fetch_add(1, Ordering::SeqCst);

            let executing_index = rayon_core::current_thread_index()
                .expect("ThreadPool::spawn_fifo work should execute on a Rayon worker");

            assert!(executing_index < thread_count);
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            let origin_index = seed_record.index;
            let seed = seed_record.seed;
            let (seed_component, worker_component) =
                rayon_core::join(move || seed + origin_index, move || thread_count + executing_index);

            fifo_tx
                .send(SpawnFifoRecord {
                    origin_index,
                    seed,
                    executing_index,
                    num_threads: rayon_core::current_num_threads(),
                    current_index: rayon_core::current_thread_index(),
                    derived: seed_component + worker_component,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                })
                .expect("spawn_fifo worker should report its derived value");
        });
    }

    drop(fifo_tx);

    let mut fifo_records = recv_exact(
        &fifo_rx,
        thread_count,
        "ThreadPool::spawn_fifo seeded pipeline",
    );
    fifo_records.sort_by_key(|record| record.origin_index);

    assert_eq!(fifo_started.load(Ordering::SeqCst), thread_count);
    assert!(
        fifo_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each detached FIFO task should report exactly once"
    );

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
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.executing_index));
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.derived,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "spawn_fifo work should be able to query worker-local pending-task status"
        );
    }

    let fifo_by_origin: BTreeMap<usize, SpawnFifoRecord> = fifo_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(fifo_by_origin.len(), thread_count);

    let expected_fifo_sum: usize = fifo_records.iter().map(|record| record.derived).sum();
    let scoped_followups = Mutex::new(Vec::<ScopedFollowupRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        let body_index = rayon_core::current_thread_index()
            .expect("ThreadPool::scope body should run inside the custom pool");
        assert!(body_index < thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        for fifo_record in fifo_records.iter().cloned() {
            let scoped_followups = &scoped_followups;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped follow-up work should run on a Rayon worker");

                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let origin_index = fifo_record.origin_index;
                let fifo_executing_index = fifo_record.executing_index;
                let derived = fifo_record.derived;

                let (left, right) =
                    rayon_core::join(move || derived * 2, move || origin_index + executing_index);

                scoped_followups
                    .lock()
                    .expect("scoped follow-up mutex should not be poisoned")
                    .push(ScopedFollowupRecord {
                        origin_index,
                        fifo_executing_index,
                        executing_index,
                        combined: left + right,
                    });
            });
        }

        expected_fifo_sum
    });

    assert_eq!(scope_return, expected_fifo_sum);

    let mut followup_records = scoped_followups
        .into_inner()
        .expect("scoped follow-up mutex should not be poisoned");
    followup_records.sort_by_key(|record| record.origin_index);

    assert_eq!(followup_records.len(), thread_count);
    assert_eq!(
        followup_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &followup_records {
        assert!(record.origin_index < thread_count);
        assert!(record.fifo_executing_index < thread_count);
        assert!(record.executing_index < thread_count);

        let fifo_record = fifo_by_origin
            .get(&record.origin_index)
            .expect("follow-up record should correspond to a FIFO record");

        assert_eq!(record.fifo_executing_index, fifo_record.executing_index);
        assert_eq!(
            record.combined,
            fifo_record.derived * 2 + record.origin_index + record.executing_index
        );
    }

    let (observed_followup_sum, recomputed_followup_sum) = rayon_core::ThreadPool::join(
        &pool,
        || followup_records.iter().map(|record| record.combined).sum::<usize>(),
        || {
            followup_records
                .iter()
                .map(|record| {
                    let fifo_record = fifo_by_origin
                        .get(&record.origin_index)
                        .expect("FIFO record should exist during recomputation");

                    fifo_record.derived * 2 + record.origin_index + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_followup_sum, recomputed_followup_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_spawn_fifo_detached_panics_use_configured_handler_and_pool_recovers() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (panic_tx, panic_rx) = mpsc::channel::<HandledFifoPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("spawn-fifo-panic-worker-{index}"))
        .panic_handler({
            let panic_tx = Arc::clone(&panic_tx);

            move |payload| {
                let event = classify_payload(&*payload);
                if let Ok(sender) = panic_tx.lock() {
                    let _ = sender.send(event);
                }
            }
        })
        .build()
        .expect("custom pool with panic handler should build");

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 5) * (num_threads + 43),
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

    let seed_by_origin: BTreeMap<usize, usize> = seeds
        .iter()
        .map(|record| (record.index, record.seed))
        .collect();

    let panic_started = Arc::new(AtomicUsize::new(0));

    for seed_record in seeds.iter().cloned() {
        let panic_started = Arc::clone(&panic_started);

        rayon_core::ThreadPool::spawn_fifo(&pool, move || {
            panic_started.fetch_add(1, Ordering::SeqCst);

            let executing_index = rayon_core::current_thread_index()
                .expect("panicking spawn_fifo work should execute on a Rayon worker");

            assert!(executing_index < thread_count);
            assert_eq!(rayon_core::current_num_threads(), thread_count);
            assert!(
                rayon_core::current_thread_has_pending_tasks().is_some(),
                "detached FIFO work should observe worker-local pending-task status"
            );

            let origin_index = seed_record.index;
            let seed = seed_record.seed;

            let (left, right) =
                rayon_core::join(move || seed + origin_index, move || thread_count * 100 + executing_index);

            std::panic::panic_any(FifoPanicRecord {
                origin_index,
                seed,
                executing_index,
                num_threads: thread_count,
                checksum: left + right,
            });
        });
    }

    let events = recv_exact(
        &panic_rx,
        thread_count,
        "ThreadPool::spawn_fifo panic handler",
    );

    assert_eq!(panic_started.load(Ordering::SeqCst), thread_count);

    let mut panic_records = Vec::new();

    for event in events {
        match event {
            HandledFifoPanic::Record(record) => panic_records.push(record),
            unexpected => panic!("unexpected panic handler event: {unexpected:?}"),
        }
    }

    panic_records.sort_by_key(|record| record.origin_index);

    assert_eq!(
        panic_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &panic_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );
        assert_eq!(
            record.checksum,
            record.seed + record.origin_index + thread_count * 100 + record.executing_index
        );
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should receive exactly one payload per panicking FIFO task"
    );

    let records_by_origin: BTreeMap<usize, FifoPanicRecord> = panic_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(records_by_origin.len(), thread_count);

    let (recovery_tx, recovery_rx) = mpsc::channel::<FifoRecoveryRecord>();
    let recovery_started = Arc::new(AtomicUsize::new(0));

    for panic_record in panic_records.iter().cloned() {
        let recovery_tx = recovery_tx.clone();
        let recovery_started = Arc::clone(&recovery_started);

        rayon_core::ThreadPool::spawn_fifo(&pool, move || {
            recovery_started.fetch_add(1, Ordering::SeqCst);

            let executing_index = rayon_core::current_thread_index()
                .expect("recovery spawn_fifo work should execute on a Rayon worker");

            assert!(executing_index < panic_record.num_threads);
            assert_eq!(rayon_core::current_num_threads(), panic_record.num_threads);

            let origin_index = panic_record.origin_index;
            let seed = panic_record.seed;
            let panic_checksum = panic_record.checksum;

            let (left, right) =
                rayon_core::join(move || seed + origin_index, move || panic_checksum + executing_index);

            recovery_tx
                .send(FifoRecoveryRecord {
                    origin_index,
                    seed,
                    panic_checksum,
                    executing_index,
                    value: left + right,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                })
                .expect("recovery FIFO work should report successfully");
        });
    }

    drop(recovery_tx);

    let mut recovery_records = recv_exact(
        &recovery_rx,
        thread_count,
        "ThreadPool::spawn_fifo recovery work",
    );
    recovery_records.sort_by_key(|record| record.origin_index);

    assert_eq!(recovery_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        recovery_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &recovery_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );

        let panic_record = records_by_origin
            .get(&record.origin_index)
            .expect("recovery record should correspond to a handled panic record");

        assert_eq!(record.panic_checksum, panic_record.checksum);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + record.panic_checksum + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "recovery FIFO work should still run inside Rayon workers"
        );
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "non-panicking recovery FIFO work should not invoke the panic handler"
    );

    let mut after_recovery = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, index + num_threads * 10)
    });

    after_recovery.sort_by_key(|record| record.0);

    assert_eq!(
        after_recovery
            .iter()
            .map(|(index, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, value) in after_recovery {
        assert_eq!(value, index + thread_count * 10);
    }
}