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
struct ScopeFifoSummary {
    body_index: Option<usize>,
    body_threads: usize,
    pending_status_available: bool,
    scheduled_parent_jobs: usize,
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
    parent_value: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BroadcastChildRecord {
    origin_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfirmationRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    total: usize,
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
fn free_scope_fifo_waits_for_nested_fifo_and_broadcast_pipeline() {
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "the integration-test thread should start outside any Rayon worker"
    );

    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());

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
            seed: (index + 1) * (num_threads + 13),
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
        assert_eq!(record.seed, (expected_index + 1) * (global_threads + 13));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let parent_records = Mutex::new(Vec::<ParentRecord>::new());
    let child_records = Mutex::new(Vec::<ChildRecord>::new());
    let broadcast_records = Mutex::new(Vec::<BroadcastRecord>::new());
    let broadcast_child_records = Mutex::new(Vec::<BroadcastChildRecord>::new());

    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);
    let broadcast_started = AtomicUsize::new(0);
    let broadcast_child_started = AtomicUsize::new(0);

    let summary = rayon_core::scope_fifo(|scope| {
        let body_index = rayon_core::current_thread_index();

        if let Some(index) = body_index {
            assert!(index < global_threads);
            assert_eq!(rayon_core::current_num_threads(), global_threads);
        }

        let pending_status_available =
            rayon_core::current_thread_has_pending_tasks().is_some();

        for seed_record in seeds.iter().cloned() {
            let parent_records_ref = &parent_records;
            let child_records_ref = &child_records;
            let parent_started_ref = &parent_started;
            let child_started_ref = &child_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |nested_scope| {
                parent_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("parent FIFO work should run on a Rayon worker");
                assert!(executing_index < seed_record.num_threads);
                assert_eq!(rayon_core::current_num_threads(), seed_record.num_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || num_threads * 2 + executing_index,
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
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index = rayon_core::current_thread_index()
                        .expect("nested child FIFO work should run on a Rayon worker");
                    assert!(child_executing_index < num_threads);

                    let (doubled_parent, worker_component) = rayon_core::join(
                        move || parent_value * 2,
                        move || child_executing_index,
                    );

                    child_records_ref
                        .lock()
                        .expect("child records mutex should not be poisoned")
                        .push(ChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            parent_value,
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
            let (seed_component, index_component) =
                rayon_core::join(move || seed * 3, move || index + num_threads * 5);
            let broadcast_value = seed_component + index_component;

            broadcast_records_ref
                .lock()
                .expect("broadcast records mutex should not be poisoned")
                .push(BroadcastRecord {
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
                    .expect("broadcast child records mutex should not be poisoned")
                    .push(BroadcastChildRecord {
                        origin_index: index,
                        executing_index,
                        value: broadcast_value + num_threads + executing_index,
                    });
            });
        });

        ScopeFifoSummary {
            body_index,
            body_threads: rayon_core::current_num_threads(),
            pending_status_available,
            scheduled_parent_jobs: seeds.len(),
            scheduled_broadcast_jobs: global_threads,
            seed_sum: expected_seed_sum,
        }
    });

    if let Some(index) = summary.body_index {
        assert!(index < global_threads);
        assert!(
            summary.pending_status_available,
            "a worker-local scope_fifo body should be able to query pending-task status"
        );
    } else {
        assert!(
            !summary.pending_status_available,
            "pending-task status should be unavailable when the scope_fifo body runs on the external caller"
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
        .expect("parent records mutex should not be poisoned");
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
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + global_threads * 2 + record.executing_index
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

    let mut child_records = child_records
        .into_inner()
        .expect("child records mutex should not be poisoned");
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
            .expect("child record should correspond to a parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(record.parent_value, parent.value);
        assert_eq!(record.value, parent.value * 2 + record.executing_index);
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
            seed_by_index[record.index] * 3 + record.index + global_threads * 5
        );
        assert!(
            record.pending_status_available,
            "broadcast work spawned from scope_fifo should observe pending-task status"
        );
    }

    let broadcast_by_index: BTreeMap<usize, BroadcastRecord> = broadcast_records
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();

    let mut broadcast_child_records = broadcast_child_records
        .into_inner()
        .expect("broadcast child records mutex should not be poisoned");
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

        let source = broadcast_by_index
            .get(&record.origin_index)
            .expect("broadcast child should correspond to broadcast output");

        assert_eq!(
            record.value,
            source.value + global_threads + record.executing_index
        );
    }

    let mut confirmations = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let parent = parent_by_origin
            .get(&index)
            .expect("confirmation should find parent output by index");
        let child = child_by_origin
            .get(&index)
            .expect("confirmation should find child output by index");
        let broadcast = broadcast_by_index
            .get(&index)
            .expect("confirmation should find broadcast output by index");

        let parent_value = parent.value;
        let child_value = child.value;
        let broadcast_value = broadcast.value;

        let (left, right) =
            rayon_core::join(move || parent_value + child_value, move || broadcast_value);

        ConfirmationRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            total: left + right,
        }
    });

    confirmations.sort_by_key(|record| record.index);

    assert_eq!(confirmations.len(), global_threads);
    assert_eq!(
        confirmations
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &confirmations {
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(
            record.total,
            parent_by_origin[&record.index].value
                + child_by_origin[&record.index].value
                + broadcast_by_index[&record.index].value
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_scope_fifo_uses_current_single_worker_pool_and_preserves_fifo_order() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("free-scope-fifo-order-worker-{index}"))
        .build()
        .expect("single-worker custom Rayon pool should build");

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
                "free scope_fifo called from a pool worker should use that current pool"
            );
            assert_eq!(rayon_core::current_num_threads(), 1);

            for value in 0usize..12 {
                let observed_order_ref = &observed_order;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    assert_eq!(rayon_core::current_thread_index(), Some(0));
                    assert_eq!(rayon_core::current_num_threads(), 1);
                    assert!(
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                        "FIFO work should observe worker-local pending-task status"
                    );

                    let (left, right) =
                        rayon_core::join(move || value + 1, move || value * 10);

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
    assert!(
        summary.pending_status_available,
        "scope_fifo body on the worker should report pending-task status"
    );
    assert_eq!(summary.scheduled_jobs, 12);

    let observed_order = observed_order
        .into_inner()
        .expect("observed-order mutex should not be poisoned");

    let expected_order: Vec<_> = (0usize..summary.scheduled_jobs)
        .map(|value| value + 1 + value * 10)
        .collect();

    assert_eq!(
        observed_order, expected_order,
        "free scope_fifo should preserve FIFO order for jobs queued on one worker"
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
fn free_scope_fifo_propagates_spawned_panic_and_later_reuses_global_pool() {
    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);

    let panic_started = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::scope_fifo(|scope| {
            rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::current_thread_index()
                    .expect("panicking FIFO work should run on a Rayon worker");
                assert!(worker_index < global_threads);
                assert_eq!(rayon_core::current_num_threads(), global_threads);
                assert!(
                    rayon_core::current_thread_has_pending_tasks().is_some(),
                    "panicking FIFO work should observe pending-task status"
                );

                panic!("intentional rayon_core::scope_fifo spawned panic for recovery");
            });

            2024usize
        });
    }));

    let payload =
        panic_result.expect_err("a panic in scope_fifo spawned work should propagate");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("rayon_core::scope_fifo spawned panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "after unwinding, the external caller should still not be a Rayon worker"
    );
    assert_eq!(rayon_core::current_thread_has_pending_tasks(), None);

    let mut recovery_seeds = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 5) * (num_threads + 29),
        }
    });

    recovery_seeds.sort_by_key(|record| record.index);

    assert_eq!(recovery_seeds.len(), global_threads);
    assert_eq!(
        recovery_seeds
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for record in &recovery_seeds {
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, (record.index + 5) * (global_threads + 29));
    }

    let expected_seed_sum: usize = recovery_seeds.iter().map(|record| record.seed).sum();
    let seed_by_origin: BTreeMap<usize, usize> = recovery_seeds
        .iter()
        .map(|record| (record.index, record.seed))
        .collect();

    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::scope_fifo(|scope| {
        for seed_record in recovery_seeds.iter().cloned() {
            let recovery_records_ref = &recovery_records;
            let recovery_started_ref = &recovery_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery FIFO work should run on a Rayon worker");
                assert!(executing_index < seed_record.num_threads);
                assert_eq!(rayon_core::current_num_threads(), seed_record.num_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || num_threads + executing_index,
                );

                recovery_records_ref
                    .lock()
                    .expect("recovery records mutex should not be poisoned")
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

        expected_seed_sum + panic_started.load(Ordering::SeqCst)
    });

    assert_eq!(
        recovery_return,
        expected_seed_sum + panic_started.load(Ordering::SeqCst)
    );
    assert_eq!(recovery_started.load(Ordering::SeqCst), global_threads);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery records mutex should not be poisoned");
    recovery_records.sort_by_key(|record| record.origin_index);

    assert_eq!(recovery_records.len(), global_threads);
    assert_eq!(
        recovery_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for record in &recovery_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );
        assert_eq!(
            record.value,
            record.seed + record.origin_index + global_threads + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "recovery FIFO work should observe worker-local pending-task status"
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::join(
        || recovery_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            recovery_records
                .iter()
                .map(|record| {
                    seed_by_origin[&record.origin_index]
                        + record.origin_index
                        + global_threads
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}