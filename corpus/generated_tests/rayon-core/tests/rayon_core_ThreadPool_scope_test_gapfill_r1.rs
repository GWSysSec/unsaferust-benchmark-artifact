use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopeSeed {
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
    expected_child_jobs: usize,
    seed_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    joined_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    inherited_value: usize,
    child_value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
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
fn thread_pool_scope_consumes_broadcast_output_and_waits_for_nested_scoped_pipeline() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("thread-pool-scope-pipeline-worker-{index}"))
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

        ScopeSeed {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 11),
        }
    });

    seeds.sort_by_key(|seed| seed.index);

    assert_eq!(seeds.len(), thread_count);
    assert_eq!(
        seeds.iter().map(|seed| seed.index).collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (expected_index, seed) in seeds.iter().enumerate() {
        assert_eq!(seed.index, expected_index);
        assert_eq!(seed.num_threads, thread_count);
        assert_eq!(seed.current_index, Some(expected_index));
        assert_eq!(seed.seed, (expected_index + 1) * (thread_count + 11));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|seed| seed.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let parent_records = Mutex::new(Vec::<ScopedParentRecord>::new());
    let child_records = Mutex::new(Vec::<ScopedChildRecord>::new());
    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);

    let summary = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        let body_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
            .expect("ThreadPool::scope body should execute inside the custom pool");
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
            let parent_records = &parent_records;
            let child_records = &child_records;
            let parent_started = &parent_started;
            let child_started = &child_started;

            rayon_core::Scope::spawn(scope, move |nested_scope| {
                parent_started.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("scoped parent work should run inside the custom pool");
                assert!(executing_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                let pending_status_available =
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();

                let origin_index = seed_record.index;
                let seed = seed_record.seed;

                let (seed_component, worker_component) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || {
                        let inner_index =
                            rayon_core::ThreadPool::current_thread_index(pool_ref)
                                .expect("nested left join branch should run in the custom pool");
                        assert!(inner_index < thread_count);
                        seed + origin_index
                    },
                    move || {
                        let inner_index =
                            rayon_core::ThreadPool::current_thread_index(pool_ref)
                                .expect("nested right join branch should run in the custom pool");
                        assert!(inner_index < thread_count);
                        thread_count + executing_index
                    },
                );

                let joined_value = seed_component + worker_component;

                parent_records
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(ScopedParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                        joined_value,
                        pending_status_available,
                    });

                rayon_core::Scope::spawn(nested_scope, move |_| {
                    child_started.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index =
                        rayon_core::ThreadPool::current_thread_index(pool_ref)
                            .expect("scoped child work should run inside the custom pool");
                    assert!(child_executing_index < thread_count);

                    let (doubled_parent, worker_component) = rayon_core::join(
                        move || joined_value * 2,
                        move || child_executing_index,
                    );

                    child_records
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(ScopedChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            inherited_value: joined_value,
                            child_value: doubled_parent + worker_component,
                        });
                });
            });
        }

        ScopeSummary {
            body_index,
            body_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
            body_pending_status_available,
            scheduled_parent_jobs: seeds.len(),
            expected_child_jobs: seeds.len(),
            seed_sum: expected_seed_sum,
        }
    });

    assert!(summary.body_index < thread_count);
    assert_eq!(summary.body_threads, thread_count);
    assert!(summary.body_pending_status_available);
    assert_eq!(summary.scheduled_parent_jobs, thread_count);
    assert_eq!(summary.expected_child_jobs, thread_count);
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
            "scoped parent work should observe worker-local pending-task status"
        );
    }

    let parent_by_origin: BTreeMap<usize, ScopedParentRecord> = parent_records
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
            .expect("child record should correspond to a scoped parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(record.inherited_value, parent.joined_value);
        assert_eq!(
            record.child_value,
            parent.joined_value * 2 + record.executing_index
        );
    }

    let expected_parent_sum: usize = parent_records
        .iter()
        .map(|record| record.joined_value)
        .sum();
    let expected_child_sum: usize = child_records.iter().map(|record| record.child_value).sum();

    let (observed_parent_sum, observed_child_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || parent_records.iter().map(|record| record.joined_value).sum::<usize>(),
        || child_records.iter().map(|record| record.child_value).sum::<usize>(),
    );

    assert_eq!(observed_parent_sum, expected_parent_sum);
    assert_eq!(observed_child_sum, expected_child_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_scope_propagates_worker_panic_and_later_scope_reuses_pool() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("thread-pool-scope-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon thread pool should build");

    let pool_ref = &pool;

    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<(usize, usize)>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::scope(pool_ref, |scope| {
            let panic_started = &panic_started;

            rayon_core::Scope::spawn(scope, move |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking scoped work should run inside the custom pool");
                assert!(worker_index < thread_count);
                assert!(
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some()
                );

                panic!("intentional ThreadPool::scope panic for recovery");
            });

            for value in 0..(thread_count * 2) {
                let completed_before_panic = &completed_before_panic;
                let sibling_started = &sibling_started;

                rayon_core::Scope::spawn(scope, move |_| {
                    sibling_started.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                        .expect("non-panicking sibling work should run inside the custom pool");
                    assert!(worker_index < thread_count);

                    completed_before_panic
                        .lock()
                        .expect("completed sibling mutex should not be poisoned")
                        .push((value, worker_index));
                });
            }

            31337usize
        });
    }));

    let payload = panic_result
        .expect_err("a panic in scoped work should propagate out of ThreadPool::scope");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional ThreadPool::scope panic for recovery"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);

    let sibling_count = sibling_started.load(Ordering::SeqCst);
    assert!(sibling_count <= thread_count * 2);

    let completed_before_panic = completed_before_panic
        .into_inner()
        .expect("completed sibling mutex should not be poisoned");

    assert_eq!(completed_before_panic.len(), sibling_count);
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

        (index, (index + 2) * (num_threads + 17))
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

    let recovery_return = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        let body_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
            .expect("recovery ThreadPool::scope body should run inside the custom pool");
        assert!(body_index < thread_count);

        for (origin_index, seed) in recovery_seeds.iter().copied() {
            let recovery_records = &recovery_records;
            let recovery_started = &recovery_started;

            rayon_core::Scope::spawn(scope, move |_| {
                recovery_started.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery scoped work should run inside the custom pool");
                assert!(executing_index < thread_count);

                let (left, right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || thread_count + executing_index,
                );

                recovery_records
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
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
        assert_eq!(record.num_threads, thread_count);
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