use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoScopeSummary {
    body_index: usize,
    body_threads: usize,
    pending_status_available: bool,
    scheduled_parent_jobs: usize,
    scheduled_broadcast_jobs: usize,
    seed_checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoOrderSummary {
    body_index: Option<usize>,
    pending_status_available: bool,
    scheduled_jobs: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoRecoveryRecord {
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
fn thread_pool_scope_fifo_drives_broadcast_seeded_nested_fifo_pipeline() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-fifo-pipeline-worker-{index}"))
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
        "the integration-test thread should not be a worker in this custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None,
        "pending-task status should be unavailable outside the custom pool"
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
            seed: (index + 1) * (num_threads + 53),
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 53));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_checksum: usize = seed_by_index.iter().sum();
    let expected_parent_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(origin_index, seed)| *seed + origin_index + thread_count)
        .sum();
    let expected_broadcast_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed + index + thread_count * 10)
        .sum();

    let parent_records = Mutex::new(Vec::<FifoParentRecord>::new());
    let child_records = Mutex::new(Vec::<FifoChildRecord>::new());
    let broadcast_records = Mutex::new(Vec::<FifoBroadcastRecord>::new());
    let broadcast_child_records = Mutex::new(Vec::<FifoBroadcastChildRecord>::new());

    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);
    let broadcast_child_started = AtomicUsize::new(0);

    let summary = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
        let body_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
            .expect("ThreadPool::scope_fifo body should execute inside the custom pool");

        assert!(body_index < thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(body_index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let pending_status_available =
            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();
        assert!(
            pending_status_available,
            "ThreadPool::scope_fifo body should be able to query worker-local pending-task status"
        );

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

                let (seed_component, thread_component) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || thread_count,
                );
                let parent_value = seed_component + thread_component;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(FifoParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        value: parent_value,
                        pending_status_available,
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index =
                        rayon_core::ThreadPool::current_thread_index(pool_ref)
                            .expect("FIFO child work should run inside the custom pool");
                    assert!(child_executing_index < thread_count);

                    let (doubled_parent, worker_component) = rayon_core::join(
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
                            value: doubled_parent + worker_component,
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
            let (seed_component, thread_component) =
                rayon_core::join(move || seed + index, move || num_threads * 10);
            let broadcast_value = seed_component + thread_component;

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

                let executing_index = rayon_core::current_thread_index()
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

        FifoScopeSummary {
            body_index,
            body_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
            pending_status_available,
            scheduled_parent_jobs: seeds.len(),
            scheduled_broadcast_jobs: thread_count,
            seed_checksum: expected_seed_checksum,
        }
    });

    assert!(summary.body_index < thread_count);
    assert_eq!(summary.body_threads, thread_count);
    assert!(summary.pending_status_available);
    assert_eq!(summary.scheduled_parent_jobs, thread_count);
    assert_eq!(summary.scheduled_broadcast_jobs, thread_count);
    assert_eq!(summary.seed_checksum, expected_seed_checksum);

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
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count
        );
        assert!(
            record.pending_status_available,
            "FIFO parent work should observe worker-local pending-task status"
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
            .expect("child record should correspond to a parent FIFO record");

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
        assert_eq!(
            record.value,
            seed_by_index[record.index] + record.index + thread_count * 10
        );
        assert!(
            record.pending_status_available,
            "broadcast work spawned from scope_fifo should observe pending-task status"
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
            seed_by_index[record.origin_index]
                + record.origin_index
                + thread_count * 10
                + thread_count
                + record.executing_index
        );
    }

    let (observed_parent_sum, observed_broadcast_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || parent_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            broadcast_records
                .iter()
                .map(|record| record.value)
                .sum::<usize>()
        },
    );

    assert_eq!(observed_parent_sum, expected_parent_sum);
    assert_eq!(observed_broadcast_sum, expected_broadcast_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_scope_fifo_preserves_relative_fifo_order_on_single_worker() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("scope-fifo-order-worker-{index}"))
        .build()
        .expect("single-worker Rayon pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);

    let observed_order = Mutex::new(Vec::<usize>::new());

    let summary = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        assert_eq!(rayon_core::current_num_threads(), 1);
        assert_eq!(rayon_core::current_thread_index(), Some(0));

        let pending_status_available = rayon_core::current_thread_has_pending_tasks().is_some();
        assert!(
            pending_status_available,
            "scope_fifo body should run on a worker that can report pending-task status"
        );

        for value in 0usize..8 {
            let observed_order_ref = &observed_order;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                assert_eq!(rayon_core::current_num_threads(), 1);
                assert_eq!(rayon_core::current_thread_index(), Some(0));
                assert!(
                    rayon_core::current_thread_has_pending_tasks().is_some(),
                    "FIFO worker task should be able to query pending-task status"
                );

                let (left, right) = rayon_core::join(move || value, move || value * 10);

                observed_order_ref
                    .lock()
                    .expect("observed-order mutex should not be poisoned")
                    .push(left + right);
            });
        }

        FifoOrderSummary {
            body_index: rayon_core::current_thread_index(),
            pending_status_available,
            scheduled_jobs: 8,
        }
    });

    assert_eq!(summary.body_index, Some(0));
    assert!(summary.pending_status_available);
    assert_eq!(summary.scheduled_jobs, 8);

    let observed_order = observed_order
        .into_inner()
        .expect("observed-order mutex should not be poisoned");

    let expected_order: Vec<_> = (0usize..summary.scheduled_jobs)
        .map(|value| value + value * 10)
        .collect();

    assert_eq!(
        observed_order, expected_order,
        "jobs queued by the same scope_fifo body should execute in FIFO order on one worker"
    );

    let (observed_sum, expected_sum) = rayon_core::ThreadPool::join(
        &pool,
        || observed_order.iter().sum::<usize>(),
        || expected_order.iter().sum::<usize>(),
    );

    assert_eq!(observed_sum, expected_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_scope_fifo_propagates_scoped_panic_and_pool_recovers() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-fifo-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");
    let pool_ref = &pool;

    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<usize>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
            rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking FIFO work should run inside the custom pool");
                assert!(worker_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                panic!("intentional ThreadPool::scope_fifo panic for integration coverage");
            });

            for value in 0usize..(thread_count * 2) {
                let completed_before_panic_ref = &completed_before_panic;
                let sibling_started_ref = &sibling_started;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    sibling_started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                        .expect("non-panicking sibling FIFO work should run inside the pool");
                    assert!(worker_index < thread_count);

                    completed_before_panic_ref
                        .lock()
                        .expect("completed sibling mutex should not be poisoned")
                        .push(value);
                });
            }

            123usize
        });
    }));

    let payload = panic_result
        .expect_err("a panic in ThreadPool::scope_fifo work should propagate to the caller");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional ThreadPool::scope_fifo panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);

    let sibling_count_before_recovery = sibling_started.load(Ordering::SeqCst);
    assert!(sibling_count_before_recovery <= thread_count * 2);

    let completed_before_panic = completed_before_panic
        .into_inner()
        .expect("completed sibling mutex should not be poisoned");

    assert_eq!(completed_before_panic.len(), sibling_count_before_recovery);
    assert!(
        completed_before_panic
            .iter()
            .all(|value| *value < thread_count * 2)
    );
    assert_eq!(
        completed_before_panic
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len(),
        completed_before_panic.len(),
        "each non-panicking sibling should report at most once"
    );

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the caller should still not be a pool worker"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None
    );

    let mut recovery_seeds = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        (index, (index + 7) * (num_threads + 61))
    });

    recovery_seeds.sort_by_key(|entry| entry.0);

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
    assert_eq!(seed_by_origin.len(), thread_count);

    let recovery_records = Mutex::new(Vec::<FifoRecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
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
                    .push(FifoRecoveryRecord {
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
                    seed_by_origin[&record.origin_index]
                        + record.origin_index
                        + thread_count
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}