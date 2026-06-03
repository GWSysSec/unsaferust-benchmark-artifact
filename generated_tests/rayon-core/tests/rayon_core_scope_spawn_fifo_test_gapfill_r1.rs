use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Mutex, Once};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlobalFifoEvent {
    queue_position: usize,
    worker_index: Option<usize>,
    num_threads: usize,
    pending_status_available: bool,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlobalScopedFollowup {
    queue_position: usize,
    executing_index: usize,
    source_value: usize,
    combined: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GlobalFifoPanic {
    queue_position: usize,
    input: usize,
    worker_index: usize,
    num_threads: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HandledGlobalPanic {
    Fifo(GlobalFifoPanic),
    Message(String),
    Unexpected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlobalRecoveryRecord {
    queue_position: usize,
    input: usize,
    panic_checksum: usize,
    executing_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlobalRecoveryFollowup {
    queue_position: usize,
    executing_index: usize,
    value: usize,
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
    scheduling_threads: usize,
    pending_status_available: bool,
    scheduled_jobs: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomPoolFifoRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: Option<String>,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomPoolFollowup {
    origin_index: usize,
    fifo_executing_index: usize,
    executing_index: usize,
    combined: usize,
}

static TEST_MUTEX: Mutex<()> = Mutex::new(());
static GLOBAL_INIT: Once = Once::new();
static GLOBAL_INIT_STATUS: AtomicUsize = AtomicUsize::new(0);
static PANIC_SENDER: Mutex<Option<mpsc::Sender<HandledGlobalPanic>>> = Mutex::new(None);

fn classify_payload(payload: &(dyn Any + Send)) -> HandledGlobalPanic {
    if let Some(record) = payload.downcast_ref::<GlobalFifoPanic>() {
        HandledGlobalPanic::Fifo(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledGlobalPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledGlobalPanic::Message(message.clone())
    } else {
        HandledGlobalPanic::Unexpected
    }
}

fn ensure_single_worker_global_pool() {
    GLOBAL_INIT.call_once(|| {
        let builder = rayon_core::ThreadPoolBuilder::new()
            .num_threads(1)
            .thread_name(|index| format!("free-spawn-fifo-global-worker-{index}"))
            .panic_handler(|payload| {
                let event = classify_payload(&*payload);
                if let Ok(sender_guard) = PANIC_SENDER.lock() {
                    if let Some(sender) = sender_guard.as_ref() {
                        let _ = sender.send(event);
                    }
                }
            });

        let result = rayon_core::ThreadPoolBuilder::build_global(builder);
        GLOBAL_INIT_STATUS.store(if result.is_ok() { 1 } else { 2 }, Ordering::SeqCst);
    });

    assert_eq!(
        GLOBAL_INIT_STATUS.load(Ordering::SeqCst),
        1,
        "global Rayon pool should be initialized exactly once for these spawn_fifo tests"
    );
    assert_eq!(rayon_core::current_num_threads(), 1);
    assert!(rayon_core::current_num_threads() <= rayon_core::max_num_threads());
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
fn free_spawn_fifo_runs_static_jobs_in_fifo_order_and_feeds_scoped_pipeline() {
    let _serial = TEST_MUTEX
        .lock()
        .expect("test serialization mutex should not be poisoned");
    ensure_single_worker_global_pool();

    let (event_tx, event_rx) = mpsc::channel::<GlobalFifoEvent>();
    let scheduled_jobs = 8usize;

    for queue_position in 0..scheduled_jobs {
        let event_tx = event_tx.clone();

        rayon_core::spawn_fifo(move || {
            assert_eq!(rayon_core::current_num_threads(), 1);
            assert_eq!(rayon_core::current_thread_index(), Some(0));

            let pending_status_available =
                rayon_core::current_thread_has_pending_tasks().is_some();

            let (left, right) = rayon_core::join(
                move || queue_position + 3,
                move || queue_position * 10 + rayon_core::current_num_threads(),
            );

            event_tx
                .send(GlobalFifoEvent {
                    queue_position,
                    worker_index: rayon_core::current_thread_index(),
                    num_threads: rayon_core::current_num_threads(),
                    pending_status_available,
                    value: left + right,
                })
                .expect("spawn_fifo worker should report its result");
        });
    }
    drop(event_tx);

    let events = recv_exact(&event_rx, scheduled_jobs, "rayon_core::spawn_fifo FIFO batch");
    assert!(
        event_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each free spawn_fifo job should report exactly once"
    );

    for (expected_position, event) in events.iter().enumerate() {
        assert_eq!(event.queue_position, expected_position);
        assert_eq!(event.worker_index, Some(0));
        assert_eq!(event.num_threads, 1);
        assert!(
            event.pending_status_available,
            "spawn_fifo work should run on a Rayon worker that can report pending-task status"
        );
        assert_eq!(event.value, expected_position + 3 + expected_position * 10 + 1);
    }

    let observed_order: Vec<_> = events.iter().map(|event| event.queue_position).collect();
    let expected_order: Vec<_> = (0..scheduled_jobs).collect();
    assert_eq!(
        observed_order, expected_order,
        "free spawn_fifo jobs queued by the same external thread should run FIFO on one worker"
    );

    let followups = Mutex::new(Vec::<GlobalScopedFollowup>::new());
    let expected_event_sum: usize = events.iter().map(|event| event.value).sum();

    let scope_return = rayon_core::scope_fifo(|scope| {
        for event in events.iter().cloned() {
            let followups = &followups;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped FIFO follow-up should execute on the global worker");
                assert_eq!(executing_index, 0);

                let queue_position = event.queue_position;
                let source_value = event.value;

                let (left, right) = rayon_core::join(
                    move || source_value + queue_position,
                    move || executing_index + event.num_threads,
                );

                followups
                    .lock()
                    .expect("follow-up mutex should not be poisoned")
                    .push(GlobalScopedFollowup {
                        queue_position,
                        executing_index,
                        source_value,
                        combined: left + right,
                    });
            });
        }

        expected_event_sum
    });

    assert_eq!(scope_return, expected_event_sum);

    let mut followups = followups
        .into_inner()
        .expect("follow-up mutex should not be poisoned");
    followups.sort_by_key(|record| record.queue_position);

    assert_eq!(followups.len(), scheduled_jobs);
    for record in &followups {
        assert_eq!(record.executing_index, 0);
        assert_eq!(record.source_value, events[record.queue_position].value);
        assert_eq!(
            record.combined,
            record.source_value + record.queue_position + record.executing_index + 1
        );
    }

    let scoped_total: usize = followups.iter().map(|record| record.combined).sum();
    let confirmation = rayon_core::broadcast(|context| {
        assert_eq!(rayon_core::BroadcastContext::index(&context), 0);
        assert_eq!(rayon_core::BroadcastContext::num_threads(&context), 1);
        assert_eq!(rayon_core::current_thread_index(), Some(0));

        scoped_total + expected_event_sum + rayon_core::BroadcastContext::num_threads(&context)
    });

    assert_eq!(confirmation, vec![scoped_total + expected_event_sum + 1]);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_spawn_fifo_detached_panics_reach_global_handler_and_later_fifo_work_recovers() {
    let _serial = TEST_MUTEX
        .lock()
        .expect("test serialization mutex should not be poisoned");
    ensure_single_worker_global_pool();

    let (panic_tx, panic_rx) = mpsc::channel::<HandledGlobalPanic>();
    {
        let mut sender_guard = PANIC_SENDER
            .lock()
            .expect("panic sender mutex should not be poisoned");
        *sender_guard = Some(panic_tx);
    }

    let panic_inputs = [(0usize, 11usize), (1usize, 17usize)];

    for (queue_position, input) in panic_inputs {
        rayon_core::spawn_fifo(move || {
            let worker_index = rayon_core::current_thread_index()
                .expect("panicking free spawn_fifo work should run on a Rayon worker");
            assert_eq!(worker_index, 0);
            assert_eq!(rayon_core::current_num_threads(), 1);

            let (left, right) =
                rayon_core::join(move || input * 2, move || worker_index + 1000);

            std::panic::panic_any(GlobalFifoPanic {
                queue_position,
                input,
                worker_index,
                num_threads: rayon_core::current_num_threads(),
                checksum: left + right,
            });
        });
    }

    rayon_core::spawn_fifo(|| {
        std::panic::panic_any(String::from(
            "string payload from free spawn_fifo reaches the global panic handler",
        ));
    });

    let panic_events = recv_exact(
        &panic_rx,
        panic_inputs.len() + 1,
        "global panic handler for free spawn_fifo",
    );

    let mut observed_panics = Vec::<GlobalFifoPanic>::new();
    let mut observed_messages = BTreeSet::<String>::new();

    for event in panic_events {
        match event {
            HandledGlobalPanic::Fifo(record) => observed_panics.push(record),
            HandledGlobalPanic::Message(message) => {
                assert!(
                    observed_messages.insert(message),
                    "message panic payload should be observed at most once"
                );
            }
            unexpected => panic!("unexpected global panic handler event: {unexpected:?}"),
        }
    }

    observed_panics.sort_by_key(|record| record.queue_position);

    assert_eq!(observed_panics.len(), panic_inputs.len());
    for record in &observed_panics {
        assert_eq!(record.worker_index, 0);
        assert_eq!(record.num_threads, 1);
        assert_eq!(record.input, panic_inputs[record.queue_position].1);
        assert_eq!(record.checksum, record.input * 2 + 1000);
    }

    assert_eq!(
        observed_messages,
        ["string payload from free spawn_fifo reaches the global panic handler".to_owned()]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should receive exactly the scheduled free spawn_fifo panics"
    );

    {
        let mut sender_guard = PANIC_SENDER
            .lock()
            .expect("panic sender mutex should not be poisoned");
        *sender_guard = None;
    }

    let (recovery_tx, recovery_rx) = mpsc::channel::<GlobalRecoveryRecord>();

    for panic_record in observed_panics.iter().cloned() {
        let recovery_tx = recovery_tx.clone();

        rayon_core::spawn_fifo(move || {
            let executing_index = rayon_core::current_thread_index()
                .expect("recovery free spawn_fifo work should run on a Rayon worker");
            assert_eq!(executing_index, 0);
            assert_eq!(rayon_core::current_num_threads(), panic_record.num_threads);

            let (left, right) = rayon_core::join(
                move || panic_record.checksum + panic_record.input,
                move || executing_index + panic_record.num_threads,
            );

            recovery_tx
                .send(GlobalRecoveryRecord {
                    queue_position: panic_record.queue_position,
                    input: panic_record.input,
                    panic_checksum: panic_record.checksum,
                    executing_index,
                    num_threads: rayon_core::current_num_threads(),
                    value: left + right,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                })
                .expect("recovery spawn_fifo worker should report its result");
        });
    }
    drop(recovery_tx);

    let mut recovery_records = recv_exact(
        &recovery_rx,
        observed_panics.len(),
        "post-panic free spawn_fifo recovery batch",
    );
    recovery_records.sort_by_key(|record| record.queue_position);

    assert_eq!(recovery_records.len(), observed_panics.len());
    for record in &recovery_records {
        let panic_record = &observed_panics[record.queue_position];
        assert_eq!(record.input, panic_record.input);
        assert_eq!(record.panic_checksum, panic_record.checksum);
        assert_eq!(record.executing_index, 0);
        assert_eq!(record.num_threads, 1);
        assert!(record.pending_status_available);
        assert_eq!(
            record.value,
            panic_record.checksum + panic_record.input + record.executing_index + record.num_threads
        );
    }

    let followups = Mutex::new(Vec::<GlobalRecoveryFollowup>::new());
    let expected_recovery_sum: usize = recovery_records.iter().map(|record| record.value).sum();

    let scope_return = rayon_core::scope(|scope| {
        for record in recovery_records.iter().cloned() {
            let followups = &followups;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped follow-up after detached panic should run on Rayon");

                let (left, right) = rayon_core::join(
                    move || record.value + record.panic_checksum,
                    move || executing_index,
                );

                followups
                    .lock()
                    .expect("recovery follow-up mutex should not be poisoned")
                    .push(GlobalRecoveryFollowup {
                        queue_position: record.queue_position,
                        executing_index,
                        value: left + right,
                    });
            });
        }

        expected_recovery_sum
    });

    assert_eq!(scope_return, expected_recovery_sum);

    let mut followups = followups
        .into_inner()
        .expect("recovery follow-up mutex should not be poisoned");
    followups.sort_by_key(|record| record.queue_position);

    assert_eq!(followups.len(), recovery_records.len());
    for record in &followups {
        let recovery = &recovery_records[record.queue_position];
        assert_eq!(record.executing_index, 0);
        assert_eq!(
            record.value,
            recovery.value + recovery.panic_checksum + record.executing_index
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::join(
        || followups.iter().map(|record| record.value).sum::<usize>(),
        || {
            followups
                .iter()
                .map(|record| {
                    let recovery = &recovery_records[record.queue_position];
                    recovery.value + recovery.panic_checksum + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_spawn_fifo_called_from_custom_pool_worker_uses_current_pool_and_feeds_pool_scope_fifo() {
    let _serial = TEST_MUTEX
        .lock()
        .expect("test serialization mutex should not be poisoned");
    ensure_single_worker_global_pool();

    let thread_count = 2usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("free-spawn-fifo-current-pool-worker-{index}"))
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
            seed: (index + 3) * (num_threads + 13),
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
        assert_eq!(record.seed, (record.index + 3) * (thread_count + 13));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let (fifo_tx, fifo_rx) = mpsc::channel::<CustomPoolFifoRecord>();
    let scheduled_seeds = std::sync::Arc::new(seeds.clone());

    let schedule_report = rayon_core::ThreadPool::scope(&pool, {
        let scheduled_seeds = std::sync::Arc::clone(&scheduled_seeds);
        let fifo_tx = fifo_tx.clone();

        move |_| {
            let scheduling_worker = rayon_core::current_thread_index()
                .expect("ThreadPool::scope body should run inside the custom pool");
            assert!(scheduling_worker < thread_count);
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            for seed_record in scheduled_seeds.iter().cloned() {
                let fifo_tx = fifo_tx.clone();

                rayon_core::spawn_fifo(move || {
                    let executing_index = rayon_core::current_thread_index()
                        .expect("free spawn_fifo from a pool worker should use that current pool");
                    assert!(executing_index < thread_count);
                    assert_eq!(
                        rayon_core::current_num_threads(),
                        thread_count,
                        "free spawn_fifo called from a custom-pool worker should not fall back to the single-worker global pool"
                    );

                    let worker_name = std::thread::current().name().map(str::to_owned);
                    assert_eq!(
                        worker_name.as_deref(),
                        Some(
                            format!("free-spawn-fifo-current-pool-worker-{executing_index}")
                                .as_str()
                        )
                    );

                    let origin_index = seed_record.index;
                    let seed = seed_record.seed;

                    let (left, right) = rayon_core::join(
                        move || seed + origin_index,
                        move || thread_count * 10 + executing_index,
                    );

                    fifo_tx
                        .send(CustomPoolFifoRecord {
                            origin_index,
                            seed,
                            executing_index,
                            num_threads: rayon_core::current_num_threads(),
                            current_index: rayon_core::current_thread_index(),
                            worker_name,
                            derived: left + right,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        })
                        .expect("custom-pool free spawn_fifo task should report its result");
                });
            }

            ScheduleReport {
                scheduling_worker,
                scheduling_threads: rayon_core::current_num_threads(),
                pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
                scheduled_jobs: scheduled_seeds.len(),
            }
        }
    });

    drop(fifo_tx);

    assert!(schedule_report.scheduling_worker < thread_count);
    assert_eq!(schedule_report.scheduling_threads, thread_count);
    assert!(schedule_report.pending_status_available);
    assert_eq!(schedule_report.scheduled_jobs, thread_count);

    let mut fifo_records = recv_exact(
        &fifo_rx,
        thread_count,
        "free spawn_fifo tasks scheduled from custom pool",
    );
    fifo_records.sort_by_key(|record| record.origin_index);

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
            record.worker_name.as_deref(),
            Some(format!("free-spawn-fifo-current-pool-worker-{}", record.executing_index).as_str())
        );
        assert_eq!(
            record.derived,
            record.seed + record.origin_index + thread_count * 10 + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "free spawn_fifo work in the custom pool should observe pending-task status"
        );
    }

    let fifo_by_origin: BTreeMap<usize, CustomPoolFifoRecord> = fifo_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(fifo_by_origin.len(), thread_count);

    let expected_fifo_sum: usize = fifo_records.iter().map(|record| record.derived).sum();
    let followups = Mutex::new(Vec::<CustomPoolFollowup>::new());

    let scope_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for fifo_record in fifo_records.iter().cloned() {
            let followups = &followups;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("custom-pool FIFO follow-up should run on a Rayon worker");
                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let origin_index = fifo_record.origin_index;
                let fifo_executing_index = fifo_record.executing_index;

                let (left, right) = rayon_core::join(
                    move || fifo_record.derived + fifo_record.seed,
                    move || origin_index + executing_index + fifo_executing_index,
                );

                followups
                    .lock()
                    .expect("custom follow-up mutex should not be poisoned")
                    .push(CustomPoolFollowup {
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

    let mut followups = followups
        .into_inner()
        .expect("custom follow-up mutex should not be poisoned");
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

        let source = fifo_by_origin
            .get(&record.origin_index)
            .expect("follow-up should correspond to a free spawn_fifo output");

        assert_eq!(record.fifo_executing_index, source.executing_index);
        assert_eq!(
            record.combined,
            source.derived
                + source.seed
                + record.origin_index
                + record.executing_index
                + record.fifo_executing_index
        );
    }

    let (observed_followup_sum, recomputed_followup_sum) =
        rayon_core::ThreadPool::join(
            &pool,
            || followups.iter().map(|record| record.combined).sum::<usize>(),
            || {
                followups
                    .iter()
                    .map(|record| {
                        let source = fifo_by_origin
                            .get(&record.origin_index)
                            .expect("source record should exist during recomputation");

                        source.derived
                            + source.seed
                            + record.origin_index
                            + record.executing_index
                            + record.fifo_executing_index
                    })
                    .sum::<usize>()
            },
        );

    assert_eq!(observed_followup_sum, recomputed_followup_sum);
}