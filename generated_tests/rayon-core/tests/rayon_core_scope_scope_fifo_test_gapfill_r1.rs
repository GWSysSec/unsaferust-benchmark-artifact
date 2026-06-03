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
struct ModuleScopeFifoSummary {
    body_index: Option<usize>,
    body_threads: usize,
    body_pending_status_available: bool,
    scheduled_parent_jobs: usize,
    scheduled_broadcast_jobs: usize,
    seed_sum: usize,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct OrderSummary {
    body_index: Option<usize>,
    body_threads: usize,
    pending_status_available: bool,
    scheduled_jobs: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PanicSiblingRecord {
    input: usize,
    worker_index: usize,
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
fn module_path_scope_fifo_builds_broadcast_seeded_nested_fifo_pipeline() {
    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "integration-test thread should start outside any Rayon worker"
    );

    let expected_indices = expected_worker_indices(global_threads);

    let mut seeds = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert!(index < num_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), global_threads);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 31),
        }
    });

    seeds.sort_by_key(|record| record.index);

    assert_eq!(seeds.len(), global_threads);
    assert_eq!(
        seeds.iter().map(|record| record.index).collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (expected_index, record) in seeds.iter().enumerate() {
        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(expected_index));
        assert_eq!(record.seed, (expected_index + 1) * (global_threads + 31));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();
    let expected_parent_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(origin_index, seed)| *seed + origin_index + global_threads * 2)
        .sum();
    let expected_broadcast_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed * 3 + index + global_threads * 7)
        .sum();

    let parent_records = Mutex::new(Vec::<FifoParentRecord>::new());
    let child_records = Mutex::new(Vec::<FifoChildRecord>::new());
    let broadcast_records = Mutex::new(Vec::<FifoBroadcastRecord>::new());
    let broadcast_child_records = Mutex::new(Vec::<FifoBroadcastChildRecord>::new());

    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);
    let broadcast_started = AtomicUsize::new(0);
    let broadcast_child_started = AtomicUsize::new(0);

    let summary = rayon_core::scope_fifo(|scope| {
        let body_index = rayon_core::current_thread_index();
        if let Some(index) = body_index {
            assert!(index < global_threads);
        }

        let body_pending_status_available =
            rayon_core::current_thread_has_pending_tasks().is_some();

        for seed_record in seeds.iter().cloned() {
            let parent_records_ref = &parent_records;
            let child_records_ref = &child_records;
            let parent_started_ref = &parent_started;
            let child_started_ref = &child_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |nested_scope| {
                parent_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO parent work should run on a Rayon worker");
                assert!(executing_index < seed_record.num_threads);
                assert_eq!(rayon_core::current_num_threads(), seed_record.num_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (left, right) =
                    rayon_core::join(move || seed + origin_index, move || num_threads * 2);
                let parent_value = left + right;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(FifoParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        value: parent_value,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index = rayon_core::current_thread_index()
                        .expect("nested FIFO child should run on a Rayon worker");
                    assert!(child_executing_index < num_threads);

                    let (doubled_parent, worker_component) =
                        rayon_core::join(move || parent_value * 2, move || child_executing_index);

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
        let broadcast_started_ref = &broadcast_started;
        let broadcast_child_started_ref = &broadcast_child_started;

        rayon_core::ScopeFifo::spawn_broadcast(scope, move |scope, context| {
            broadcast_started_ref.fetch_add(1, Ordering::SeqCst);

            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, global_threads);
            assert!(index < num_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), global_threads);

            let seed = seed_by_index_ref[index];
            let (left, right) =
                rayon_core::join(move || seed * 3, move || index + num_threads * 7);
            let broadcast_value = left + right;

            broadcast_records_ref
                .lock()
                .expect("broadcast record mutex should not be poisoned")
                .push(FifoBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    value: broadcast_value,
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                });

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                broadcast_child_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO child spawned by broadcast should run on a Rayon worker");
                assert!(executing_index < num_threads);

                broadcast_child_records_ref
                    .lock()
                    .expect("broadcast child mutex should not be poisoned")
                    .push(FifoBroadcastChildRecord {
                        origin_index: index,
                        executing_index,
                        value: broadcast_value + num_threads + executing_index,
                    });
            });
        });

        ModuleScopeFifoSummary {
            body_index,
            body_threads: rayon_core::current_num_threads(),
            body_pending_status_available,
            scheduled_parent_jobs: seeds.len(),
            scheduled_broadcast_jobs: global_threads,
            seed_sum: expected_seed_sum,
        }
    });

    if let Some(body_index) = summary.body_index {
        assert!(body_index < global_threads);
        assert!(
            summary.body_pending_status_available,
            "worker-local scope_fifo body should report pending-task status"
        );
    }
    assert_eq!(summary.body_threads, global_threads);
    assert_eq!(summary.scheduled_parent_jobs, global_threads);
    assert_eq!(summary.scheduled_broadcast_jobs, global_threads);
    assert_eq!(summary.seed_sum, expected_seed_sum);

    assert_eq!(parent_started.load(Ordering::SeqCst), global_threads);
    assert_eq!(child_started.load(Ordering::SeqCst), global_threads);
    assert_eq!(broadcast_started.load(Ordering::SeqCst), global_threads);
    assert_eq!(
        broadcast_child_started.load(Ordering::SeqCst),
        global_threads
    );

    let mut parent_records = parent_records
        .into_inner()
        .expect("parent record mutex should not be poisoned");
    parent_records.sort_by_key(|record| record.origin_index);

    assert_eq!(parent_records.len(), global_threads);
    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &parent_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + global_threads * 2
        );
        assert!(
            record.pending_status_available,
            "FIFO parent work should observe worker-local pending-task status"
        );
    }

    assert_eq!(
        parent_records.iter().map(|record| record.value).sum::<usize>(),
        expected_parent_sum
    );

    let parent_by_origin: BTreeMap<usize, FifoParentRecord> = parent_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

    let mut child_records = child_records
        .into_inner()
        .expect("child record mutex should not be poisoned");
    child_records.sort_by_key(|record| record.origin_index);

    assert_eq!(child_records.len(), global_threads);
    assert_eq!(
        child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &child_records {
        assert!(record.origin_index < global_threads);
        assert!(record.parent_executing_index < global_threads);
        assert!(record.executing_index < global_threads);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("child should correspond to a parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(record.value, parent.value * 2 + record.executing_index);
    }

    let mut broadcast_records = broadcast_records
        .into_inner()
        .expect("broadcast record mutex should not be poisoned");
    broadcast_records.sort_by_key(|record| record.index);

    assert_eq!(broadcast_records.len(), global_threads);
    assert_eq!(
        broadcast_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &broadcast_records {
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.value,
            seed_by_index[record.index] * 3 + record.index + global_threads * 7
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
        .expect("broadcast child mutex should not be poisoned");
    broadcast_child_records.sort_by_key(|record| record.origin_index);

    assert_eq!(broadcast_child_records.len(), global_threads);
    assert_eq!(
        broadcast_child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &broadcast_child_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(
            record.value,
            seed_by_index[record.origin_index] * 3
                + record.origin_index
                + global_threads * 7
                + global_threads
                + record.executing_index
        );
    }

    let (observed_parent_sum, observed_broadcast_sum) = rayon_core::join(
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
fn module_path_scope_fifo_uses_current_single_worker_pool_and_preserves_fifo_order() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("module-scope-fifo-order-worker-{index}"))
        .build()
        .expect("single-worker custom pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let observed_order = Mutex::new(Vec::<usize>::new());

    let summary = rayon_core::ThreadPool::scope(&pool, |_| {
        assert_eq!(rayon_core::current_thread_index(), Some(0));
        assert_eq!(rayon_core::current_num_threads(), 1);

        rayon_core::scope_fifo(|scope| {
            assert_eq!(
                rayon_core::current_thread_index(),
                Some(0),
                "module-path scope_fifo called from a pool worker should use the current pool"
            );
            assert_eq!(rayon_core::current_num_threads(), 1);

            for value in 0usize..12 {
                let observed_order_ref = &observed_order;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    assert_eq!(rayon_core::current_thread_index(), Some(0));
                    assert_eq!(rayon_core::current_num_threads(), 1);
                    assert!(
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                        "FIFO work should be able to query worker-local pending-task status"
                    );

                    let (left, right) =
                        rayon_core::join(move || value, move || value * 10);

                    observed_order_ref
                        .lock()
                        .expect("observed-order mutex should not be poisoned")
                        .push(left + right);
                });
            }

            OrderSummary {
                body_index: rayon_core::current_thread_index(),
                body_threads: rayon_core::current_num_threads(),
                pending_status_available: rayon_core::current_thread_has_pending_tasks()
                    .is_some(),
                scheduled_jobs: 12,
            }
        })
    });

    assert_eq!(summary.body_index, Some(0));
    assert_eq!(summary.body_threads, 1);
    assert!(summary.pending_status_available);
    assert_eq!(summary.scheduled_jobs, 12);

    let observed_order = observed_order
        .into_inner()
        .expect("observed-order mutex should not be poisoned");

    let expected_order: Vec<_> = (0usize..summary.scheduled_jobs)
        .map(|value| value + value * 10)
        .collect();

    assert_eq!(
        observed_order, expected_order,
        "module-path scope_fifo should preserve FIFO order for jobs queued on one worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn module_path_scope_fifo_propagates_spawned_panic_and_custom_pool_recovers() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("module-scope-fifo-panic-worker-{index}"))
        .build()
        .expect("custom pool should build");

    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<PanicSiblingRecord>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::scope(&pool, |_| {
            rayon_core::scope_fifo(|scope| {
                rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
                    panic_started.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::current_thread_index()
                        .expect("panicking FIFO work should run on a Rayon worker");
                    assert!(worker_index < thread_count);
                    assert_eq!(rayon_core::current_num_threads(), thread_count);

                    panic!("intentional rayon_core::scope::scope_fifo spawned panic");
                });

                for input in 0usize..(thread_count * 3) {
                    let completed_ref = &completed_before_panic;
                    let sibling_started_ref = &sibling_started;

                    rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                        sibling_started_ref.fetch_add(1, Ordering::SeqCst);

                        let worker_index = rayon_core::current_thread_index()
                            .expect("sibling FIFO work should run on a Rayon worker");
                        assert!(worker_index < thread_count);

                        completed_ref
                            .lock()
                            .expect("completed sibling mutex should not be poisoned")
                            .push(PanicSiblingRecord {
                                input,
                                worker_index,
                            });
                    });
                }

                808usize
            })
        });
    }));

    let payload = panic_result
        .expect_err("panic in module-path scope_fifo work should propagate");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("rayon_core::scope::scope_fifo spawned panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);

    let sibling_count_before_recovery = sibling_started.load(Ordering::SeqCst);
    assert!(sibling_count_before_recovery <= thread_count * 3);

    let completed_before_panic = completed_before_panic
        .into_inner()
        .expect("completed sibling mutex should not be poisoned");

    assert_eq!(completed_before_panic.len(), sibling_count_before_recovery);
    assert!(
        completed_before_panic
            .iter()
            .all(|record| record.input < thread_count * 3 && record.worker_index < thread_count)
    );
    assert_eq!(
        completed_before_panic
            .iter()
            .map(|record| record.input)
            .collect::<BTreeSet<_>>()
            .len(),
        completed_before_panic.len(),
        "each completed sibling should report at most once"
    );

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "after unwinding, the external caller should still not be a pool worker"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut recovery_seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        (index, (index + 5) * (num_threads + 43))
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

    let recovery_return = rayon_core::ThreadPool::scope(&pool, |_| {
        rayon_core::scope_fifo(|scope| {
            for (origin_index, seed) in recovery_seeds.iter().copied() {
                let recovery_records_ref = &recovery_records;
                let recovery_started_ref = &recovery_started;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                    let executing_index = rayon_core::current_thread_index()
                        .expect("recovery FIFO work should run on a Rayon worker");
                    assert!(executing_index < thread_count);
                    assert_eq!(rayon_core::current_num_threads(), thread_count);

                    let (left, right) = rayon_core::join(
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
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });
                });
            }

            expected_seed_sum + completed_before_panic.len()
        })
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
        assert_eq!(seed_by_origin.get(&record.origin_index), Some(&record.seed));
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "recovery FIFO work should observe worker-local pending-task status"
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::ThreadPool::join(
        &pool,
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