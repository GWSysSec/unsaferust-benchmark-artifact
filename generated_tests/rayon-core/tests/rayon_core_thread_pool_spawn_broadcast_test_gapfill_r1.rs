use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScheduleReport {
    worker_index: usize,
    num_threads: usize,
    pending_status_available: bool,
    seed_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AsyncBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: Option<String>,
    seed: usize,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FollowUpRecord {
    origin_index: usize,
    seed: usize,
    async_value: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfirmationRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SpawnBroadcastPanicRecord {
    index: usize,
    num_threads: usize,
    seed: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HandledPanic {
    Broadcast(SpawnBroadcastPanicRecord),
    Message(String),
    Unexpected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryAsyncRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryScopedRecord {
    origin_index: usize,
    seed: usize,
    panic_checksum: usize,
    executing_index: usize,
    value: usize,
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

fn classify_payload(payload: &(dyn Any + Send)) -> HandledPanic {
    if let Some(record) = payload.downcast_ref::<SpawnBroadcastPanicRecord>() {
        HandledPanic::Broadcast(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledPanic::Message(message.clone())
    } else {
        HandledPanic::Unexpected
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_spawn_broadcast_from_custom_pool_worker_feeds_scoped_and_pool_method_broadcast_work() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("target-thread-pool-spawn-broadcast-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

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
            seed: (index + 5) * (num_threads + 31),
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
        assert_eq!(record.seed, (record.index + 5) * (thread_count + 31));
    }

    let seed_by_index = Arc::new(seeds.iter().map(|record| record.seed).collect::<Vec<_>>());
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let (async_tx, async_rx) = mpsc::channel::<AsyncBroadcastRecord>();
    let run_count = Arc::new(AtomicUsize::new(0));

    let schedule_report = rayon_core::ThreadPool::scope(&pool, |_| {
        let worker_index = rayon_core::current_thread_index()
            .expect("scheduling scope body should run inside the custom pool");
        assert!(worker_index < thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        rayon_core::spawn_broadcast({
            let seed_by_index = Arc::clone(&seed_by_index);
            let run_count = Arc::clone(&run_count);

            move |context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(
                    num_threads, thread_count,
                    "free spawn_broadcast called from a pool worker should use that current pool"
                );
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let worker_name = std::thread::current().name().map(str::to_owned);
                assert_eq!(
                    worker_name.as_deref(),
                    Some(
                        format!("target-thread-pool-spawn-broadcast-worker-{index}").as_str()
                    )
                );

                let prior_runs = run_count.fetch_add(1, Ordering::SeqCst);
                assert!(
                    prior_runs < num_threads,
                    "spawn_broadcast should run at most once per worker"
                );

                let seed = seed_by_index[index];
                let (left, right) =
                    rayon_core::join(move || seed + index, move || num_threads * 10);

                async_tx
                    .send(AsyncBroadcastRecord {
                        index,
                        num_threads,
                        current_index: rayon_core::current_thread_index(),
                        worker_name,
                        seed,
                        derived: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    })
                    .expect("spawn_broadcast worker should report its derived value");
            }
        });

        ScheduleReport {
            worker_index,
            num_threads: rayon_core::current_num_threads(),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
            seed_sum: expected_seed_sum,
        }
    });

    assert!(schedule_report.worker_index < thread_count);
    assert_eq!(schedule_report.num_threads, thread_count);
    assert!(schedule_report.pending_status_available);
    assert_eq!(schedule_report.seed_sum, expected_seed_sum);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "detached spawn_broadcast should return control to the external caller"
    );

    let mut async_records = recv_exact(
        &async_rx,
        thread_count,
        "free spawn_broadcast custom-pool batch",
    );
    async_records.sort_by_key(|record| record.index);

    assert_eq!(run_count.load(Ordering::SeqCst), thread_count);
    assert!(
        async_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "free spawn_broadcast should report exactly once per worker"
    );
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
        assert_eq!(
            record.worker_name.as_deref(),
            Some(
                format!(
                    "target-thread-pool-spawn-broadcast-worker-{}",
                    record.index
                )
                .as_str()
            )
        );
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.derived,
            seed_by_index[record.index] + record.index + thread_count * 10
        );
        assert!(
            record.pending_status_available,
            "spawn_broadcast work should observe worker-local pending-task status"
        );
    }

    let expected_async_sum: usize = async_records.iter().map(|record| record.derived).sum();
    let followups = Mutex::new(Vec::<FollowUpRecord>::new());

    let scope_fifo_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for record in async_records.iter().cloned() {
            let followups_ref = &followups;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO follow-up work should run in the custom pool");
                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let origin_index = record.index;
                let seed = record.seed;
                let async_value = record.derived;

                let (left, right) = rayon_core::join(
                    move || async_value + seed,
                    move || origin_index + executing_index + thread_count,
                );

                followups_ref
                    .lock()
                    .expect("follow-up record mutex should not be poisoned")
                    .push(FollowUpRecord {
                        origin_index,
                        seed,
                        async_value,
                        executing_index,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        expected_async_sum
    });

    assert_eq!(scope_fifo_return, expected_async_sum);

    let mut followups = followups
        .into_inner()
        .expect("follow-up record mutex should not be poisoned");
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
        assert!(record.executing_index < thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.async_value,
            async_records[record.origin_index].derived
        );
        assert_eq!(
            record.value,
            record.async_value
                + record.seed
                + record.origin_index
                + record.executing_index
                + thread_count
        );
        assert!(
            record.pending_status_available,
            "scoped FIFO follow-up should observe pending-task status"
        );
    }

    let async_by_index: Arc<BTreeMap<usize, AsyncBroadcastRecord>> = Arc::new(
        async_records
            .iter()
            .cloned()
            .map(|record| (record.index, record))
            .collect(),
    );
    let followup_by_index: Arc<BTreeMap<usize, FollowUpRecord>> = Arc::new(
        followups
            .iter()
            .cloned()
            .map(|record| (record.origin_index, record))
            .collect(),
    );

    assert_eq!(async_by_index.len(), thread_count);
    assert_eq!(followup_by_index.len(), thread_count);

    let (confirm_tx, confirm_rx) = mpsc::channel::<ConfirmationRecord>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let async_by_index = Arc::clone(&async_by_index);
        let followup_by_index = Arc::clone(&followup_by_index);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let async_value = async_by_index
                .get(&index)
                .expect("confirmation should find async output")
                .derived;
            let followup_value = followup_by_index
                .get(&index)
                .expect("confirmation should find follow-up output")
                .value;

            let (left, right) =
                rayon_core::join(move || async_value, move || followup_value + index);

            confirm_tx
                .send(ConfirmationRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    total: left + right,
                })
                .expect("ThreadPool::spawn_broadcast confirmation should report");
        }
    });

    let mut confirmations = recv_exact(
        &confirm_rx,
        thread_count,
        "ThreadPool::spawn_broadcast confirmation batch",
    );
    confirmations.sort_by_key(|record| record.index);

    assert!(
        confirm_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "ThreadPool::spawn_broadcast should confirm exactly once per worker"
    );
    assert_eq!(
        confirmations
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &confirmations {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(
            record.total,
            async_by_index[&record.index].derived
                + followup_by_index[&record.index].value
                + record.index
        );
    }

    let (observed_followup_sum, recomputed_followup_sum) = rayon_core::ThreadPool::join(
        &pool,
        || followups.iter().map(|record| record.value).sum::<usize>(),
        || {
            followups
                .iter()
                .map(|record| {
                    record.async_value
                        + record.seed
                        + record.origin_index
                        + record.executing_index
                        + thread_count
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_followup_sum, recomputed_followup_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_spawn_broadcast_detached_panics_use_current_pool_handler_then_recovery_work_succeeds() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (panic_tx, panic_rx) = mpsc::channel::<HandledPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("target-thread-pool-spawn-broadcast-panic-worker-{index}"))
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
        .expect("custom Rayon pool with panic handler should build");

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
            seed: (index + 7) * (num_threads + 43),
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

    let seed_by_index = Arc::new(seeds.iter().map(|record| record.seed).collect::<Vec<_>>());

    let panic_schedule_report = rayon_core::ThreadPool::scope(&pool, |_| {
        let worker_index = rayon_core::current_thread_index()
            .expect("panic scheduling scope should run inside the custom pool");
        assert!(worker_index < thread_count);

        rayon_core::spawn_broadcast({
            let seed_by_index = Arc::clone(&seed_by_index);

            move |context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(
                    num_threads, thread_count,
                    "panicking free spawn_broadcast should use the current custom pool"
                );
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let seed = seed_by_index[index];
                let (left, right) =
                    rayon_core::join(move || seed + index, move || num_threads * 100);

                std::panic::panic_any(SpawnBroadcastPanicRecord {
                    index,
                    num_threads,
                    seed,
                    checksum: left + right,
                });
            }
        });

        ScheduleReport {
            worker_index,
            num_threads: rayon_core::current_num_threads(),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
            seed_sum: seed_by_index.iter().sum(),
        }
    });

    assert!(panic_schedule_report.worker_index < thread_count);
    assert_eq!(panic_schedule_report.num_threads, thread_count);
    assert!(panic_schedule_report.pending_status_available);

    let panic_events = recv_exact(
        &panic_rx,
        thread_count,
        "panic handler for free spawn_broadcast",
    );

    let mut observed_panics = BTreeSet::<SpawnBroadcastPanicRecord>::new();

    for event in panic_events {
        match event {
            HandledPanic::Broadcast(record) => {
                assert!(
                    observed_panics.insert(record),
                    "each panicking broadcast worker should be handled exactly once"
                );
            }
            unexpected => panic!("unexpected panic handler event: {unexpected:?}"),
        }
    }

    let expected_panics: BTreeSet<_> = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| SpawnBroadcastPanicRecord {
            index,
            num_threads: thread_count,
            seed: *seed,
            checksum: *seed + index + thread_count * 100,
        })
        .collect();

    assert_eq!(observed_panics, expected_panics);
    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should receive exactly the scheduled broadcast panics"
    );

    let panic_checksum_by_index: BTreeMap<usize, usize> = observed_panics
        .iter()
        .map(|record| (record.index, record.checksum))
        .collect();

    let (recovery_tx, recovery_rx) = mpsc::channel::<RecoveryAsyncRecord>();

    let recovery_schedule_report = rayon_core::ThreadPool::scope(&pool, |_| {
        let worker_index = rayon_core::current_thread_index()
            .expect("recovery scheduling scope should run inside the custom pool");
        assert!(worker_index < thread_count);

        rayon_core::spawn_broadcast({
            let seed_by_index = Arc::clone(&seed_by_index);

            move |context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(
                    num_threads, thread_count,
                    "recovery free spawn_broadcast should still use the custom pool"
                );
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let seed = seed_by_index[index];
                let (left, right) =
                    rayon_core::join(move || seed * 2 + index, move || num_threads * 5);

                recovery_tx
                    .send(RecoveryAsyncRecord {
                        index,
                        num_threads,
                        current_index: rayon_core::current_thread_index(),
                        seed,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    })
                    .expect("recovery spawn_broadcast worker should report");
            }
        });

        ScheduleReport {
            worker_index,
            num_threads: rayon_core::current_num_threads(),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
            seed_sum: seed_by_index.iter().sum(),
        }
    });

    assert!(recovery_schedule_report.worker_index < thread_count);
    assert_eq!(recovery_schedule_report.num_threads, thread_count);
    assert!(recovery_schedule_report.pending_status_available);

    let mut recovery_records = recv_exact(
        &recovery_rx,
        thread_count,
        "recovery free spawn_broadcast batch",
    );
    recovery_records.sort_by_key(|record| record.index);

    assert_eq!(
        recovery_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &recovery_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.value,
            seed_by_index[record.index] * 2 + record.index + thread_count * 5
        );
        assert!(
            record.pending_status_available,
            "recovery spawn_broadcast should run on Rayon workers"
        );
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "non-panicking recovery broadcast should not invoke the panic handler"
    );

    let expected_recovery_sum: usize = recovery_records.iter().map(|record| record.value).sum();
    let scoped_recovery = Mutex::new(Vec::<RecoveryScopedRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for record in recovery_records.iter().cloned() {
            let scoped_recovery_ref = &scoped_recovery;
            let panic_checksum = panic_checksum_by_index[&record.index];

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("post-panic scoped recovery should run inside the custom pool");
                assert!(executing_index < thread_count);

                let origin_index = record.index;
                let seed = record.seed;
                let value = record.value;

                let (left, right) = rayon_core::join(
                    move || value + panic_checksum,
                    move || seed + origin_index + executing_index + thread_count,
                );

                scoped_recovery_ref
                    .lock()
                    .expect("scoped recovery mutex should not be poisoned")
                    .push(RecoveryScopedRecord {
                        origin_index,
                        seed,
                        panic_checksum,
                        executing_index,
                        value: left + right,
                    });
            });
        }

        expected_recovery_sum
    });

    assert_eq!(scope_return, expected_recovery_sum);

    let mut scoped_recovery = scoped_recovery
        .into_inner()
        .expect("scoped recovery mutex should not be poisoned");
    scoped_recovery.sort_by_key(|record| record.origin_index);

    assert_eq!(scoped_recovery.len(), thread_count);
    assert_eq!(
        scoped_recovery
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let recovery_value_by_index: BTreeMap<usize, usize> = recovery_records
        .iter()
        .map(|record| (record.index, record.value))
        .collect();

    for record in &scoped_recovery {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.panic_checksum,
            panic_checksum_by_index[&record.origin_index]
        );
        assert_eq!(
            record.value,
            recovery_value_by_index[&record.origin_index]
                + record.panic_checksum
                + record.seed
                + record.origin_index
                + record.executing_index
                + thread_count
        );
    }

    let (observed_scoped_sum, recomputed_scoped_sum) = rayon_core::ThreadPool::join(
        &pool,
        || scoped_recovery.iter().map(|record| record.value).sum::<usize>(),
        || {
            scoped_recovery
                .iter()
                .map(|record| {
                    recovery_value_by_index[&record.origin_index]
                        + panic_checksum_by_index[&record.origin_index]
                        + seed_by_index[record.origin_index]
                        + record.origin_index
                        + record.executing_index
                        + thread_count
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_scoped_sum, recomputed_scoped_sum);

    let mut final_check = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, num_threads, index + num_threads)
    });

    final_check.sort_by_key(|record| record.0);

    assert_eq!(
        final_check
            .iter()
            .map(|(index, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, num_threads, value) in final_check {
        assert_eq!(num_threads, thread_count);
        assert_eq!(value, index + thread_count);
    }
}