use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
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
struct FifoParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoBroadcastChildRecord {
    origin_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug)]
struct LocalOutcome {
    trace: Rc<RefCell<Vec<String>>>,
    scheduled_fifo_jobs: usize,
    scheduled_broadcast_jobs: usize,
    expected_parent_sum: usize,
    expected_broadcast_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    value: usize,
}

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

fn panic_payload_to_string(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_owned()
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_in_place_scope_fifo_builds_nested_fifo_pipeline_from_broadcast_seeds() {
    let thread_count = 4usize;
    let caller_thread = std::thread::current().id();

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("in-place-fifo-pipeline-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let pool_ref = &pool;

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(pool_ref),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "the integration-test thread should not be a worker in this pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None,
        "pending-task status is unavailable outside this custom pool"
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 19),
        }
    });

    seeds.sort_by_key(|record| record.index);

    let expected_indices = expected_worker_indices(thread_count);
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 19));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_parent_sum: usize = seeds
        .iter()
        .map(|record| record.seed + record.index + record.num_threads)
        .sum();
    let expected_broadcast_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| seed * 2 + index)
        .sum();

    let parent_records = Mutex::new(Vec::<FifoParentRecord>::new());
    let child_records = Mutex::new(Vec::<FifoChildRecord>::new());
    let broadcast_records = Mutex::new(Vec::<FifoBroadcastRecord>::new());
    let broadcast_child_records = Mutex::new(Vec::<FifoBroadcastChildRecord>::new());

    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);
    let broadcast_child_started = AtomicUsize::new(0);

    let local_trace = Rc::new(RefCell::new(Vec::<String>::new()));

    let outcome = rayon_core::ThreadPool::in_place_scope_fifo(pool_ref, |scope| {
        assert_eq!(
            std::thread::current().id(),
            caller_thread,
            "ThreadPool::in_place_scope_fifo should run its body on the calling thread"
        );
        assert_eq!(
            rayon_core::ThreadPool::current_thread_index(pool_ref),
            None
        );
        assert_eq!(
            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
            None
        );

        local_trace
            .borrow_mut()
            .push(format!("body-started-with-{}-seeds", seeds.len()));

        for seed_record in seeds.iter().cloned() {
            let parent_records_ref = &parent_records;
            let child_records_ref = &child_records;
            let parent_started_ref = &parent_started;
            let child_started_ref = &child_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |nested_scope| {
                parent_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("FIFO parent work should run inside the custom pool");
                assert!(executing_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                let pending_status_available =
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (seed_and_origin, thread_component) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || num_threads,
                );
                let parent_value = seed_and_origin + thread_component;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(FifoParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        value: parent_value,
                        pending_status_available,
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index =
                        rayon_core::ThreadPool::current_thread_index(pool_ref)
                            .expect("FIFO child work should run inside the custom pool");
                    assert!(child_executing_index < thread_count);

                    let (parent_component, worker_component) = rayon_core::join(
                        move || parent_value * 2,
                        move || child_executing_index,
                    );

                    child_records_ref
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(FifoChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            value: parent_component + worker_component,
                        });
                });
            });
        }

        let seed_by_index_ref = &seed_by_index;
        let broadcast_records_ref = &broadcast_records;
        let broadcast_child_records_ref = &broadcast_child_records;
        let broadcast_child_started_ref = &broadcast_child_started;

        rayon_core::ScopeFifo::spawn_broadcast(scope, move |scope, context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert!(index < num_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            let seed = seed_by_index_ref[index];
            let (doubled_seed, index_component) =
                rayon_core::join(move || seed * 2, move || index);
            let broadcast_value = doubled_seed + index_component;

            broadcast_records_ref
                .lock()
                .expect("broadcast record mutex should not be poisoned")
                .push(FifoBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    value: broadcast_value,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                });

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                broadcast_child_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("FIFO child spawned by broadcast should run inside the custom pool");
                assert!(executing_index < num_threads);

                broadcast_child_records_ref
                    .lock()
                    .expect("broadcast child record mutex should not be poisoned")
                    .push(FifoBroadcastChildRecord {
                        origin_index: index,
                        executing_index,
                        value: broadcast_value + num_threads + executing_index,
                    });
            });
        });

        local_trace.borrow_mut().push(format!(
            "body-spawned-parent-sum-{expected_parent_sum}-broadcast-sum-{expected_broadcast_sum}"
        ));

        LocalOutcome {
            trace: Rc::clone(&local_trace),
            scheduled_fifo_jobs: seeds.len(),
            scheduled_broadcast_jobs: thread_count,
            expected_parent_sum,
            expected_broadcast_sum,
        }
    });

    assert!(
        Rc::ptr_eq(&outcome.trace, &local_trace),
        "ThreadPool::in_place_scope_fifo should be able to return non-Send caller-local data"
    );
    assert_eq!(outcome.scheduled_fifo_jobs, thread_count);
    assert_eq!(outcome.scheduled_broadcast_jobs, thread_count);
    assert_eq!(outcome.expected_parent_sum, expected_parent_sum);
    assert_eq!(outcome.expected_broadcast_sum, expected_broadcast_sum);
    assert_eq!(
        outcome.trace.borrow().clone(),
        vec![
            format!("body-started-with-{thread_count}-seeds"),
            format!(
                "body-spawned-parent-sum-{expected_parent_sum}-broadcast-sum-{expected_broadcast_sum}"
            )
        ]
    );

    assert_eq!(parent_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(child_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        broadcast_child_started.load(Ordering::SeqCst),
        thread_count
    );

    let mut parent_records = parent_records
        .into_inner()
        .expect("parent record mutex should not be poisoned");
    parent_records.sort_by_key(|record| record.origin_index);

    assert_eq!(parent_records.len(), thread_count);
    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &parent_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count
        );
        assert!(
            record.pending_status_available,
            "FIFO worker should be able to query pending-task status"
        );
    }

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>(),
        expected_parent_sum
    );

    let parent_by_origin: BTreeMap<usize, FifoParentRecord> = parent_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(parent_by_origin.len(), thread_count);

    let mut child_records = child_records
        .into_inner()
        .expect("child record mutex should not be poisoned");
    child_records.sort_by_key(|record| record.origin_index);

    assert_eq!(child_records.len(), thread_count);
    assert_eq!(
        child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &child_records {
        assert!(record.origin_index < thread_count);
        assert!(record.parent_executing_index < thread_count);
        assert!(record.executing_index < thread_count);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("child record should correspond to a FIFO parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(record.value, parent.value * 2 + record.executing_index);
    }

    let mut broadcast_records = broadcast_records
        .into_inner()
        .expect("broadcast record mutex should not be poisoned");
    broadcast_records.sort_by_key(|record| record.index);

    assert_eq!(broadcast_records.len(), thread_count);
    assert_eq!(
        broadcast_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &broadcast_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(record.value, seed_by_index[record.index] * 2 + record.index);
        assert!(
            record.pending_status_available,
            "broadcast work should be able to query pending-task status"
        );
    }

    assert_eq!(
        broadcast_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>(),
        expected_broadcast_sum
    );

    let mut broadcast_child_records = broadcast_child_records
        .into_inner()
        .expect("broadcast child record mutex should not be poisoned");
    broadcast_child_records.sort_by_key(|record| record.origin_index);

    assert_eq!(broadcast_child_records.len(), thread_count);
    assert_eq!(
        broadcast_child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &broadcast_child_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(
            record.value,
            seed_by_index[record.origin_index] * 2
                + record.origin_index
                + thread_count
                + record.executing_index
        );
    }

    let expected_broadcast_child_sum: usize = broadcast_child_records
        .iter()
        .map(|record| {
            seed_by_index[record.origin_index] * 2
                + record.origin_index
                + thread_count
                + record.executing_index
        })
        .sum();

    let (observed_parent_sum, observed_broadcast_child_sum) =
        rayon_core::ThreadPool::join(
            pool_ref,
            || parent_records.iter().map(|record| record.value).sum::<usize>(),
            || {
                broadcast_child_records
                    .iter()
                    .map(|record| record.value)
                    .sum::<usize>()
            },
        );

    assert_eq!(observed_parent_sum, expected_parent_sum);
    assert_eq!(
        observed_broadcast_child_sum,
        expected_broadcast_child_sum
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_in_place_scope_fifo_preserves_fifo_order_for_jobs_queued_by_caller() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("in-place-fifo-order-worker-{index}"))
        .build()
        .expect("single-worker Rayon pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);

    let observed_order = Arc::new(Mutex::new(Vec::<usize>::new()));
    let release_worker = Arc::new((Mutex::new(false), Condvar::new()));
    let (started_tx, started_rx) = mpsc::channel::<()>();

    rayon_core::ThreadPool::spawn(&pool, {
        let release_worker = Arc::clone(&release_worker);

        move || {
            assert_eq!(rayon_core::current_num_threads(), 1);
            assert_eq!(rayon_core::current_thread_index(), Some(0));

            started_tx
                .send(())
                .expect("test thread should wait for the blocking worker task");

            let (lock, condvar) = &*release_worker;
            let mut released = lock
                .lock()
                .expect("release mutex should not be poisoned while blocking worker");
            while !*released {
                released = condvar
                    .wait(released)
                    .expect("release mutex should not be poisoned while waiting");
            }
        }
    });

    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("blocking worker task should start before FIFO jobs are queued");

    let returned = rayon_core::ThreadPool::in_place_scope_fifo(&pool, |scope| {
        for value in 0usize..8 {
            let observed_order = Arc::clone(&observed_order);

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                assert_eq!(rayon_core::current_num_threads(), 1);
                assert_eq!(rayon_core::current_thread_index(), Some(0));

                let (left, right) =
                    rayon_core::join(move || value, move || value * 10);

                observed_order
                    .lock()
                    .expect("observed order mutex should not be poisoned")
                    .push(left + right);
            });
        }

        let (lock, condvar) = &*release_worker;
        {
            let mut released = lock
                .lock()
                .expect("release mutex should not be poisoned when releasing worker");
            *released = true;
        }
        condvar.notify_one();

        "queued FIFO jobs released"
    });

    assert_eq!(returned, "queued FIFO jobs released");

    let observed_order = observed_order
        .lock()
        .expect("observed order mutex should not be poisoned")
        .clone();

    let expected_order: Vec<_> = (0usize..8).map(|value| value + value * 10).collect();
    assert_eq!(
        observed_order, expected_order,
        "with one blocked worker, FIFO scoped jobs should run in the order they were queued"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_in_place_scope_fifo_propagates_scoped_panics_and_pool_recovers() {
    let thread_count = 3usize;

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("in-place-fifo-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let pool_ref = &pool;

    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<(usize, usize)>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::ThreadPool::in_place_scope_fifo(pool_ref, |scope| {
            for value in 0..(thread_count * 2) {
                let sibling_started_ref = &sibling_started;
                let completed_before_panic_ref = &completed_before_panic;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    sibling_started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                        .expect("non-panicking FIFO work should run in the custom pool");
                    assert!(worker_index < thread_count);

                    completed_before_panic_ref
                        .lock()
                        .expect("completed sibling mutex should not be poisoned")
                        .push((value, worker_index));
                });
            }

            rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking FIFO work should run in the custom pool");
                assert!(worker_index < thread_count);
                assert!(
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some()
                );

                panic!("intentional ThreadPool::in_place_scope_fifo scoped panic");
            });

            77usize
        })
    }));

    let payload = panic_result.expect_err(
        "a panic in FIFO scoped work should propagate out of ThreadPool::in_place_scope_fifo",
    );
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional ThreadPool::in_place_scope_fifo scoped panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);
    assert!(sibling_started.load(Ordering::SeqCst) <= thread_count * 2);

    let sibling_count_before_recovery = sibling_started.load(Ordering::SeqCst);
    let completed_before_panic = completed_before_panic
        .into_inner()
        .expect("completed sibling mutex should not be poisoned");

    assert_eq!(completed_before_panic.len(), sibling_count_before_recovery);
    assert!(
        completed_before_panic
            .iter()
            .all(|(value, worker_index)| *value < thread_count * 2 && *worker_index < thread_count)
    );
    assert_eq!(
        completed_before_panic
            .iter()
            .map(|(value, _)| *value)
            .collect::<BTreeSet<_>>()
            .len(),
        completed_before_panic.len(),
        "each completed sibling should report at most once"
    );

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the caller should still not be a pool worker"
    );

    let mut recovery_seeds = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, (index + 3) * (num_threads + 37))
    });

    recovery_seeds.sort_by_key(|entry| entry.0);

    let expected_indices = expected_worker_indices(thread_count);
    assert_eq!(recovery_seeds.len(), thread_count);
    assert_eq!(
        recovery_seeds
            .iter()
            .map(|(index, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let expected_seed_sum: usize = recovery_seeds.iter().map(|(_, seed)| *seed).sum();
    let seed_by_origin: BTreeMap<usize, usize> = recovery_seeds.iter().copied().collect();

    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::ThreadPool::in_place_scope_fifo(pool_ref, |scope| {
        for (origin_index, seed) in recovery_seeds.iter().copied() {
            let recovery_records_ref = &recovery_records;
            let recovery_started_ref = &recovery_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery FIFO work should run inside the custom pool");
                assert!(executing_index < thread_count);

                let (left, right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || thread_count + executing_index,
                );

                recovery_records_ref
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        origin_index,
                        seed,
                        executing_index,
                        value: left + right,
                    });
            });
        }

        expected_seed_sum + completed_before_panic.len()
    });

    assert_eq!(
        recovery_return,
        expected_seed_sum + completed_before_panic.len()
    );
    assert_eq!(recovery_started.load(Ordering::SeqCst), thread_count);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| record.origin_index);

    assert_eq!(recovery_records.len(), thread_count);
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
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || recovery_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            recovery_records
                .iter()
                .map(|record| {
                    record.seed + record.origin_index + thread_count + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}