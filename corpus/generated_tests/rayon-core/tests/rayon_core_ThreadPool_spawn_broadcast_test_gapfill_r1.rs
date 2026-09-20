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
struct AsyncBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FollowUpRecord {
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
fn thread_pool_spawn_broadcast_consumes_broadcast_seeds_and_feeds_scoped_followup_work() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("spawn-broadcast-pipeline-worker-{index}"))
        .build()
        .expect("custom Rayon thread pool should build");

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
        None,
        "pending-task status is unavailable outside the custom pool"
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
            seed: (index + 1) * (num_threads + 101),
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 101));
    }

    let seed_by_index = Arc::new(seeds.iter().map(|record| record.seed).collect::<Vec<_>>());
    let expected_async_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed + thread_count + index * 10)
        .sum();

    let (async_tx, async_rx) = mpsc::channel::<AsyncBroadcastRecord>();
    let run_count = Arc::new(AtomicUsize::new(0));

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let seed_by_index = Arc::clone(&seed_by_index);
        let run_count = Arc::clone(&run_count);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert!(index < num_threads);
            assert_eq!(rayon_core::current_num_threads(), thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let prior_runs = run_count.fetch_add(1, Ordering::SeqCst);
            assert!(
                prior_runs < num_threads,
                "spawn_broadcast should run at most once per worker"
            );

            let seed = seed_by_index[index];
            let (seed_component, index_component) =
                rayon_core::join(move || seed + num_threads, move || index * 10);

            async_tx
                .send(AsyncBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    derived: seed_component + index_component,
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                })
                .expect("spawn_broadcast worker should report its record");
        }
    });

    let mut async_records = recv_exact(
        &async_rx,
        thread_count,
        "ThreadPool::spawn_broadcast seeded pipeline",
    );
    async_records.sort_by_key(|record| record.index);

    assert!(
        async_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_broadcast should report exactly once per worker"
    );
    assert_eq!(run_count.load(Ordering::SeqCst), thread_count);

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
            record.derived,
            seed_by_index[record.index] + thread_count + record.index * 10
        );
        assert!(
            record.pending_status_available,
            "spawn_broadcast work should be able to query worker-local pending-task status"
        );
    }

    assert_eq!(
        async_records
            .iter()
            .map(|record| record.derived)
            .sum::<usize>(),
        expected_async_sum
    );

    let followup_records = Mutex::new(Vec::<FollowUpRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for record in async_records.iter().cloned() {
            let followup_records = &followup_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped follow-up work should run inside the custom pool");
                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let origin_index = record.index;
                let seed = record.seed;
                let derived = record.derived;
                let num_threads = record.num_threads;

                let (left, right) = rayon_core::join(
                    move || derived + seed,
                    move || origin_index + executing_index + num_threads,
                );

                followup_records
                    .lock()
                    .expect("follow-up record mutex should not be poisoned")
                    .push(FollowUpRecord {
                        origin_index,
                        executing_index,
                        combined: left + right,
                    });
            });
        }

        async_records.len()
    });

    assert_eq!(scope_return, thread_count);

    let mut followup_records = followup_records
        .into_inner()
        .expect("follow-up record mutex should not be poisoned");
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
        assert!(record.executing_index < thread_count);

        let broadcast_record = async_records
            .iter()
            .find(|candidate| candidate.index == record.origin_index)
            .expect("follow-up record should correspond to an async broadcast record");

        assert_eq!(
            record.combined,
            broadcast_record.derived
                + broadcast_record.seed
                + broadcast_record.index
                + record.executing_index
                + broadcast_record.num_threads
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::ThreadPool::join(
        &pool,
        || followup_records.iter().map(|record| record.combined).sum::<usize>(),
        || {
            followup_records
                .iter()
                .map(|record| {
                    let broadcast_record = async_records
                        .iter()
                        .find(|candidate| candidate.index == record.origin_index)
                        .expect("broadcast record should exist during recomputation");

                    broadcast_record.derived
                        + broadcast_record.seed
                        + broadcast_record.index
                        + record.executing_index
                        + broadcast_record.num_threads
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_spawn_broadcast_is_detached_and_blocked_tasks_resume_after_release() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("spawn-broadcast-detached-worker-{index}"))
        .build()
        .expect("custom Rayon thread pool should build");

    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let started = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    let (started_tx, started_rx) = mpsc::channel::<(usize, usize, Option<usize>)>();
    let (done_tx, done_rx) = mpsc::channel::<(usize, usize)>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let release = Arc::clone(&release);
        let started = Arc::clone(&started);
        let completed = Arc::clone(&completed);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            started.fetch_add(1, Ordering::SeqCst);

            started_tx
                .send((index, num_threads, rayon_core::current_thread_index()))
                .expect("started receiver should remain alive");

            let (lock, condvar) = &*release;
            let mut released = lock
                .lock()
                .expect("release mutex should not be poisoned while waiting");
            while !*released {
                released = condvar
                    .wait(released)
                    .expect("release mutex should not be poisoned while blocked");
            }
            drop(released);

            let (left, right) =
                rayon_core::join(move || index + 1, move || num_threads * 100);

            completed.fetch_add(1, Ordering::SeqCst);

            done_tx
                .send((index, left + right))
                .expect("done receiver should remain alive");
        }
    });

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "spawn_broadcast should return to the external caller without making it a worker"
    );

    let mut started_records = recv_exact(
        &started_rx,
        thread_count,
        "blocked ThreadPool::spawn_broadcast start",
    );
    started_records.sort_by_key(|record| record.0);

    assert_eq!(started.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        started_records
            .iter()
            .map(|(index, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, num_threads, current_index) in &started_records {
        assert_eq!(*num_threads, thread_count);
        assert_eq!(*current_index, Some(*index));
    }

    assert!(
        started_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "blocked spawn_broadcast tasks should start exactly once per worker"
    );
    assert!(
        done_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "blocked spawn_broadcast tasks should not complete before the test releases them"
    );
    assert_eq!(completed.load(Ordering::SeqCst), 0);

    {
        let (lock, condvar) = &*release;
        let mut released = lock
            .lock()
            .expect("release mutex should not be poisoned when releasing workers");
        *released = true;
        condvar.notify_all();
    }

    let mut completed_records = recv_exact(
        &done_rx,
        thread_count,
        "blocked ThreadPool::spawn_broadcast completion",
    );
    completed_records.sort_by_key(|record| record.0);

    assert_eq!(completed.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        completed_records
            .iter()
            .map(|(index, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, value) in &completed_records {
        assert_eq!(*value, *index + 1 + thread_count * 100);
    }

    let mut followup_broadcast = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, index * 2 + num_threads)
    });
    followup_broadcast.sort_by_key(|record| record.0);

    assert_eq!(
        followup_broadcast
            .iter()
            .map(|(index, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, value) in followup_broadcast {
        assert_eq!(value, index * 2 + thread_count);
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_spawn_broadcast_panics_are_handled_and_pool_accepts_later_broadcast_work() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (panic_tx, panic_rx) = mpsc::channel::<HandledSpawnBroadcastPanic>();
    let panic_tx = Arc::new(Mutex::new(panic_tx));

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("spawn-broadcast-panic-worker-{index}"))
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

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 3) * (num_threads + 17),
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

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let seed_by_index = Arc::clone(&seed_by_index);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let seed = seed_by_index[index];

            std::panic::panic_any(SpawnBroadcastPanicRecord {
                index,
                num_threads,
                seed,
                checksum: seed + index + num_threads * 100,
            });
        }
    });

    let events = recv_exact(
        &panic_rx,
        thread_count,
        "ThreadPool::spawn_broadcast panic handler",
    );

    let mut observed_panics = BTreeSet::new();
    for event in events {
        match event {
            HandledSpawnBroadcastPanic::Broadcast(record) => {
                assert!(
                    observed_panics.insert(record),
                    "each spawn_broadcast panic payload should be handled exactly once"
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
        "panic handler should receive exactly one payload per panicking broadcast worker"
    );

    let (recovery_tx, recovery_rx) = mpsc::channel::<AsyncBroadcastRecord>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let seed_by_index = Arc::clone(&seed_by_index);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let seed = seed_by_index[index];
            let (left, right) =
                rayon_core::join(move || seed + index, move || num_threads * 5);

            recovery_tx
                .send(AsyncBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    derived: left + right,
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                })
                .expect("recovery spawn_broadcast worker should report successfully");
        }
    });

    let mut recovery_records = recv_exact(
        &recovery_rx,
        thread_count,
        "recovery ThreadPool::spawn_broadcast",
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
            record.derived,
            seed_by_index[record.index] + record.index + thread_count * 5
        );
        assert!(
            record.pending_status_available,
            "recovery spawn_broadcast work should run inside Rayon workers"
        );
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "non-panicking recovery broadcast should not invoke the panic handler"
    );

    let (observed_recovery_sum, expected_recovery_sum) = rayon_core::ThreadPool::join(
        &pool,
        || recovery_records.iter().map(|record| record.derived).sum::<usize>(),
        || {
            seed_by_index
                .iter()
                .enumerate()
                .map(|(index, seed)| *seed + index + thread_count * 5)
                .sum::<usize>()
        },
    );

    assert_eq!(observed_recovery_sum, expected_recovery_sum);
}