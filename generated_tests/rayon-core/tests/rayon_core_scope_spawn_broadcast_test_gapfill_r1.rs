use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlobalAsyncRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    joined_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlobalScopedRecord {
    origin_index: usize,
    seed: usize,
    async_value: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScheduleReport {
    worker_index: usize,
    num_threads: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomStartedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: Option<String>,
    seed: usize,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomDoneRecord {
    index: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomFollowUpRecord {
    origin_index: usize,
    executing_index: usize,
    combined: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SpawnBroadcastPanicRecord {
    index: usize,
    num_threads: usize,
    seed: usize,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HandledSpawnBroadcastPanic {
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

fn classify_payload(payload: &(dyn Any + Send)) -> HandledSpawnBroadcastPanic {
    if let Some(record) = payload.downcast_ref::<SpawnBroadcastPanicRecord>() {
        HandledSpawnBroadcastPanic::Broadcast(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledSpawnBroadcastPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledSpawnBroadcastPanic::Message(message.clone())
    } else {
        HandledSpawnBroadcastPanic::Unexpected
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_spawn_broadcast_global_pipeline_feeds_scope_and_scoped_broadcast() {
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "the integration-test thread should start outside Rayon"
    );

    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());

    let expected_indices = expected_worker_indices(global_threads);
    let run_count = Arc::new(AtomicUsize::new(0));
    let (async_tx, async_rx) = mpsc::channel::<GlobalAsyncRecord>();

    rayon_core::spawn_broadcast({
        let run_count = Arc::clone(&run_count);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert!(index < num_threads);
            assert_eq!(num_threads, global_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), num_threads);

            let previous_runs = run_count.fetch_add(1, Ordering::SeqCst);
            assert!(
                previous_runs < num_threads,
                "rayon_core::spawn_broadcast should run at most once per worker"
            );

            let seed = (index + 3) * (num_threads + 31);
            let (left, right) =
                rayon_core::join(move || seed + index, move || num_threads * 11);

            async_tx
                .send(GlobalAsyncRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    joined_value: left + right,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                })
                .expect("spawn_broadcast worker should report its result");
        }
    });

    let mut async_records = recv_exact(
        &async_rx,
        global_threads,
        "rayon_core::spawn_broadcast global batch",
    );
    async_records.sort_by_key(|record| record.index);

    assert_eq!(run_count.load(Ordering::SeqCst), global_threads);
    assert!(
        async_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_broadcast should report exactly once per global worker"
    );
    assert_eq!(
        async_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &async_records {
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, (record.index + 3) * (global_threads + 31));
        assert_eq!(
            record.joined_value,
            record.seed + record.index + global_threads * 11
        );
        assert!(
            record.pending_status_available,
            "spawn_broadcast work should be able to query pending-task status"
        );
    }

    let async_by_index: BTreeMap<usize, GlobalAsyncRecord> = async_records
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();
    assert_eq!(async_by_index.len(), global_threads);

    let expected_async_sum: usize = async_records
        .iter()
        .map(|record| record.joined_value)
        .sum();

    let scoped_records = Mutex::new(Vec::<GlobalScopedRecord>::new());
    let scoped_broadcast_records = Mutex::new(Vec::<ScopedBroadcastRecord>::new());
    let scoped_broadcast_count = AtomicUsize::new(0);

    let scope_return = rayon_core::scope(|scope| {
        for record in async_records.iter().cloned() {
            let scoped_records_ref = &scoped_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped follow-up work should run on a Rayon worker");

                assert!(executing_index < record.num_threads);
                assert_eq!(rayon_core::current_num_threads(), record.num_threads);

                let origin_index = record.index;
                let seed = record.seed;
                let async_value = record.joined_value;
                let num_threads = record.num_threads;

                let (left, right) = rayon_core::join(
                    move || async_value + seed,
                    move || origin_index + executing_index + num_threads,
                );

                scoped_records_ref
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(GlobalScopedRecord {
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

        let async_by_index_ref = &async_by_index;
        let scoped_broadcast_records_ref = &scoped_broadcast_records;
        let scoped_broadcast_count_ref = &scoped_broadcast_count;

        rayon_core::Scope::spawn_broadcast(scope, move |_, context| {
            scoped_broadcast_count_ref.fetch_add(1, Ordering::SeqCst);

            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, global_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let source = async_by_index_ref
                .get(&index)
                .expect("scoped broadcast should find async output by worker index");

            let joined_value = source.joined_value;
            let seed = source.seed;
            let (left, right) =
                rayon_core::join(move || joined_value, move || seed + index + num_threads);

            scoped_broadcast_records_ref
                .lock()
                .expect("scoped broadcast record mutex should not be poisoned")
                .push(ScopedBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    value: left + right,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                });
        });

        expected_async_sum
    });

    assert_eq!(scope_return, expected_async_sum);
    assert_eq!(scoped_broadcast_count.load(Ordering::SeqCst), global_threads);

    let mut scoped_records = scoped_records
        .into_inner()
        .expect("scoped record mutex should not be poisoned");
    scoped_records.sort_by_key(|record| record.origin_index);

    assert_eq!(scoped_records.len(), global_threads);
    assert_eq!(
        scoped_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &scoped_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(record.seed, async_by_index[&record.origin_index].seed);
        assert_eq!(
            record.async_value,
            async_by_index[&record.origin_index].joined_value
        );
        assert_eq!(
            record.value,
            record.async_value
                + record.seed
                + record.origin_index
                + record.executing_index
                + global_threads
        );
        assert!(
            record.pending_status_available,
            "scoped follow-up work should observe pending-task status"
        );
    }

    let mut scoped_broadcast_records = scoped_broadcast_records
        .into_inner()
        .expect("scoped broadcast record mutex should not be poisoned");
    scoped_broadcast_records.sort_by_key(|record| record.index);

    assert_eq!(scoped_broadcast_records.len(), global_threads);
    assert_eq!(
        scoped_broadcast_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &scoped_broadcast_records {
        let source = &async_by_index[&record.index];

        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, source.seed);
        assert_eq!(
            record.value,
            source.joined_value + source.seed + record.index + global_threads
        );
        assert!(
            record.pending_status_available,
            "Scope::spawn_broadcast work should observe pending-task status"
        );
    }

    let scoped_by_index: BTreeMap<usize, GlobalScopedRecord> = scoped_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    let scoped_broadcast_by_index: BTreeMap<usize, ScopedBroadcastRecord> =
        scoped_broadcast_records
            .iter()
            .cloned()
            .map(|record| (record.index, record))
            .collect();

    let mut confirmations = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let scoped = scoped_by_index
            .get(&index)
            .expect("confirmation broadcast should find scoped output");
        let scoped_broadcast = scoped_broadcast_by_index
            .get(&index)
            .expect("confirmation broadcast should find scoped broadcast output");

        let scoped_value = scoped.value;
        let scoped_broadcast_value = scoped_broadcast.value;
        let (left, right) = rayon_core::join(move || scoped_value, move || scoped_broadcast_value);

        (index, num_threads, rayon_core::current_thread_index(), left + right)
    });

    confirmations.sort_by_key(|record| record.0);

    assert_eq!(confirmations.len(), global_threads);
    assert_eq!(
        confirmations
            .iter()
            .map(|(index, _, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, num_threads, current_index, total) in confirmations {
        assert_eq!(num_threads, global_threads);
        assert_eq!(current_index, Some(index));
        assert_eq!(
            total,
            scoped_by_index[&index].value + scoped_broadcast_by_index[&index].value
        );
    }

    let (scoped_sum, broadcast_sum) = rayon_core::join(
        || scoped_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            scoped_broadcast_records
                .iter()
                .map(|record| record.value)
                .sum::<usize>()
        },
    );

    assert_eq!(
        scoped_sum,
        scoped_records.iter().map(|record| record.value).sum::<usize>()
    );
    assert_eq!(
        broadcast_sum,
        scoped_broadcast_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>()
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_spawn_broadcast_from_custom_pool_worker_is_detached_uses_current_pool_and_recovers_after_panics()
{
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (panic_tx, panic_rx) = mpsc::channel::<HandledSpawnBroadcastPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("free-spawn-broadcast-current-pool-worker-{index}"))
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

        (index, (index + 2) * (num_threads + 47))
    });
    seeds.sort_by_key(|record| record.0);

    assert_eq!(seeds.len(), thread_count);
    assert_eq!(
        seeds.iter().map(|(index, _)| *index).collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, seed) in &seeds {
        assert_eq!(*seed, (*index + 2) * (thread_count + 47));
    }

    let seed_by_index = Arc::new(seeds.iter().map(|(_, seed)| *seed).collect::<Vec<_>>());
    let release = Arc::new((Mutex::new(false), Condvar::new()));

    let (started_tx, started_rx) = mpsc::channel::<CustomStartedRecord>();
    let (done_tx, done_rx) = mpsc::channel::<CustomDoneRecord>();

    let schedule_report = rayon_core::ThreadPool::scope(pool_ref, |_| {
        let worker_index = rayon_core::current_thread_index()
            .expect("ThreadPool::scope body should run inside the custom pool");
        assert!(worker_index < thread_count);

        rayon_core::spawn_broadcast({
            let seed_by_index = Arc::clone(&seed_by_index);
            let release = Arc::clone(&release);

            move |context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(
                    num_threads, thread_count,
                    "free spawn_broadcast called from a custom-pool worker should use that pool"
                );
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let worker_name = std::thread::current().name().map(str::to_owned);
                let expected_name = format!("free-spawn-broadcast-current-pool-worker-{index}");
                assert_eq!(worker_name.as_deref(), Some(expected_name.as_str()));

                let seed = seed_by_index[index];
                let (left, right) =
                    rayon_core::join(move || seed + index, move || num_threads * 100);
                let derived = left + right;

                started_tx
                    .send(CustomStartedRecord {
                        index,
                        num_threads,
                        current_index: rayon_core::current_thread_index(),
                        worker_name,
                        seed,
                        derived,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    })
                    .expect("custom-pool broadcast worker should report before blocking");

                let (lock, condvar) = &*release;
                let mut released = lock
                    .lock()
                    .expect("release mutex should not be poisoned while broadcast waits");
                while !*released {
                    released = condvar
                        .wait(released)
                        .expect("release mutex should not be poisoned while waiting");
                }
                drop(released);

                done_tx
                    .send(CustomDoneRecord {
                        index,
                        value: derived + num_threads + index,
                    })
                    .expect("custom-pool broadcast worker should report after release");
            }
        });

        ScheduleReport {
            worker_index,
            num_threads: rayon_core::current_num_threads(),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    assert!(schedule_report.worker_index < thread_count);
    assert_eq!(schedule_report.num_threads, thread_count);
    assert!(
        schedule_report.pending_status_available,
        "custom-pool scope body should observe worker-local pending-task status"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "detached spawn_broadcast should return to the external caller"
    );

    let mut started_records = recv_exact(
        &started_rx,
        thread_count,
        "detached free spawn_broadcast custom-pool start",
    );
    started_records.sort_by_key(|record| record.index);

    assert!(
        started_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each custom-pool worker should start exactly one detached broadcast task"
    );
    assert!(
        done_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "blocked detached broadcast tasks should not complete before release"
    );

    assert_eq!(
        started_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &started_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.derived,
            seed_by_index[record.index] + record.index + thread_count * 100
        );

        let expected_name =
            format!("free-spawn-broadcast-current-pool-worker-{}", record.index);
        assert_eq!(record.worker_name.as_deref(), Some(expected_name.as_str()));
        assert!(
            record.pending_status_available,
            "custom-pool spawn_broadcast task should query pending-task status"
        );
    }

    {
        let (lock, condvar) = &*release;
        let mut released = lock
            .lock()
            .expect("release mutex should not be poisoned when releasing broadcast workers");
        *released = true;
        condvar.notify_all();
    }

    let mut done_records = recv_exact(
        &done_rx,
        thread_count,
        "detached free spawn_broadcast custom-pool completion",
    );
    done_records.sort_by_key(|record| record.index);

    assert!(
        done_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each custom-pool broadcast task should complete exactly once"
    );
    assert_eq!(
        done_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let done_by_index: BTreeMap<usize, usize> = done_records
        .iter()
        .map(|record| (record.index, record.value))
        .collect();
    let derived_by_index: BTreeMap<usize, usize> = started_records
        .iter()
        .map(|record| (record.index, record.derived))
        .collect();

    for record in &done_records {
        assert_eq!(
            record.value,
            derived_by_index[&record.index] + thread_count + record.index
        );
    }

    let done_sum: usize = done_records.iter().map(|record| record.value).sum();
    let derived_sum: usize = started_records.iter().map(|record| record.derived).sum();

    let followups = Mutex::new(Vec::<CustomFollowUpRecord>::new());

    let scope_fifo_return = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
        for record in started_records.iter().cloned() {
            let followups_ref = &followups;
            let done_value = done_by_index[&record.index];

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO follow-up work should run in the custom pool");
                assert!(executing_index < thread_count);

                let origin_index = record.index;
                let (left, right) = rayon_core::join(
                    move || record.derived + done_value,
                    move || origin_index + executing_index,
                );

                followups_ref
                    .lock()
                    .expect("follow-up record mutex should not be poisoned")
                    .push(CustomFollowUpRecord {
                        origin_index,
                        executing_index,
                        combined: left + right,
                    });
            });
        }

        done_sum + derived_sum
    });

    assert_eq!(scope_fifo_return, done_sum + derived_sum);

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
        assert_eq!(
            record.combined,
            derived_by_index[&record.origin_index]
                + done_by_index[&record.origin_index]
                + record.origin_index
                + record.executing_index
        );
    }

    let panic_schedule_report = rayon_core::ThreadPool::scope(pool_ref, |_| {
        let worker_index = rayon_core::current_thread_index()
            .expect("panic scheduling scope should run inside the custom pool");
        assert!(worker_index < thread_count);

        rayon_core::spawn_broadcast({
            let seed_by_index = Arc::clone(&seed_by_index);

            move |context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(num_threads, thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let seed = seed_by_index[index];
                let (left, right) =
                    rayon_core::join(move || seed + index, move || num_threads * 1000);

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
        }
    });

    assert!(panic_schedule_report.worker_index < thread_count);
    assert_eq!(panic_schedule_report.num_threads, thread_count);
    assert!(panic_schedule_report.pending_status_available);

    let panic_events = recv_exact(
        &panic_rx,
        thread_count,
        "panic handler for detached free spawn_broadcast",
    );

    let mut observed_panics = BTreeSet::<SpawnBroadcastPanicRecord>::new();

    for event in panic_events {
        match event {
            HandledSpawnBroadcastPanic::Broadcast(record) => {
                assert!(
                    observed_panics.insert(record),
                    "each broadcast panic payload should be handled exactly once"
                );
            }
            unexpected => panic!("unexpected spawn_broadcast panic event: {unexpected:?}"),
        }
    }

    let expected_panics: BTreeSet<_> = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| SpawnBroadcastPanicRecord {
            index,
            num_threads: thread_count,
            seed: *seed,
            checksum: *seed + index + thread_count * 1000,
        })
        .collect();

    assert_eq!(observed_panics, expected_panics);
    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic handler should receive exactly one payload per panicking broadcast worker"
    );

    let panic_checksum_sum: usize = observed_panics.iter().map(|record| record.checksum).sum();

    let (recovery_tx, recovery_rx) = mpsc::channel::<RecoveryAsyncRecord>();

    let recovery_schedule_report = rayon_core::ThreadPool::scope(pool_ref, |_| {
        let worker_index = rayon_core::current_thread_index()
            .expect("recovery scheduling scope should run inside the custom pool");
        assert!(worker_index < thread_count);

        rayon_core::spawn_broadcast({
            let seed_by_index = Arc::clone(&seed_by_index);

            move |context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(num_threads, thread_count);
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
        }
    });

    assert!(recovery_schedule_report.worker_index < thread_count);
    assert_eq!(recovery_schedule_report.num_threads, thread_count);
    assert!(recovery_schedule_report.pending_status_available);

    let mut recovery_records = recv_exact(
        &recovery_rx,
        thread_count,
        "recovery detached free spawn_broadcast",
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
            "non-panicking recovery broadcast should run on Rayon workers"
        );
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "successful recovery broadcast should not invoke the panic handler"
    );

    let recovery_value_sum: usize = recovery_records.iter().map(|record| record.value).sum();
    let recovery_value_by_index: BTreeMap<usize, usize> = recovery_records
        .iter()
        .map(|record| (record.index, record.value))
        .collect();

    let scoped_recovery = Mutex::new(Vec::<RecoveryScopedRecord>::new());

    let recovery_scope_return = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        for record in recovery_records.iter().cloned() {
            let scoped_recovery_ref = &scoped_recovery;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("post-panic scoped recovery should run in the custom pool");
                assert!(executing_index < thread_count);

                let origin_index = record.index;
                let seed = record.seed;
                let value = record.value;

                let (left, right) = rayon_core::join(
                    move || value + seed,
                    move || origin_index + executing_index + thread_count,
                );

                scoped_recovery_ref
                    .lock()
                    .expect("scoped recovery mutex should not be poisoned")
                    .push(RecoveryScopedRecord {
                        origin_index,
                        seed,
                        executing_index,
                        value: left + right,
                    });
            });
        }

        panic_checksum_sum + recovery_value_sum
    });

    assert_eq!(
        recovery_scope_return,
        panic_checksum_sum + recovery_value_sum
    );

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

    for record in &scoped_recovery {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            recovery_value_by_index[&record.origin_index]
                + record.seed
                + record.origin_index
                + record.executing_index
                + thread_count
        );
    }

    let (observed_scoped_sum, recomputed_scoped_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || scoped_recovery.iter().map(|record| record.value).sum::<usize>(),
        || {
            scoped_recovery
                .iter()
                .map(|record| {
                    recovery_value_by_index[&record.origin_index]
                        + seed_by_index[record.origin_index]
                        + record.origin_index
                        + record.executing_index
                        + thread_count
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_scoped_sum, recomputed_scoped_sum);
}