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
struct ParentRecord {
    origin_index: usize,
    executing_index: usize,
    seed: usize,
    parent_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    parent_value: usize,
    child_value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GrandchildRecord {
    origin_index: usize,
    child_executing_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
    stage: usize,
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    value: usize,
}

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn scope_fifo_spawn_fifo_builds_nested_pipeline_from_broadcast_seed_data() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-fifo-spawn-fifo-pipeline-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "the integration-test thread should not be a worker in the custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None,
        "pending-task status is unavailable outside a Rayon worker"
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
            seed: (index + 1) * (num_threads + 41),
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 41));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_parent_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(origin_index, seed)| *seed + origin_index + thread_count)
        .sum();

    let parent_records = Mutex::new(Vec::<ParentRecord>::new());
    let child_records = Mutex::new(Vec::<ChildRecord>::new());
    let grandchild_records = Mutex::new(Vec::<GrandchildRecord>::new());

    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);
    let grandchild_started = AtomicUsize::new(0);

    let scope_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for origin_index in 0..thread_count {
            let seed = seed_by_index[origin_index];

            let parent_records = &parent_records;
            let child_records = &child_records;
            let grandchild_records = &grandchild_records;

            let parent_started = &parent_started;
            let child_started = &child_started;
            let grandchild_started = &grandchild_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |nested_scope| {
                parent_started.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("parent FIFO job should execute on a Rayon worker");

                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let pending_status_available =
                    rayon_core::current_thread_has_pending_tasks().is_some();
                assert!(
                    pending_status_available,
                    "parent FIFO job should be able to query worker pending-task status"
                );

                let (left, right) =
                    rayon_core::join(move || seed + origin_index, move || thread_count);
                let parent_value = left + right;

                parent_records
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(ParentRecord {
                        origin_index,
                        executing_index,
                        seed,
                        parent_value,
                        pending_status_available,
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |grandchild_scope| {
                    child_started.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index = rayon_core::current_thread_index()
                        .expect("child FIFO job should execute on a Rayon worker");

                    assert!(child_executing_index < thread_count);
                    assert_eq!(rayon_core::current_num_threads(), thread_count);

                    let (doubled_parent, worker_component) = rayon_core::join(
                        move || parent_value * 2,
                        move || child_executing_index,
                    );
                    let child_value = doubled_parent + worker_component;

                    child_records
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(ChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            parent_value,
                            child_value,
                        });

                    rayon_core::ScopeFifo::spawn_fifo(grandchild_scope, move |_| {
                        grandchild_started.fetch_add(1, Ordering::SeqCst);

                        let grandchild_executing_index = rayon_core::current_thread_index()
                            .expect("grandchild FIFO job should execute on a Rayon worker");

                        assert!(grandchild_executing_index < thread_count);

                        let (from_child, from_worker_and_seed) = rayon_core::join(
                            move || child_value + origin_index,
                            move || grandchild_executing_index + seed,
                        );

                        grandchild_records
                            .lock()
                            .expect("grandchild record mutex should not be poisoned")
                            .push(GrandchildRecord {
                                origin_index,
                                child_executing_index,
                                executing_index: grandchild_executing_index,
                                value: from_child + from_worker_and_seed,
                            });
                    });
                });
            });
        }

        expected_parent_sum
    });

    assert_eq!(scope_return, expected_parent_sum);
    assert_eq!(parent_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(child_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(grandchild_started.load(Ordering::SeqCst), thread_count);

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
            record.parent_value,
            record.seed + record.origin_index + thread_count
        );
        assert!(record.pending_status_available);
    }

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.parent_value)
            .sum::<usize>(),
        expected_parent_sum
    );

    let parent_by_origin: BTreeMap<usize, ParentRecord> = parent_records
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
        assert_eq!(record.parent_value, parent.parent_value);
        assert_eq!(
            record.child_value,
            record.parent_value * 2 + record.executing_index
        );
    }

    let child_by_origin: BTreeMap<usize, ChildRecord> = child_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(child_by_origin.len(), thread_count);

    let mut grandchild_records = grandchild_records
        .into_inner()
        .expect("grandchild record mutex should not be poisoned");
    grandchild_records.sort_by_key(|record| record.origin_index);

    assert_eq!(grandchild_records.len(), thread_count);
    assert_eq!(
        grandchild_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &grandchild_records {
        assert!(record.origin_index < thread_count);
        assert!(record.child_executing_index < thread_count);
        assert!(record.executing_index < thread_count);

        let child = child_by_origin
            .get(&record.origin_index)
            .expect("grandchild record should correspond to a child FIFO record");

        assert_eq!(record.child_executing_index, child.executing_index);
        assert_eq!(
            record.value,
            child.child_value
                + record.origin_index
                + record.executing_index
                + seed_by_index[record.origin_index]
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn scope_fifo_spawn_fifo_panic_propagates_and_pool_recovers_for_more_fifo_work() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-fifo-spawn-fifo-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<usize>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
            rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::current_thread_index()
                    .expect("panicking FIFO job should execute on a Rayon worker");

                assert!(worker_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                panic!("intentional ScopeFifo::spawn_fifo panic for propagation");
            });

            for value in 0..(thread_count * 2) {
                let completed_before_panic = &completed_before_panic;
                let sibling_started = &sibling_started;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    sibling_started.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::current_thread_index()
                        .expect("sibling FIFO job should execute on a Rayon worker");

                    assert!(worker_index < thread_count);

                    completed_before_panic
                        .lock()
                        .expect("sibling record mutex should not be poisoned")
                        .push(value);
                });
            }

            123usize
        })
    }));

    let payload = panic_result
        .expect_err("a panic in ScopeFifo::spawn_fifo should propagate out of scope_fifo");

    let panic_message = if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_owned()
    };

    assert!(
        panic_message.contains("intentional ScopeFifo::spawn_fifo panic for propagation"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);
    assert!(sibling_started.load(Ordering::SeqCst) <= thread_count * 2);

    let sibling_count_before_recovery = sibling_started.load(Ordering::SeqCst);
    let mut completed_before_panic = completed_before_panic
        .into_inner()
        .expect("sibling record mutex should not be poisoned");
    completed_before_panic.sort_unstable();

    assert_eq!(completed_before_panic.len(), sibling_count_before_recovery);
    assert!(
        completed_before_panic
            .iter()
            .all(|value| *value < thread_count * 2)
    );
    assert_eq!(
        completed_before_panic.iter().copied().collect::<BTreeSet<_>>().len(),
        completed_before_panic.len(),
        "each non-panicking sibling FIFO job should report at most once"
    );

    let mut recovery_seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, (index + 5) * (num_threads + 29))
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

    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for (origin_index, seed) in recovery_seeds.iter().copied() {
            let recovery_records = &recovery_records;
            let recovery_started = &recovery_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |nested_scope| {
                recovery_started.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery parent FIFO job should execute on a Rayon worker");

                assert!(executing_index < thread_count);

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || executing_index + thread_count,
                );
                let parent_value = left + right;

                recovery_records
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        stage: 0,
                        origin_index,
                        seed,
                        executing_index,
                        value: parent_value,
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    recovery_started.fetch_add(1, Ordering::SeqCst);

                    let nested_index = rayon_core::current_thread_index()
                        .expect("recovery child FIFO job should execute on a Rayon worker");

                    assert!(nested_index < thread_count);

                    let (from_parent, from_nested_worker) =
                        rayon_core::join(move || parent_value, move || nested_index);

                    recovery_records
                        .lock()
                        .expect("recovery record mutex should not be poisoned")
                        .push(RecoveryRecord {
                            stage: 1,
                            origin_index,
                            seed,
                            executing_index: nested_index,
                            value: from_parent + from_nested_worker,
                        });
                });
            });
        }

        expected_seed_sum + sibling_count_before_recovery
    });

    assert_eq!(
        recovery_return,
        expected_seed_sum + sibling_count_before_recovery
    );
    assert_eq!(recovery_started.load(Ordering::SeqCst), thread_count * 2);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| (record.stage, record.origin_index));

    assert_eq!(recovery_records.len(), thread_count * 2);

    let parent_records: Vec<_> = recovery_records
        .iter()
        .filter(|record| record.stage == 0)
        .cloned()
        .collect();
    let child_records: Vec<_> = recovery_records
        .iter()
        .filter(|record| record.stage == 1)
        .cloned()
        .collect();

    assert_eq!(parent_records.len(), thread_count);
    assert_eq!(child_records.len(), thread_count);

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );
    assert_eq!(
        child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let seed_by_origin: BTreeMap<usize, usize> = recovery_seeds.into_iter().collect();
    assert_eq!(seed_by_origin.len(), thread_count);

    for record in &parent_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );
        assert_eq!(
            record.value,
            record.seed + record.origin_index + record.executing_index + thread_count
        );
    }

    let parent_by_origin: BTreeMap<usize, RecoveryRecord> = parent_records
        .into_iter()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(parent_by_origin.len(), thread_count);

    for record in &child_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("recovery child FIFO record should have a parent record");

        assert_eq!(record.seed, parent.seed);
        assert_eq!(record.value, parent.value + record.executing_index);
    }

    let (observed_child_sum, recomputed_child_sum) = rayon_core::ThreadPool::join(
        &pool,
        || child_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            child_records
                .iter()
                .map(|record| {
                    let parent = parent_by_origin
                        .get(&record.origin_index)
                        .expect("parent record should exist during recomputation");
                    parent.value + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_child_sum, recomputed_child_sum);
}