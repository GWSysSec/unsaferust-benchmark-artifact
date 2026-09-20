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
struct ScopeSummary {
    body_index: usize,
    body_threads: usize,
    body_pending_status_available: bool,
    scheduled_parent_jobs: usize,
    scheduled_child_jobs: usize,
    seed_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    joined_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    inherited_value: usize,
    child_value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfirmationRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    total: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
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

fn scope_join_with_thread_pool<A, B, RA, RB>(
    pool: &rayon_core::ThreadPool,
    oper_a: A,
    oper_b: B,
) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    let mut result_a = None;
    let mut result_b = None;

    rayon_core::ThreadPool::scope(pool, |scope| {
        rayon_core::Scope::spawn(scope, |_| {
            result_a = Some(oper_a());
        });

        rayon_core::Scope::spawn(scope, |_| {
            result_b = Some(oper_b());
        });
    });

    (
        result_a.expect("left scoped join branch should complete"),
        result_b.expect("right scoped join branch should complete"),
    )
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_scope_emulates_join_and_drives_broadcast_seeded_nested_pipeline() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("thread-pool-scope-integration-worker-{index}"))
        .build()
        .expect("custom Rayon thread pool should build");
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

    let (label, arithmetic_seed) = scope_join_with_thread_pool(
        pool_ref,
        || "thread-pool-scope".to_owned(),
        || (1usize..=6).sum::<usize>(),
    );

    assert_eq!(label, "thread-pool-scope");
    assert_eq!(arithmetic_seed, 21);

    let base = label.len() + arithmetic_seed;

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
            seed: base + (index + 1) * (num_threads + 31),
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
        assert_eq!(
            record.seed,
            base + (expected_index + 1) * (thread_count + 31)
        );
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let parent_records = Mutex::new(Vec::<ParentRecord>::new());
    let child_records = Mutex::new(Vec::<ChildRecord>::new());
    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);

    let summary = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        let body_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
            .expect("ThreadPool::scope body should run inside the custom pool");
        assert!(body_index < thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(body_index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let body_pending_status_available =
            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();
        assert!(
            body_pending_status_available,
            "ThreadPool::scope body should be able to query worker-local pending-task status"
        );

        for seed_record in seeds.iter().cloned() {
            let parent_records_ref = &parent_records;
            let child_records_ref = &child_records;
            let parent_started_ref = &parent_started;
            let child_started_ref = &child_started;

            rayon_core::Scope::spawn(scope, move |nested_scope| {
                parent_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("parent scoped work should run inside the custom pool");
                assert!(executing_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (seed_component, worker_component) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || num_threads + executing_index,
                );
                let joined_value = seed_component + worker_component;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(ParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        joined_value,
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    });

                rayon_core::Scope::spawn(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index =
                        rayon_core::ThreadPool::current_thread_index(pool_ref)
                            .expect("nested child scoped work should run inside the custom pool");
                    assert!(child_executing_index < num_threads);

                    let (doubled_parent, child_worker_component) = rayon_core::join(
                        move || joined_value * 2,
                        move || child_executing_index,
                    );

                    child_records_ref
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(ChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            inherited_value: joined_value,
                            child_value: doubled_parent + child_worker_component,
                        });
                });
            });
        }

        ScopeSummary {
            body_index,
            body_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
            body_pending_status_available,
            scheduled_parent_jobs: seeds.len(),
            scheduled_child_jobs: seeds.len(),
            seed_sum: expected_seed_sum,
        }
    });

    assert!(summary.body_index < thread_count);
    assert_eq!(summary.body_threads, thread_count);
    assert!(summary.body_pending_status_available);
    assert_eq!(summary.scheduled_parent_jobs, thread_count);
    assert_eq!(summary.scheduled_child_jobs, thread_count);
    assert_eq!(summary.seed_sum, expected_seed_sum);

    assert_eq!(parent_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(child_started.load(Ordering::SeqCst), thread_count);

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
            record.joined_value,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "parent scoped work should observe worker-local pending-task status"
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
            .expect("child record should correspond to a parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(record.inherited_value, parent.joined_value);
        assert_eq!(
            record.child_value,
            parent.joined_value * 2 + record.executing_index
        );
    }

    let child_by_origin: BTreeMap<usize, ChildRecord> = child_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(child_by_origin.len(), thread_count);

    let mut confirmations = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let parent = parent_by_origin
            .get(&index)
            .expect("confirmation broadcast should find parent output");
        let child = child_by_origin
            .get(&index)
            .expect("confirmation broadcast should find child output");

        let parent_value = parent.joined_value;
        let child_value = child.child_value;

        let (left, right) =
            rayon_core::ThreadPool::join(pool_ref, move || parent_value + child_value, move || {
                index + num_threads
            });

        ConfirmationRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            total: left + right,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    confirmations.sort_by_key(|record| record.index);

    assert_eq!(confirmations.len(), thread_count);
    assert_eq!(
        confirmations
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &confirmations {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert!(record.pending_status_available);
        assert_eq!(
            record.total,
            parent_by_origin[&record.index].joined_value
                + child_by_origin[&record.index].child_value
                + record.index
                + thread_count
        );
    }

    let (observed_parent_sum, recomputed_parent_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || parent_records.iter().map(|record| record.joined_value).sum::<usize>(),
        || {
            parent_records
                .iter()
                .map(|record| {
                    seed_by_index[record.origin_index]
                        + record.origin_index
                        + thread_count
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_parent_sum, recomputed_parent_sum);

    let (observed_child_sum, recomputed_child_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || child_records.iter().map(|record| record.child_value).sum::<usize>(),
        || {
            child_records
                .iter()
                .map(|record| {
                    parent_by_origin[&record.origin_index].joined_value * 2
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_child_sum, recomputed_child_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_scope_propagates_spawned_panic_and_later_scope_reuses_pool() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("thread-pool-scope-panic-worker-{index}"))
        .build()
        .expect("custom Rayon thread pool should build");
    let pool_ref = &pool;

    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<(usize, usize)>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::scope(pool_ref, |scope| {
            rayon_core::Scope::spawn(scope, |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking scoped work should run inside the custom pool");
                assert!(worker_index < thread_count);
                assert!(
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some()
                );

                panic!("intentional ThreadPool::scope spawned panic for integration coverage");
            });

            for input in 0usize..(thread_count * 2) {
                let completed_ref = &completed_before_panic;
                let sibling_started_ref = &sibling_started;

                rayon_core::Scope::spawn(scope, move |_| {
                    sibling_started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                        .expect("non-panicking sibling scoped work should run inside the pool");
                    assert!(worker_index < thread_count);

                    completed_ref
                        .lock()
                        .expect("completed sibling mutex should not be poisoned")
                        .push((input, worker_index));
                });
            }

            123usize
        });
    }));

    let payload = panic_result
        .expect_err("panic in ThreadPool::scope spawned work should propagate to caller");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional ThreadPool::scope spawned panic"),
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
            .all(|(input, worker_index)| *input < thread_count * 2 && *worker_index < thread_count)
    );
    assert_eq!(
        completed_before_panic
            .iter()
            .map(|(input, _)| *input)
            .collect::<BTreeSet<_>>()
            .len(),
        completed_before_panic.len(),
        "each completed sibling task should report at most once"
    );

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

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 5) * (num_threads + 19),
        }
    });

    recovery_seeds.sort_by_key(|record| record.index);

    assert_eq!(recovery_seeds.len(), thread_count);
    assert_eq!(
        recovery_seeds
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &recovery_seeds {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, (record.index + 5) * (thread_count + 19));
    }

    let seed_by_origin: BTreeMap<usize, usize> = recovery_seeds
        .iter()
        .map(|record| (record.index, record.seed))
        .collect();
    assert_eq!(seed_by_origin.len(), thread_count);

    let expected_seed_sum: usize = recovery_seeds.iter().map(|record| record.seed).sum();
    let completed_count = completed_before_panic.len();

    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        let body_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
            .expect("recovery ThreadPool::scope body should run inside the custom pool");
        assert!(body_index < thread_count);

        for seed_record in recovery_seeds.iter().cloned() {
            let recovery_records_ref = &recovery_records;
            let recovery_started_ref = &recovery_started;

            rayon_core::Scope::spawn(scope, move |_| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery scoped work should run inside the custom pool");
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
                    move || executing_index + thread_count + completed_count,
                );

                recovery_records_ref
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                        value: left + right,
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    });
            });
        }

        expected_seed_sum + completed_count
    });

    assert_eq!(recovery_return, expected_seed_sum + completed_count);
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
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(seed_by_origin.get(&record.origin_index), Some(&record.seed));
        assert_eq!(
            record.value,
            record.seed
                + record.origin_index
                + record.executing_index
                + thread_count
                + completed_count
        );
        assert!(
            record.pending_status_available,
            "recovery scoped work should observe worker-local pending-task status"
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
                        + record.executing_index
                        + thread_count
                        + completed_count
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}