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
    worker_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ModuleScopeSummary {
    body_index: usize,
    body_name: String,
    body_threads: usize,
    body_pending_status_available: bool,
    scheduled_parent_jobs: usize,
    scheduled_broadcast_jobs: usize,
    seed_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BranchReport {
    worker_index: usize,
    num_threads: usize,
    pending_status_available: bool,
    seed_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    inherited_value: usize,
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
struct PanicSiblingRecord {
    input: usize,
    worker_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
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
fn module_path_scope_scope_uses_current_custom_pool_and_waits_for_nested_broadcast_pipeline() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("module-path-scope-current-pool-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

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
        let worker_name = std::thread::current().name().map(str::to_owned);
        let expected_name = format!("module-path-scope-current-pool-worker-{index}");

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(worker_name.as_deref(), Some(expected_name.as_str()));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 67),
            worker_name,
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 67));

        let expected_name = format!("module-path-scope-current-pool-worker-{expected_index}");
        assert_eq!(record.worker_name.as_deref(), Some(expected_name.as_str()));
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

    let (summary, branch_report) = rayon_core::ThreadPool::join(
        pool_ref,
        || {
            rayon_core::scope(|scope| {
                let body_index = rayon_core::current_thread_index()
                    .expect("rayon_core::scope::scope body should run on the current pool");
                assert!(body_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_thread_index(pool_ref),
                    Some(body_index)
                );

                let body_name = std::thread::current()
                    .name()
                    .map(str::to_owned)
                    .expect("custom pool worker should have a configured name");
                assert_eq!(
                    body_name,
                    format!("module-path-scope-current-pool-worker-{body_index}")
                );

                let body_pending_status_available =
                    rayon_core::current_thread_has_pending_tasks().is_some();

                for seed_record in seeds.iter().cloned() {
                    let parent_records_ref = &parent_records;
                    let child_records_ref = &child_records;
                    let parent_started_ref = &parent_started;
                    let child_started_ref = &child_started;

                    rayon_core::Scope::spawn(scope, move |nested_scope| {
                        parent_started_ref.fetch_add(1, Ordering::SeqCst);

                        let executing_index = rayon_core::current_thread_index()
                            .expect("parent scoped work should run on a Rayon worker");
                        assert!(executing_index < seed_record.num_threads);
                        assert_eq!(rayon_core::current_num_threads(), seed_record.num_threads);

                        let origin_index = seed_record.index;
                        let seed = seed_record.seed;
                        let num_threads = seed_record.num_threads;

                        let (left, right) = rayon_core::join(
                            move || seed + origin_index,
                            move || num_threads + executing_index,
                        );
                        let parent_value = left + right;

                        parent_records_ref
                            .lock()
                            .expect("parent record mutex should not be poisoned")
                            .push(ParentRecord {
                                origin_index,
                                seed,
                                executing_index,
                                num_threads,
                                value: parent_value,
                                pending_status_available:
                                    rayon_core::current_thread_has_pending_tasks().is_some(),
                            });

                        rayon_core::Scope::spawn(nested_scope, move |_| {
                            child_started_ref.fetch_add(1, Ordering::SeqCst);

                            let child_executing_index = rayon_core::current_thread_index()
                                .expect("nested child scoped work should run on a Rayon worker");
                            assert!(child_executing_index < num_threads);

                            let (doubled_parent, worker_component) = rayon_core::join(
                                move || parent_value * 2,
                                move || child_executing_index,
                            );

                            child_records_ref
                                .lock()
                                .expect("child record mutex should not be poisoned")
                                .push(ChildRecord {
                                    origin_index,
                                    parent_executing_index: executing_index,
                                    executing_index: child_executing_index,
                                    inherited_value: parent_value,
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

                rayon_core::Scope::spawn_broadcast(scope, move |scope, context| {
                    broadcast_started_ref.fetch_add(1, Ordering::SeqCst);

                    let index = rayon_core::BroadcastContext::index(&context);
                    let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                    assert_eq!(num_threads, thread_count);
                    assert!(index < num_threads);
                    assert_eq!(rayon_core::current_thread_index(), Some(index));
                    assert_eq!(rayon_core::current_num_threads(), thread_count);

                    let expected_name =
                        format!("module-path-scope-current-pool-worker-{index}");
                    assert_eq!(
                        std::thread::current().name(),
                        Some(expected_name.as_str())
                    );

                    let seed = seed_by_index_ref[index];
                    let (seed_component, index_component) =
                        rayon_core::join(move || seed * 3, move || index + num_threads * 5);
                    let broadcast_value = seed_component + index_component;

                    broadcast_records_ref
                        .lock()
                        .expect("broadcast record mutex should not be poisoned")
                        .push(BroadcastRecord {
                            index,
                            num_threads,
                            current_index: rayon_core::current_thread_index(),
                            seed,
                            value: broadcast_value,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });

                    rayon_core::Scope::spawn(scope, move |_| {
                        broadcast_child_started_ref.fetch_add(1, Ordering::SeqCst);

                        let executing_index = rayon_core::current_thread_index()
                            .expect("broadcast child work should run on a Rayon worker");
                        assert!(executing_index < num_threads);

                        broadcast_child_records_ref
                            .lock()
                            .expect("broadcast child record mutex should not be poisoned")
                            .push(BroadcastChildRecord {
                                origin_index: index,
                                executing_index,
                                value: broadcast_value + num_threads + executing_index,
                            });
                    });
                });

                ModuleScopeSummary {
                    body_index,
                    body_name,
                    body_threads: rayon_core::current_num_threads(),
                    body_pending_status_available,
                    scheduled_parent_jobs: seeds.len(),
                    scheduled_broadcast_jobs: thread_count,
                    seed_sum: expected_seed_sum,
                }
            })
        },
        || {
            let worker_index = rayon_core::current_thread_index()
                .expect("sibling ThreadPool::join branch should run in the custom pool");
            assert!(worker_index < thread_count);

            BranchReport {
                worker_index,
                num_threads: rayon_core::current_num_threads(),
                pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
                seed_sum: seed_by_index.iter().sum(),
            }
        },
    );

    assert!(summary.body_index < thread_count);
    assert_eq!(
        summary.body_name,
        format!(
            "module-path-scope-current-pool-worker-{}",
            summary.body_index
        )
    );
    assert_eq!(summary.body_threads, thread_count);
    assert!(summary.body_pending_status_available);
    assert_eq!(summary.scheduled_parent_jobs, thread_count);
    assert_eq!(summary.scheduled_broadcast_jobs, thread_count);
    assert_eq!(summary.seed_sum, expected_seed_sum);

    assert!(branch_report.worker_index < thread_count);
    assert_eq!(branch_report.num_threads, thread_count);
    assert!(branch_report.pending_status_available);
    assert_eq!(branch_report.seed_sum, expected_seed_sum);

    assert_eq!(parent_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(child_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(broadcast_started.load(Ordering::SeqCst), thread_count);
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
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(record.pending_status_available);
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
        assert_eq!(record.inherited_value, parent.value);
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
            seed_by_index[record.index] * 3 + record.index + thread_count * 5
        );
        assert!(record.pending_status_available);
    }

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
            seed_by_index[record.origin_index] * 3
                + record.origin_index
                + thread_count * 5
                + thread_count
                + record.executing_index
        );
    }

    let expected_parent_sum: usize = parent_records.iter().map(|record| record.value).sum();
    let expected_broadcast_child_sum: usize =
        broadcast_child_records.iter().map(|record| record.value).sum();

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
fn module_path_scope_scope_propagates_spawned_panic_and_global_pool_recovers() {
    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());
    assert_eq!(rayon_core::current_thread_index(), None);

    let task_count = (global_threads + 5).min(16);
    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<PanicSiblingRecord>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::scope(|scope| {
            rayon_core::Scope::spawn(scope, |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::current_thread_index()
                    .expect("panicking scoped work should run on a Rayon worker");
                assert!(worker_index < global_threads);
                assert!(
                    rayon_core::current_thread_has_pending_tasks().is_some(),
                    "panicking scoped work should observe worker-local pending-task status"
                );

                panic!(
                    "intentional rayon_core::scope::scope panic from worker {worker_index}"
                );
            });

            for input in 0usize..task_count {
                let completed_before_panic_ref = &completed_before_panic;
                let sibling_started_ref = &sibling_started;

                rayon_core::Scope::spawn(scope, move |_| {
                    sibling_started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::current_thread_index()
                        .expect("sibling scoped work should run on a Rayon worker");
                    assert!(worker_index < global_threads);
                    assert_eq!(rayon_core::current_num_threads(), global_threads);

                    let (square, cube) =
                        rayon_core::join(move || input * input, move || input * input * input);

                    completed_before_panic_ref
                        .lock()
                        .expect("completed sibling mutex should not be poisoned")
                        .push(PanicSiblingRecord {
                            input,
                            worker_index,
                            num_threads: rayon_core::current_num_threads(),
                            value: square + cube,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });
                });
            }

            321usize
        });
    }));

    let payload =
        panic_result.expect_err("panic in rayon_core::scope::scope work should propagate");
    let message = panic_payload_to_string(&*payload);

    assert!(
        message.contains("intentional rayon_core::scope::scope panic"),
        "unexpected propagated panic payload: {message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);

    let sibling_count_before_recovery = sibling_started.load(Ordering::SeqCst);
    assert!(sibling_count_before_recovery <= task_count);

    let mut completed_before_panic = completed_before_panic
        .into_inner()
        .expect("completed sibling mutex should not be poisoned");
    completed_before_panic.sort_by_key(|record| record.input);

    assert_eq!(completed_before_panic.len(), sibling_count_before_recovery);
    assert_eq!(
        completed_before_panic
            .iter()
            .map(|record| record.input)
            .collect::<BTreeSet<_>>()
            .len(),
        completed_before_panic.len(),
        "each completed sibling should report at most once"
    );

    for record in &completed_before_panic {
        assert!(record.input < task_count);
        assert!(record.worker_index < global_threads);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(
            record.value,
            record.input * record.input + record.input * record.input * record.input
        );
        assert!(record.pending_status_available);
    }

    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "after unwinding, the external caller should still not be a Rayon worker"
    );

    let mut recovery_seeds = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), global_threads);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 3) * (num_threads + 41),
            worker_name: std::thread::current().name().map(str::to_owned),
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
        assert_eq!(record.seed, (record.index + 3) * (global_threads + 41));
    }

    let seed_by_origin: BTreeMap<usize, usize> = recovery_seeds
        .iter()
        .map(|record| (record.index, record.seed))
        .collect();
    assert_eq!(seed_by_origin.len(), global_threads);

    let expected_seed_sum: usize = recovery_seeds.iter().map(|record| record.seed).sum();
    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::scope(|scope| {
        for seed_record in recovery_seeds.iter().cloned() {
            let recovery_records_ref = &recovery_records;
            let recovery_started_ref = &recovery_started;

            rayon_core::Scope::spawn(scope, move |nested_scope| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let parent_executing_index = rayon_core::current_thread_index()
                    .expect("recovery parent work should run on a Rayon worker");
                assert!(parent_executing_index < global_threads);
                assert_eq!(rayon_core::current_num_threads(), global_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || parent_executing_index + global_threads,
                );
                let parent_value = left + right;

                recovery_records_ref
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        stage: 0,
                        origin_index,
                        seed,
                        executing_index: parent_executing_index,
                        value: parent_value,
                    });

                rayon_core::Scope::spawn(nested_scope, move |_| {
                    recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index = rayon_core::current_thread_index()
                        .expect("recovery child work should run on a Rayon worker");
                    assert!(child_executing_index < global_threads);

                    let (from_parent, from_child_worker) =
                        rayon_core::join(move || parent_value, move || child_executing_index);

                    recovery_records_ref
                        .lock()
                        .expect("recovery record mutex should not be poisoned")
                        .push(RecoveryRecord {
                            stage: 1,
                            origin_index,
                            seed,
                            executing_index: child_executing_index,
                            value: from_parent + from_child_worker,
                        });
                });
            });
        }

        expected_seed_sum + completed_before_panic.len()
    });

    assert_eq!(
        recovery_return,
        expected_seed_sum + completed_before_panic.len()
    );
    assert_eq!(recovery_started.load(Ordering::SeqCst), global_threads * 2);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| (record.stage, record.origin_index));

    assert_eq!(recovery_records.len(), global_threads * 2);

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

    assert_eq!(parent_records.len(), global_threads);
    assert_eq!(child_records.len(), global_threads);

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );
    assert_eq!(
        child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for record in &parent_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );
        assert_eq!(
            record.value,
            record.seed + record.origin_index + record.executing_index + global_threads
        );
    }

    let parent_by_origin: BTreeMap<usize, RecoveryRecord> = parent_records
        .into_iter()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(parent_by_origin.len(), global_threads);

    for record in &child_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("child recovery record should correspond to a parent recovery record");

        assert_eq!(record.seed, parent.seed);
        assert_eq!(record.value, parent.value + record.executing_index);
    }

    let (observed_child_sum, recomputed_child_sum) = rayon_core::join(
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