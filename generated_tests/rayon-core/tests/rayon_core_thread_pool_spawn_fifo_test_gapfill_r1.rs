use std::any::Any;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScheduleSummary {
    scheduling_worker: usize,
    num_threads: usize,
    pending_status_available: bool,
    scheduled_jobs: usize,
    seed_checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FreeFifoRecord {
    queue_position: usize,
    seed: usize,
    worker_index: Option<usize>,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedFollowupRecord {
    queue_position: usize,
    source_value: usize,
    executing_index: usize,
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
struct MethodFifoRecord {
    queue_position: usize,
    worker_index: Option<usize>,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MethodFifoPanicRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HandledFifoPanic {
    Record(MethodFifoPanicRecord),
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
struct RecoveryFollowupRecord {
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
    if let Some(record) = payload.downcast_ref::<MethodFifoPanicRecord>() {
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
fn free_spawn_fifo_from_current_single_worker_pool_preserves_fifo_order_and_feeds_scoped_work() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("target-free-spawn-fifo-current-pool-{index}"))
        .build()
        .expect("single-worker custom Rayon pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "external integration-test thread should not be a pool worker"
    );
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
            seed: (index + 7) * (num_threads + 19),
        }
    });

    seeds.sort_by_key(|record| record.index);

    assert_eq!(seeds.len(), 1);
    assert_eq!(
        seeds
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(1)
    );
    assert_eq!(seeds[0].current_index, Some(0));
    assert_eq!(seeds[0].seed, 7 * 20);

    let scheduled_jobs = 10usize;
    let seed = seeds[0].seed;
    let (fifo_tx, fifo_rx) = mpsc::channel::<FreeFifoRecord>();
    let fifo_tx_for_schedule = fifo_tx.clone();

    let summary = rayon_core::ThreadPool::scope(&pool, move |_| {
        let scheduling_worker = rayon_core::current_thread_index()
            .expect("ThreadPool::scope body should run inside the custom pool");
        assert_eq!(scheduling_worker, 0);
        assert_eq!(rayon_core::current_num_threads(), 1);

        for queue_position in 0usize..scheduled_jobs {
            let fifo_tx = fifo_tx_for_schedule.clone();

            rayon_core::spawn_fifo(move || {
                assert_eq!(rayon_core::current_thread_index(), Some(0));
                assert_eq!(rayon_core::current_num_threads(), 1);

                let pending_status_available =
                    rayon_core::current_thread_has_pending_tasks().is_some();

                let (left, right) = rayon_core::join(
                    move || seed + queue_position,
                    move || queue_position * 10 + rayon_core::current_num_threads(),
                );

                fifo_tx
                    .send(FreeFifoRecord {
                        queue_position,
                        seed,
                        worker_index: rayon_core::current_thread_index(),
                        num_threads: rayon_core::current_num_threads(),
                        value: left + right,
                        pending_status_available,
                    })
                    .expect("free spawn_fifo worker should report its result");
            });
        }

        ScheduleSummary {
            scheduling_worker,
            num_threads: rayon_core::current_num_threads(),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
            scheduled_jobs,
            seed_checksum: seed * scheduled_jobs,
        }
    });

    drop(fifo_tx);

    assert_eq!(summary.scheduling_worker, 0);
    assert_eq!(summary.num_threads, 1);
    assert!(summary.pending_status_available);
    assert_eq!(summary.scheduled_jobs, scheduled_jobs);
    assert_eq!(summary.seed_checksum, seed * scheduled_jobs);

    let fifo_records = recv_exact(
        &fifo_rx,
        scheduled_jobs,
        "rayon_core::spawn_fifo single-worker batch",
    );

    assert!(
        fifo_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each free spawn_fifo job should report exactly once"
    );

    let observed_order: Vec<_> = fifo_records
        .iter()
        .map(|record| record.queue_position)
        .collect();

    assert_eq!(
        observed_order,
        (0usize..scheduled_jobs).collect::<Vec<_>>(),
        "free spawn_fifo jobs queued by the same worker should run FIFO on one worker"
    );

    for (expected_position, record) in fifo_records.iter().enumerate() {
        assert_eq!(record.queue_position, expected_position);
        assert_eq!(record.seed, seed);
        assert_eq!(record.worker_index, Some(0));
        assert_eq!(record.num_threads, 1);
        assert!(
            record.pending_status_available,
            "free spawn_fifo work should observe worker-local pending-task status"
        );
        assert_eq!(
            record.value,
            seed + expected_position + expected_position * 10 + 1
        );
    }

    let expected_fifo_sum: usize = fifo_records.iter().map(|record| record.value).sum();
    let followups = Mutex::new(Vec::<ScopedFollowupRecord>::new());

    let scope_fifo_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for record in fifo_records.iter().cloned() {
            let followups_ref = &followups;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped FIFO follow-up should run inside the custom pool");
                assert_eq!(executing_index, 0);
                assert_eq!(rayon_core::current_num_threads(), record.num_threads);

                let queue_position = record.queue_position;
                let source_value = record.value;

                let (left, right) = rayon_core::join(
                    move || source_value + queue_position,
                    move || executing_index + record.num_threads,
                );

                followups_ref
                    .lock()
                    .expect("follow-up record mutex should not be poisoned")
                    .push(ScopedFollowupRecord {
                        queue_position,
                        source_value,
                        executing_index,
                        combined: left + right,
                    });
            });
        }

        expected_fifo_sum
    });

    assert_eq!(scope_fifo_return, expected_fifo_sum);

    let mut followups = followups
        .into_inner()
        .expect("follow-up record mutex should not be poisoned");
    followups.sort_by_key(|record| record.queue_position);

    assert_eq!(followups.len(), scheduled_jobs);

    for followup in &followups {
        assert_eq!(followup.executing_index, 0);
        assert_eq!(
            followup.source_value,
            fifo_records[followup.queue_position].value
        );
        assert_eq!(
            followup.combined,
            followup.source_value + followup.queue_position + 1
        );
    }

    let followup_sum: usize = followups.iter().map(|record| record.combined).sum();

    let confirmation = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(index, 0);
        assert_eq!(num_threads, 1);
        assert_eq!(rayon_core::current_thread_index(), Some(0));

        ConfirmationRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            total: expected_fifo_sum + followup_sum + num_threads,
        }
    });

    assert_eq!(
        confirmation,
        vec![ConfirmationRecord {
            index: 0,
            num_threads: 1,
            current_index: Some(0),
            total: expected_fifo_sum + followup_sum + 1,
        }]
    );

    let (observed_fifo_sum, recomputed_fifo_sum) = rayon_core::ThreadPool::join(
        &pool,
        || fifo_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            fifo_records
                .iter()
                .map(|record| record.seed + record.queue_position + record.queue_position * 10 + 1)
                .sum::<usize>()
        },
    );

    assert_eq!(observed_fifo_sum, expected_fifo_sum);
    assert_eq!(observed_fifo_sum, recomputed_fifo_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_spawn_fifo_is_detached_orders_blocked_single_worker_jobs_and_recovers_after_panic() {
    let (panic_tx, panic_rx) = mpsc::channel::<HandledFifoPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("target-thread-pool-spawn-fifo-worker-{index}"))
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
                .expect("blocking worker should report startup");

            let (lock, condvar) = &*release_worker;
            let mut released = lock
                .lock()
                .expect("release mutex should not be poisoned while worker waits");

            while !*released {
                released = condvar
                    .wait(released)
                    .expect("release mutex should not be poisoned while waiting");
            }
        }
    });

    let (blocking_index, blocking_threads) = started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("blocking worker should start before FIFO jobs are queued");

    assert_eq!(blocking_index, Some(0));
    assert_eq!(blocking_threads, 1);

    let queued_jobs = 8usize;
    let (method_tx, method_rx) = mpsc::channel::<MethodFifoRecord>();

    for queue_position in 0usize..queued_jobs {
        let method_tx = method_tx.clone();

        rayon_core::ThreadPool::spawn_fifo(&pool, move || {
            assert_eq!(rayon_core::current_thread_index(), Some(0));
            assert_eq!(rayon_core::current_num_threads(), 1);

            let pending_status_available =
                rayon_core::current_thread_has_pending_tasks().is_some();

            let (left, right) =
                rayon_core::join(move || queue_position * 2, move || queue_position * 20 + 1);

            method_tx
                .send(MethodFifoRecord {
                    queue_position,
                    worker_index: rayon_core::current_thread_index(),
                    num_threads: rayon_core::current_num_threads(),
                    value: left + right,
                    pending_status_available,
                })
                .expect("ThreadPool::spawn_fifo worker should report its result");
        });
    }

    drop(method_tx);

    assert!(
        method_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "detached ThreadPool::spawn_fifo work should not run while the only worker is blocked"
    );

    {
        let (lock, condvar) = &*release_worker;
        let mut released = lock
            .lock()
            .expect("release mutex should not be poisoned when releasing worker");
        *released = true;
        condvar.notify_one();
    }

    let method_records = recv_exact(
        &method_rx,
        queued_jobs,
        "blocked single-worker ThreadPool::spawn_fifo batch",
    );

    assert!(
        method_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each queued ThreadPool::spawn_fifo job should report exactly once"
    );

    let observed_order: Vec<_> = method_records
        .iter()
        .map(|record| record.queue_position)
        .collect();

    assert_eq!(
        observed_order,
        (0usize..queued_jobs).collect::<Vec<_>>(),
        "ThreadPool::spawn_fifo jobs queued while the worker was blocked should run FIFO"
    );

    for (expected_position, record) in method_records.iter().enumerate() {
        assert_eq!(record.queue_position, expected_position);
        assert_eq!(record.worker_index, Some(0));
        assert_eq!(record.num_threads, 1);
        assert_eq!(record.value, expected_position * 22 + 1);
        assert!(
            record.pending_status_available,
            "ThreadPool::spawn_fifo work should observe worker-local pending-task status"
        );
    }

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(index, 0);
        assert_eq!(num_threads, 1);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 5) * (num_threads + 31),
        }
    });

    seeds.sort_by_key(|record| record.index);
    assert_eq!(seeds.len(), 1);
    assert_eq!(seeds[0].seed, 5 * 32);

    let panic_seed = seeds[0].seed;

    rayon_core::ThreadPool::spawn_fifo(&pool, move || {
        let executing_index = rayon_core::current_thread_index()
            .expect("panicking ThreadPool::spawn_fifo task should run on a Rayon worker");
        assert_eq!(executing_index, 0);
        assert_eq!(rayon_core::current_num_threads(), 1);

        let (left, right) =
            rayon_core::join(move || panic_seed + executing_index, move || 1000usize);

        std::panic::panic_any(MethodFifoPanicRecord {
            origin_index: 0,
            seed: panic_seed,
            executing_index,
            num_threads: rayon_core::current_num_threads(),
            checksum: left + right,
        });
    });

    let panic_event = panic_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("ThreadPool::spawn_fifo panic should reach the configured panic handler");

    let panic_record = match panic_event {
        HandledFifoPanic::Record(record) => record,
        unexpected => panic!("unexpected ThreadPool::spawn_fifo panic handler event: {unexpected:?}"),
    };

    assert_eq!(panic_record.origin_index, 0);
    assert_eq!(panic_record.seed, panic_seed);
    assert_eq!(panic_record.executing_index, 0);
    assert_eq!(panic_record.num_threads, 1);
    assert_eq!(panic_record.checksum, panic_seed + 1000);

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should receive exactly the intentional ThreadPool::spawn_fifo panic"
    );

    let (recovery_tx, recovery_rx) = mpsc::channel::<FifoRecoveryRecord>();
    let panic_record_for_recovery = panic_record.clone();

    rayon_core::ThreadPool::spawn_fifo(&pool, move || {
        let executing_index = rayon_core::current_thread_index()
            .expect("recovery ThreadPool::spawn_fifo work should run on a Rayon worker");
        assert_eq!(executing_index, 0);
        assert_eq!(rayon_core::current_num_threads(), panic_record_for_recovery.num_threads);

        let origin_index = panic_record_for_recovery.origin_index;
        let seed = panic_record_for_recovery.seed;
        let panic_checksum = panic_record_for_recovery.checksum;

        let (left, right) = rayon_core::join(
            move || panic_checksum + seed,
            move || origin_index + executing_index + 1,
        );

        recovery_tx
            .send(FifoRecoveryRecord {
                origin_index,
                seed,
                panic_checksum,
                executing_index,
                value: left + right,
                pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
            })
            .expect("recovery ThreadPool::spawn_fifo task should report");
    });

    let recovery_records = recv_exact(
        &recovery_rx,
        1,
        "post-panic ThreadPool::spawn_fifo recovery work",
    );
    let recovery = recovery_records
        .into_iter()
        .next()
        .expect("one recovery record should be present");

    assert_eq!(recovery.origin_index, 0);
    assert_eq!(recovery.seed, panic_record.seed);
    assert_eq!(recovery.panic_checksum, panic_record.checksum);
    assert_eq!(recovery.executing_index, 0);
    assert!(recovery.pending_status_available);
    assert_eq!(
        recovery.value,
        panic_record.checksum + panic_record.seed + recovery.origin_index + recovery.executing_index + 1
    );

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "successful recovery FIFO work should not invoke the panic handler"
    );

    let recovery_followups = Mutex::new(Vec::<RecoveryFollowupRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        let recovery_followups_ref = &recovery_followups;

        let origin_index = recovery.origin_index;
        let fifo_executing_index = recovery.executing_index;
        let value = recovery.value;
        let panic_checksum = recovery.panic_checksum;

        rayon_core::Scope::spawn(scope, move |_| {
            let executing_index = rayon_core::current_thread_index()
                .expect("post-recovery scoped work should run inside the custom pool");
            assert_eq!(executing_index, 0);
            assert_eq!(rayon_core::current_num_threads(), 1);

            let (left, right) = rayon_core::join(
                move || value + panic_checksum,
                move || origin_index + fifo_executing_index + executing_index,
            );

            recovery_followups_ref
                .lock()
                .expect("recovery follow-up mutex should not be poisoned")
                .push(RecoveryFollowupRecord {
                    origin_index,
                    fifo_executing_index,
                    executing_index,
                    combined: left + right,
                });
        });

        recovery.value
    });

    assert_eq!(scope_return, recovery.value);

    let followups = recovery_followups
        .into_inner()
        .expect("recovery follow-up mutex should not be poisoned");
    assert_eq!(followups.len(), 1);

    let followup = &followups[0];
    assert_eq!(followup.origin_index, 0);
    assert_eq!(followup.fifo_executing_index, recovery.executing_index);
    assert_eq!(followup.executing_index, 0);
    assert_eq!(
        followup.combined,
        recovery.value
            + recovery.panic_checksum
            + followup.origin_index
            + followup.fifo_executing_index
            + followup.executing_index
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