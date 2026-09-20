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
struct PipelineSummary {
    body_index: usize,
    body_threads: usize,
    body_pending_status_available: bool,
    scheduled_fifo_jobs: usize,
    scheduled_broadcast_jobs: usize,
    seed_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChildRecord {
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
struct OrderSummary {
    body_index: Option<usize>,
    body_threads: usize,
    pending_status_available: bool,
    scheduled_jobs: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
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
fn thread_pool_scope_fifo_builds_seeded_fifo_and_broadcast_pipeline() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-fifo-target-pipeline-worker-{index}"))
        .build()
        .expect("custom Rayon thread pool should build");

    let pool_ref = &pool;

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(pool_ref),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "the integration test thread should start outside the custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None
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
            seed: (index + 1) * (num_threads + 37),
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 37));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();
    let expected_broadcast_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed * 3 + index + thread_count * 5)
        .sum();

    let parent_records = Mutex::new(Vec::<ParentRecord>::new());
    let child_records = Mutex::new(Vec::<ChildRecord>::new());
    let broadcast_records = Mutex::new(Vec::<FifoBroadcastRecord>::new());
    let broadcast_child_records = Mutex::new(Vec::<FifoBroadcastChildRecord>::new());

    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);
    let broadcast_started = AtomicUsize::new(0);
    let broadcast_child_started = AtomicUsize::new(0);

    let summary = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
        let body_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
            .expect("ThreadPool::scope_fifo body should run inside the custom pool");
        assert!(body_index < thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(body_index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let body_pending_status_available =
            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();

        assert!(
            body_pending_status_available,
            "scope_fifo body should be able to query worker-local pending-task status"
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

                let origin_index = seed_record.index;
                let seed = seed_record.seed;

                let (left, right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || thread_count + executing_index,
                );
                let parent_value = left + right;

                parent_records_ref
                    .lock()
                    .expect("parent records mutex should not be poisoned")
                    .push(ParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        value: parent_value,
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index =
                        rayon_core::ThreadPool::current_thread_index(pool_ref)
                            .expect("nested FIFO child should run inside the custom pool");
                    assert!(child_executing_index < thread_count);

                    let (doubled_parent, child_worker_component) = rayon_core::join(
                        move || parent_value * 2,
                        move || child_executing_index + origin_index,
                    );

                    child_records_ref
                        .lock()
                        .expect("child records mutex should not be poisoned")
                        .push(ChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            value: doubled_parent + child_worker_component,
                        });
                });
            });
        }

        let seed_by_index_ref = &seed_by_index;
        let broadcast_records_ref = &broadcast_records;
        let broadcast_child_records_ref = &broadcast_child_records;
        let broadcast_started_ref = &broadcast_started;
        let broadcast_child_started_ref = &broadcast_child_started;

        rayon_core::ScopeFifo::spawn_broadcast(scope, move |scope, context| {
            broadcast_started_ref.fetch_add(1, Ordering::SeqCst);

            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert!(index < num_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            let seed = seed_by_index_ref[index];
            let (left, right) =
                rayon_core::join(move || seed * 3, move || index + num_threads * 5);
            let broadcast_value = left + right;

            broadcast_records_ref
                .lock()
                .expect("broadcast records mutex should not be poisoned")
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
                    .expect("FIFO child spawned by broadcast should run on a Rayon worker");
                assert!(executing_index < num_threads);

                broadcast_child_records_ref
                    .lock()
                    .expect("broadcast child records mutex should not be poisoned")
                    .push(FifoBroadcastChildRecord {
                        origin_index: index,
                        executing_index,
                        value: broadcast_value + num_threads + executing_index,
                    });
            });
        });

        PipelineSummary {
            body_index,
            body_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
            body_pending_status_available,
            scheduled_fifo_jobs: seeds.len(),
            scheduled_broadcast_jobs: thread_count,
            seed_sum: expected_seed_sum,
        }
    });

    assert!(summary.body_index < thread_count);
    assert_eq!(summary.body_threads, thread_count);
    assert!(summary.body_pending_status_available);
    assert_eq!(summary.scheduled_fifo_jobs, thread_count);
    assert_eq!(summary.scheduled_broadcast_jobs, thread_count);
    assert_eq!(summary.seed_sum, expected_seed_sum);

    assert_eq!(parent_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(child_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(broadcast_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        broadcast_child_started.load(Ordering::SeqCst),
        thread_count
    );

    let mut parent_records = parent_records
        .into_inner()
        .expect("parent records mutex should not be poisoned");
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
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "FIFO parent work should observe worker-local pending-task status"
        );
    }

    let parent_by_origin: BTreeMap<usize, ParentRecord> = parent_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(parent_by_origin.len(), thread_count);

    let mut child_records = child_records
        .into_inner()
        .expect("child records mutex should not be poisoned");
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
            .expect("child record should correspond to a parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(
            record.value,
            parent.value * 2 + record.executing_index + record.origin_index
        );
    }

    let child_by_origin: BTreeMap<usize, ChildRecord> = child_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

    let mut broadcast_records = broadcast_records
        .into_inner()
        .expect("broadcast records mutex should not be poisoned");
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
            seed_by_index[record.index] * 3 + record.index + thread_count * 5
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

    let broadcast_by_index: BTreeMap<usize, FifoBroadcastRecord> = broadcast_records
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();

    let mut broadcast_child_records = broadcast_child_records
        .into_inner()
        .expect("broadcast child records mutex should not be poisoned");
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

        let source = broadcast_by_index
            .get(&record.origin_index)
            .expect("broadcast child should correspond to a broadcast record");

        assert_eq!(
            record.value,
            source.value + thread_count + record.executing_index
        );
    }

    let broadcast_child_by_origin: BTreeMap<usize, FifoBroadcastChildRecord> =
        broadcast_child_records
            .iter()
            .cloned()
            .map(|record| (record.origin_index, record))
            .collect();

    let mut confirmations = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let parent_value = parent_by_origin[&index].value;
        let child_value = child_by_origin[&index].value;
        let broadcast_value = broadcast_by_index[&index].value;
        let broadcast_child_value = broadcast_child_by_origin[&index].value;

        let (left, right) = rayon_core::join(
            move || parent_value + child_value,
            move || broadcast_value + broadcast_child_value,
        );

        (index, num_threads, rayon_core::current_thread_index(), left + right)
    });

    confirmations.sort_by_key(|record| record.0);

    assert_eq!(confirmations.len(), thread_count);
    assert_eq!(
        confirmations
            .iter()
            .map(|(index, _, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, num_threads, current_index, total) in confirmations {
        assert_eq!(num_threads, thread_count);
        assert_eq!(current_index, Some(index));
        assert_eq!(
            total,
            parent_by_origin[&index].value
                + child_by_origin[&index].value
                + broadcast_by_index[&index].value
                + broadcast_child_by_origin[&index].value
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_scope_fifo_single_worker_preserves_top_level_fifo_order() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("scope-fifo-target-order-worker-{index}"))
        .build()
        .expect("single-worker Rayon pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let observed_order = Mutex::new(Vec::<usize>::new());

    let summary = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        assert_eq!(rayon_core::current_thread_index(), Some(0));
        assert_eq!(rayon_core::current_num_threads(), 1);

        let pending_status_available =
            rayon_core::current_thread_has_pending_tasks().is_some();
        assert!(
            pending_status_available,
            "single-worker scope_fifo body should observe pending-task status"
        );

        for value in 0usize..12 {
            let observed_order_ref = &observed_order;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                assert_eq!(rayon_core::current_thread_index(), Some(0));
                assert_eq!(rayon_core::current_num_threads(), 1);
                assert!(
                    rayon_core::current_thread_has_pending_tasks().is_some(),
                    "FIFO task should observe worker-local pending-task status"
                );

                let (left, right) = rayon_core::join(move || value, move || value * 10);

                observed_order_ref
                    .lock()
                    .expect("observed order mutex should not be poisoned")
                    .push(left + right);
            });
        }

        OrderSummary {
            body_index: rayon_core::current_thread_index(),
            body_threads: rayon_core::current_num_threads(),
            pending_status_available,
            scheduled_jobs: 12,
        }
    });

    assert_eq!(summary.body_index, Some(0));
    assert_eq!(summary.body_threads, 1);
    assert!(summary.pending_status_available);
    assert_eq!(summary.scheduled_jobs, 12);

    let observed_order = observed_order
        .into_inner()
        .expect("observed order mutex should not be poisoned");

    let expected_order: Vec<_> = (0usize..summary.scheduled_jobs)
        .map(|value| value + value * 10)
        .collect();

    assert_eq!(
        observed_order, expected_order,
        "ThreadPool::scope_fifo should preserve FIFO order for tasks queued by one worker"
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
fn thread_pool_scope_fifo_propagates_spawned_panic_and_later_recovers() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-fifo-target-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon thread pool should build");
    let pool_ref = &pool;

    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<usize>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
            rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking FIFO task should run inside the custom pool");
                assert!(worker_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );
                assert!(
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some()
                );

                panic!("intentional ThreadPool::scope_fifo spawned panic for target coverage");
            });

            for input in 0usize..(thread_count * 2) {
                let sibling_started_ref = &sibling_started;
                let completed_before_panic_ref = &completed_before_panic;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    sibling_started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::current_thread_index()
                        .expect("non-panicking sibling FIFO work should run on a Rayon worker");
                    assert!(worker_index < thread_count);

                    completed_before_panic_ref
                        .lock()
                        .expect("completed sibling mutex should not be poisoned")
                        .push(input);
                });
            }

            2025usize
        });
    }));

    let payload = panic_result
        .expect_err("panic in ThreadPool::scope_fifo spawned work should propagate");
    let message = panic_payload_to_string(&*payload);

    assert!(
        message.contains("ThreadPool::scope_fifo spawned panic"),
        "unexpected propagated panic payload: {message:?}"
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
            .all(|input| *input < thread_count * 2)
    );
    assert_eq!(
        completed_before_panic
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len(),
        completed_before_panic.len(),
        "each completed sibling should report at most once"
    );

    let completed_before_panic_len = completed_before_panic.len();

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the external caller should still not be a pool worker"
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

        (index, (index + 5) * (num_threads + 19))
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

    let seed_by_origin: BTreeMap<usize, usize> = recovery_seeds.iter().copied().collect();
    assert_eq!(seed_by_origin.len(), thread_count);

    let expected_seed_sum: usize = recovery_seeds.iter().map(|(_, seed)| *seed).sum();
    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
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
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                let (left, right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || thread_count + executing_index + completed_before_panic_len,
                );

                recovery_records_ref
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        origin_index,
                        seed,
                        executing_index,
                        value: left + right,
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    });
            });
        }

        expected_seed_sum + completed_before_panic_len
    });

    assert_eq!(recovery_return, expected_seed_sum + completed_before_panic_len);
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
            record.seed
                + record.origin_index
                + thread_count
                + record.executing_index
                + completed_before_panic_len
        );
        assert!(
            record.pending_status_available,
            "recovery FIFO work should observe worker-local pending-task status"
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
                        + completed_before_panic_len
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}