use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleWorkerFifoEvent {
    queue_position: usize,
    worker_index: Option<usize>,
    num_threads: usize,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleWorkerFollowup {
    queue_position: usize,
    executing_index: usize,
    source_value: usize,
    combined: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfirmationRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScheduleReport {
    scheduling_worker: usize,
    num_threads: usize,
    pending_status_available: bool,
    scheduled_jobs: usize,
    seed_checksum: usize,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryFollowup {
    origin_index: usize,
    fifo_executing_index: usize,
    executing_index: usize,
    combined: usize,
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
fn free_spawn_fifo_from_single_worker_current_pool_preserves_fifo_order_and_feeds_scoped_pipeline()
{
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("free-spawn-fifo-single-worker-current-pool-{index}"))
        .build()
        .expect("single-worker custom pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(index, 0);
        assert_eq!(num_threads, 1);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), num_threads);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 11) * (num_threads + 17),
        }
    });

    seeds.sort_by_key(|record| record.index);
    assert_eq!(seeds.len(), 1);
    assert_eq!(seeds[0].index, 0);
    assert_eq!(seeds[0].num_threads, 1);
    assert_eq!(seeds[0].current_index, Some(0));
    assert_eq!(seeds[0].seed, 11 * 18);

    let scheduled_jobs = 10usize;
    let seed = seeds[0].seed;
    let (event_tx, event_rx) = mpsc::channel::<SingleWorkerFifoEvent>();
    let event_tx_for_schedule = event_tx.clone();

    let schedule_report = rayon_core::ThreadPool::scope(&pool, move |_| {
        let scheduling_worker = rayon_core::current_thread_index()
            .expect("ThreadPool::scope body should run inside the custom pool");
        assert_eq!(scheduling_worker, 0);
        assert_eq!(rayon_core::current_num_threads(), 1);

        for queue_position in 0usize..scheduled_jobs {
            let event_tx = event_tx_for_schedule.clone();

            rayon_core::spawn_fifo(move || {
                assert_eq!(rayon_core::current_thread_index(), Some(0));
                assert_eq!(rayon_core::current_num_threads(), 1);

                let pending_status_available =
                    rayon_core::current_thread_has_pending_tasks().is_some();

                let (left, right) = rayon_core::join(
                    move || seed + queue_position,
                    move || queue_position * 10 + rayon_core::current_num_threads(),
                );

                event_tx
                    .send(SingleWorkerFifoEvent {
                        queue_position,
                        worker_index: rayon_core::current_thread_index(),
                        num_threads: rayon_core::current_num_threads(),
                        seed,
                        value: left + right,
                        pending_status_available,
                    })
                    .expect("free spawn_fifo task should report its result");
            });
        }

        ScheduleReport {
            scheduling_worker,
            num_threads: rayon_core::current_num_threads(),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
            scheduled_jobs,
            seed_checksum: seed * scheduled_jobs,
        }
    });

    drop(event_tx);

    assert_eq!(schedule_report.scheduling_worker, 0);
    assert_eq!(schedule_report.num_threads, 1);
    assert!(schedule_report.pending_status_available);
    assert_eq!(schedule_report.scheduled_jobs, scheduled_jobs);
    assert_eq!(schedule_report.seed_checksum, seed * scheduled_jobs);

    let fifo_events = recv_exact(
        &event_rx,
        scheduled_jobs,
        "rayon_core::spawn_fifo single-worker FIFO batch",
    );

    assert!(
        event_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each scheduled free spawn_fifo task should report exactly once"
    );

    for (expected_position, event) in fifo_events.iter().enumerate() {
        assert_eq!(event.queue_position, expected_position);
        assert_eq!(event.worker_index, Some(0));
        assert_eq!(event.num_threads, 1);
        assert_eq!(event.seed, seed);
        assert_eq!(
            event.value,
            seed + expected_position + expected_position * 10 + 1
        );
        assert!(
            event.pending_status_available,
            "spawn_fifo work should be able to query worker-local pending-task status"
        );
    }

    let observed_order: Vec<_> = fifo_events
        .iter()
        .map(|event| event.queue_position)
        .collect();
    assert_eq!(
        observed_order,
        (0usize..scheduled_jobs).collect::<Vec<_>>(),
        "free spawn_fifo jobs queued by the same worker should execute in FIFO order on one worker"
    );

    let expected_fifo_sum: usize = fifo_events.iter().map(|event| event.value).sum();
    let followups = Mutex::new(Vec::<SingleWorkerFollowup>::new());

    let scope_fifo_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for event in fifo_events.iter().cloned() {
            let followups_ref = &followups;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped FIFO follow-up should run inside the custom pool");
                assert_eq!(executing_index, 0);
                assert_eq!(rayon_core::current_num_threads(), event.num_threads);

                let queue_position = event.queue_position;
                let source_value = event.value;

                let (left, right) = rayon_core::join(
                    move || source_value + queue_position,
                    move || executing_index + event.num_threads,
                );

                followups_ref
                    .lock()
                    .expect("follow-up mutex should not be poisoned")
                    .push(SingleWorkerFollowup {
                        queue_position,
                        executing_index,
                        source_value,
                        combined: left + right,
                    });
            });
        }

        expected_fifo_sum
    });

    assert_eq!(scope_fifo_return, expected_fifo_sum);

    let mut followups = followups
        .into_inner()
        .expect("follow-up mutex should not be poisoned");
    followups.sort_by_key(|record| record.queue_position);

    assert_eq!(followups.len(), scheduled_jobs);

    for followup in &followups {
        assert_eq!(followup.executing_index, 0);
        assert_eq!(followup.source_value, fifo_events[followup.queue_position].value);
        assert_eq!(
            followup.combined,
            followup.source_value + followup.queue_position + 1
        );
    }

    let expected_followup_sum: usize = followups.iter().map(|record| record.combined).sum();

    let confirmation = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(index, 0);
        assert_eq!(num_threads, 1);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        ConfirmationRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            total: expected_fifo_sum + expected_followup_sum + num_threads,
        }
    });

    assert_eq!(
        confirmation,
        vec![ConfirmationRecord {
            index: 0,
            num_threads: 1,
            current_index: Some(0),
            total: expected_fifo_sum + expected_followup_sum + 1,
        }]
    );

    let (observed_total, recomputed_total) = rayon_core::ThreadPool::join(
        &pool,
        || fifo_events.iter().map(|event| event.value).sum::<usize>(),
        || {
            fifo_events
                .iter()
                .map(|event| event.seed + event.queue_position + event.queue_position * 10 + 1)
                .sum::<usize>()
        },
    );

    assert_eq!(observed_total, expected_fifo_sum);
    assert_eq!(observed_total, recomputed_total);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_spawn_fifo_detached_panics_use_current_pool_handler_and_recovery_work_succeeds() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (panic_tx, panic_rx) = mpsc::channel::<HandledFifoPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));
    let panic_started = Arc::new(AtomicUsize::new(0));

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("free-spawn-fifo-panic-current-pool-{index}"))
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

    let pool_ref = &pool;

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(pool_ref),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(pool_ref), None);
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
            seed: (index + 2) * (num_threads + 47),
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
        assert_eq!(record.seed, (expected_index + 2) * (thread_count + 47));
    }

    let seed_by_origin: BTreeMap<usize, usize> =
        seeds.iter().map(|record| (record.index, record.seed)).collect();

    let schedule_report = rayon_core::ThreadPool::scope(pool_ref, {
        let panic_started = Arc::clone(&panic_started);

        move |_| {
            let scheduling_worker = rayon_core::current_thread_index()
                .expect("panic scheduling scope should run inside the custom pool");
            assert!(scheduling_worker < thread_count);

            for seed_record in seeds.iter().cloned() {
                let panic_started = Arc::clone(&panic_started);

                rayon_core::spawn_fifo(move || {
                    panic_started.fetch_add(1, Ordering::SeqCst);

                    let executing_index = rayon_core::current_thread_index()
                        .expect("panicking free spawn_fifo work should run on a Rayon worker");
                    assert!(executing_index < thread_count);
                    assert_eq!(rayon_core::current_num_threads(), thread_count);
                    assert!(
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                        "detached FIFO work should observe worker-local pending-task status"
                    );

                    let origin_index = seed_record.index;
                    let seed = seed_record.seed;
                    let num_threads = seed_record.num_threads;

                    let (left, right) = rayon_core::join(
                        move || seed + origin_index,
                        move || num_threads * 100 + executing_index,
                    );

                    std::panic::panic_any(FifoPanicRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        checksum: left + right,
                    });
                });
            }

            ScheduleReport {
                scheduling_worker,
                num_threads: rayon_core::current_num_threads(),
                pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
                scheduled_jobs: seeds.len(),
                seed_checksum: seeds.iter().map(|record| record.seed).sum(),
            }
        }
    });

    assert!(schedule_report.scheduling_worker < thread_count);
    assert_eq!(schedule_report.num_threads, thread_count);
    assert!(schedule_report.pending_status_available);
    assert_eq!(schedule_report.scheduled_jobs, thread_count);
    assert_eq!(
        schedule_report.seed_checksum,
        seed_by_origin.values().copied().sum::<usize>()
    );

    let panic_events = recv_exact(
        &panic_rx,
        thread_count,
        "panic handler for rayon_core::spawn_fifo",
    );

    assert_eq!(panic_started.load(Ordering::SeqCst), thread_count);

    let mut panic_records = Vec::<FifoPanicRecord>::new();

    for event in panic_events {
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
        "panic handler should receive exactly one payload per panicking free spawn_fifo task"
    );

    let records_by_origin: BTreeMap<usize, FifoPanicRecord> = panic_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

    let (recovery_tx, recovery_rx) = mpsc::channel::<FifoRecoveryRecord>();
    let recovery_started = Arc::new(AtomicUsize::new(0));
    let panic_records_for_recovery = panic_records.clone();
    let recovery_tx_for_schedule = recovery_tx.clone();

    let recovery_schedule_report = rayon_core::ThreadPool::scope(pool_ref, {
        let recovery_started = Arc::clone(&recovery_started);

        move |_| {
            let scheduling_worker = rayon_core::current_thread_index()
                .expect("recovery scheduling scope should run inside the custom pool");
            assert!(scheduling_worker < thread_count);

            for panic_record in panic_records_for_recovery.iter().cloned() {
                let recovery_tx = recovery_tx_for_schedule.clone();
                let recovery_started = Arc::clone(&recovery_started);

                rayon_core::spawn_fifo(move || {
                    recovery_started.fetch_add(1, Ordering::SeqCst);

                    let executing_index = rayon_core::current_thread_index()
                        .expect("recovery free spawn_fifo work should run on a Rayon worker");
                    assert!(executing_index < panic_record.num_threads);
                    assert_eq!(rayon_core::current_num_threads(), panic_record.num_threads);

                    let origin_index = panic_record.origin_index;
                    let seed = panic_record.seed;
                    let panic_checksum = panic_record.checksum;

                    let (left, right) = rayon_core::join(
                        move || seed + origin_index,
                        move || panic_checksum + executing_index,
                    );

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
                        .expect("recovery free spawn_fifo task should report successfully");
                });
            }

            ScheduleReport {
                scheduling_worker,
                num_threads: rayon_core::current_num_threads(),
                pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
                scheduled_jobs: panic_records_for_recovery.len(),
                seed_checksum: panic_records_for_recovery
                    .iter()
                    .map(|record| record.seed)
                    .sum(),
            }
        }
    });

    drop(recovery_tx);

    assert!(recovery_schedule_report.scheduling_worker < thread_count);
    assert_eq!(recovery_schedule_report.num_threads, thread_count);
    assert!(recovery_schedule_report.pending_status_available);
    assert_eq!(recovery_schedule_report.scheduled_jobs, thread_count);
    assert_eq!(
        recovery_schedule_report.seed_checksum,
        panic_records.iter().map(|record| record.seed).sum::<usize>()
    );

    let mut recovery_records = recv_exact(
        &recovery_rx,
        thread_count,
        "post-panic recovery rayon_core::spawn_fifo batch",
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
        assert!(
            record.pending_status_available,
            "recovery FIFO work should observe worker-local pending-task status"
        );

        let panic_record = records_by_origin
            .get(&record.origin_index)
            .expect("recovery record should correspond to a handled panic");

        assert_eq!(record.panic_checksum, panic_record.checksum);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + record.panic_checksum + record.executing_index
        );
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "non-panicking recovery spawn_fifo work should not invoke the panic handler"
    );

    let recovery_value_sum: usize = recovery_records.iter().map(|record| record.value).sum();
    let recovery_by_origin: BTreeMap<usize, FifoRecoveryRecord> = recovery_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

    let followups = Mutex::new(Vec::<RecoveryFollowup>::new());

    let scoped_return = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
        for record in recovery_records.iter().cloned() {
            let followups_ref = &followups;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped FIFO follow-up should run in the custom pool");
                assert!(executing_index < thread_count);

                let origin_index = record.origin_index;
                let fifo_executing_index = record.executing_index;

                let (left, right) = rayon_core::join(
                    move || record.value + record.panic_checksum,
                    move || origin_index + executing_index + fifo_executing_index,
                );

                followups_ref
                    .lock()
                    .expect("recovery follow-up mutex should not be poisoned")
                    .push(RecoveryFollowup {
                        origin_index,
                        fifo_executing_index,
                        executing_index,
                        combined: left + right,
                    });
            });
        }

        recovery_value_sum
    });

    assert_eq!(scoped_return, recovery_value_sum);

    let mut followups = followups
        .into_inner()
        .expect("recovery follow-up mutex should not be poisoned");
    followups.sort_by_key(|record| record.origin_index);

    assert_eq!(followups.len(), thread_count);
    assert_eq!(
        followups
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &followups {
        assert!(record.origin_index < thread_count);
        assert!(record.fifo_executing_index < thread_count);
        assert!(record.executing_index < thread_count);

        let recovery = recovery_by_origin
            .get(&record.origin_index)
            .expect("follow-up should correspond to a recovery record");

        assert_eq!(record.fifo_executing_index, recovery.executing_index);
        assert_eq!(
            record.combined,
            recovery.value
                + recovery.panic_checksum
                + record.origin_index
                + record.executing_index
                + record.fifo_executing_index
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || followups.iter().map(|record| record.combined).sum::<usize>(),
        || {
            followups
                .iter()
                .map(|record| {
                    let recovery = recovery_by_origin
                        .get(&record.origin_index)
                        .expect("recovery record should exist during recomputation");

                    recovery.value
                        + recovery.panic_checksum
                        + record.origin_index
                        + record.executing_index
                        + record.fifo_executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}