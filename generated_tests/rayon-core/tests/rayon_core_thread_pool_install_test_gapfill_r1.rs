use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct InstallSeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: String,
    seed: usize,
    joined_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InstallFollowupRecord {
    origin_index: usize,
    seed: usize,
    broadcast_joined_value: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InstallPipelineSummary {
    installer_index: usize,
    installer_name: String,
    pending_status_available: bool,
    seed_count: usize,
    seed_sum: usize,
    joined_sum: usize,
    scope_return: usize,
    scoped_sum: usize,
    recomputed_scoped_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SwitchContextRecord {
    label: &'static str,
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: String,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InnerInstallSummary {
    installer_index: usize,
    outer_index_seen_inside_inner: Option<usize>,
    contexts: Vec<SwitchContextRecord>,
    reduction: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SwitchInstallSummary {
    outer_before: usize,
    outer_after: usize,
    inner_index_before: Option<usize>,
    outer_contexts: Vec<SwitchContextRecord>,
    inner: InnerInstallSummary,
    outer_reduction: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PrePanicRecord {
    input: usize,
    worker_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PanicRecoveryRecord {
    input: usize,
    original_worker_index: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PanicRecoverySummary {
    installer_index: usize,
    broadcast_seed_sum: usize,
    scope_return: usize,
    recovery_sum: usize,
    recomputed_sum: usize,
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
fn thread_pool_install_runs_on_custom_pool_and_drives_broadcast_scope_fifo_join_pipeline() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("install-pipeline-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let pool_ref = &pool;
    let followup_records = Mutex::new(Vec::<InstallFollowupRecord>::new());

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(pool_ref),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "the integration-test thread should start outside the custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None
    );

    let summary = rayon_core::ThreadPool::install(pool_ref, || {
        let installer_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
            .expect("ThreadPool::install should execute the closure on a pool worker");
        assert!(installer_index < thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(installer_index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(
            rayon_core::ThreadPool::current_num_threads(pool_ref),
            thread_count
        );

        let installer_name = std::thread::current()
            .name()
            .map(str::to_owned)
            .expect("installed closure should run on a named worker");
        assert_eq!(
            installer_name,
            format!("install-pipeline-worker-{installer_index}")
        );

        let pending_status_available =
            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();
        assert!(
            pending_status_available,
            "installed worker should be able to query pending-task status"
        );

        let mut seeds = rayon_core::broadcast(|context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(
                num_threads, thread_count,
                "free broadcast inside install should use the current custom pool"
            );
            assert!(index < num_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), thread_count);
            assert_eq!(
                rayon_core::ThreadPool::current_thread_index(pool_ref),
                Some(index)
            );

            let worker_name = std::thread::current()
                .name()
                .map(str::to_owned)
                .expect("broadcast should run on a named worker");
            assert_eq!(worker_name, format!("install-pipeline-worker-{index}"));

            let seed = (index + 1) * (num_threads + 101);
            let (left, right) =
                rayon_core::join(move || seed + index, move || num_threads * 10);

            InstallSeedRecord {
                index,
                num_threads,
                current_index: rayon_core::current_thread_index(),
                worker_name,
                seed,
                joined_value: left + right,
                pending_status_available: rayon_core::current_thread_has_pending_tasks()
                    .is_some(),
            }
        });

        seeds.sort_by_key(|record| record.index);

        assert_eq!(seeds.len(), thread_count);
        assert_eq!(
            seeds.iter().map(|record| record.index).collect::<BTreeSet<_>>(),
            expected_indices
        );

        for (expected_index, record) in seeds.iter().enumerate() {
            assert_eq!(record.index, expected_index);
            assert_eq!(record.num_threads, thread_count);
            assert_eq!(record.current_index, Some(expected_index));
            assert_eq!(
                record.worker_name,
                format!("install-pipeline-worker-{expected_index}")
            );
            assert_eq!(record.seed, (expected_index + 1) * (thread_count + 101));
            assert_eq!(
                record.joined_value,
                record.seed + record.index + thread_count * 10
            );
            assert!(
                record.pending_status_available,
                "broadcast workers should observe pending-task status"
            );
        }

        let seed_sum: usize = seeds.iter().map(|record| record.seed).sum();
        let joined_sum: usize = seeds.iter().map(|record| record.joined_value).sum();

        let scope_return = rayon_core::scope_fifo(|scope| {
            for seed_record in seeds.iter().cloned() {
                let followup_records_ref = &followup_records;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    let executing_index = rayon_core::current_thread_index()
                        .expect("FIFO follow-up should run on a pool worker");
                    assert!(executing_index < thread_count);
                    assert_eq!(rayon_core::current_num_threads(), thread_count);

                    let origin_index = seed_record.index;
                    let seed = seed_record.seed;
                    let broadcast_joined_value = seed_record.joined_value;

                    let (left, right) = rayon_core::join(
                        move || broadcast_joined_value + seed,
                        move || origin_index + executing_index + thread_count,
                    );

                    followup_records_ref
                        .lock()
                        .expect("follow-up record mutex should not be poisoned")
                        .push(InstallFollowupRecord {
                            origin_index,
                            seed,
                            broadcast_joined_value,
                            executing_index,
                            value: left + right,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });
                });
            }

            seed_sum + joined_sum
        });

        assert_eq!(scope_return, seed_sum + joined_sum);
        assert_eq!(
            rayon_core::ThreadPool::current_thread_index(pool_ref),
            Some(installer_index),
            "after nested work, ThreadPool::install should still be executing on the pool"
        );

        let followup_snapshot = followup_records
            .lock()
            .expect("follow-up record mutex should not be poisoned")
            .clone();

        let (scoped_sum, recomputed_scoped_sum) = rayon_core::join(
            || followup_snapshot.iter().map(|record| record.value).sum::<usize>(),
            || {
                followup_snapshot
                    .iter()
                    .map(|record| {
                        record.broadcast_joined_value
                            + record.seed
                            + record.origin_index
                            + record.executing_index
                            + thread_count
                    })
                    .sum::<usize>()
            },
        );

        InstallPipelineSummary {
            installer_index,
            installer_name,
            pending_status_available,
            seed_count: seeds.len(),
            seed_sum,
            joined_sum,
            scope_return,
            scoped_sum,
            recomputed_scoped_sum,
        }
    });

    assert!(summary.installer_index < thread_count);
    assert_eq!(
        summary.installer_name,
        format!("install-pipeline-worker-{}", summary.installer_index)
    );
    assert!(summary.pending_status_available);
    assert_eq!(summary.seed_count, thread_count);
    assert_eq!(
        summary.seed_sum,
        (0..thread_count)
            .map(|index| (index + 1) * (thread_count + 101))
            .sum::<usize>()
    );
    assert_eq!(
        summary.joined_sum,
        (0..thread_count)
            .map(|index| {
                let seed = (index + 1) * (thread_count + 101);
                seed + index + thread_count * 10
            })
            .sum::<usize>()
    );
    assert_eq!(summary.scope_return, summary.seed_sum + summary.joined_sum);
    assert_eq!(summary.scoped_sum, summary.recomputed_scoped_sum);

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after install returns, the external caller should not be a pool worker"
    );

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
        assert_eq!(
            record.seed,
            (record.origin_index + 1) * (thread_count + 101)
        );
        assert_eq!(
            record.broadcast_joined_value,
            record.seed + record.origin_index + thread_count * 10
        );
        assert_eq!(
            record.value,
            record.broadcast_joined_value
                + record.seed
                + record.origin_index
                + record.executing_index
                + thread_count
        );
        assert!(
            record.pending_status_available,
            "FIFO follow-up work should observe worker-local pending-task status"
        );
    }

    assert_eq!(
        followup_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>(),
        summary.scoped_sum
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_install_can_switch_between_custom_pools_and_restore_the_outer_context() {
    let outer_threads = 2usize;
    let inner_threads = 3usize;

    let outer_pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(outer_threads)
        .thread_name(|index| format!("install-switch-outer-worker-{index}"))
        .build()
        .expect("outer custom pool should build");

    let inner_pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(inner_threads)
        .thread_name(|index| format!("install-switch-inner-worker-{index}"))
        .build()
        .expect("inner custom pool should build");

    let outer_ref = &outer_pool;
    let inner_ref = &inner_pool;

    assert_eq!(rayon_core::ThreadPool::current_thread_index(outer_ref), None);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(inner_ref), None);

    let summary = rayon_core::ThreadPool::install(outer_ref, || {
        let outer_before = rayon_core::ThreadPool::current_thread_index(outer_ref)
            .expect("outer install should run on an outer worker");
        assert!(outer_before < outer_threads);
        assert_eq!(rayon_core::current_num_threads(), outer_threads);
        assert_eq!(
            rayon_core::ThreadPool::current_thread_index(inner_ref),
            None,
            "an outer worker should not be considered a worker of the inner pool"
        );

        let inner_index_before = rayon_core::ThreadPool::current_thread_index(inner_ref);

        let inner = rayon_core::ThreadPool::install(inner_ref, || {
            let installer_index = rayon_core::ThreadPool::current_thread_index(inner_ref)
                .expect("inner install should run on an inner worker");
            assert!(installer_index < inner_threads);
            assert_eq!(rayon_core::current_num_threads(), inner_threads);
            assert_eq!(
                rayon_core::ThreadPool::current_thread_index(outer_ref),
                None,
                "while inside the inner install, the current worker belongs to the inner pool"
            );

            let mut contexts = rayon_core::broadcast(|context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);
                let worker_name = std::thread::current()
                    .name()
                    .map(str::to_owned)
                    .expect("inner broadcast should run on a named worker");

                assert_eq!(num_threads, inner_threads);
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(
                    worker_name,
                    format!("install-switch-inner-worker-{index}")
                );

                let (left, right) =
                    rayon_core::join(move || index + 1, move || num_threads * 100);

                SwitchContextRecord {
                    label: "inner-broadcast",
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    worker_name,
                    value: left + right,
                }
            });

            contexts.sort_by_key(|record| record.index);

            let (value_sum, index_sum) = rayon_core::join(
                || contexts.iter().map(|record| record.value).sum::<usize>(),
                || contexts.iter().map(|record| record.index).sum::<usize>(),
            );

            InnerInstallSummary {
                installer_index,
                outer_index_seen_inside_inner: rayon_core::ThreadPool::current_thread_index(
                    outer_ref,
                ),
                contexts,
                reduction: value_sum + index_sum,
            }
        });

        assert_eq!(
            rayon_core::current_num_threads(),
            outer_threads,
            "returning from inner install should restore the outer pool as current"
        );
        let outer_after = rayon_core::ThreadPool::current_thread_index(outer_ref)
            .expect("outer install should resume on an outer worker");
        assert_eq!(outer_after, outer_before);

        let mut outer_contexts = rayon_core::broadcast(|context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);
            let worker_name = std::thread::current()
                .name()
                .map(str::to_owned)
                .expect("outer broadcast should run on a named worker");

            assert_eq!(num_threads, outer_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(
                worker_name,
                format!("install-switch-outer-worker-{index}")
            );

            let (left, right) =
                rayon_core::join(move || index * 10, move || num_threads * 1000);

            SwitchContextRecord {
                label: "outer-broadcast",
                index,
                num_threads,
                current_index: rayon_core::current_thread_index(),
                worker_name,
                value: left + right,
            }
        });

        outer_contexts.sort_by_key(|record| record.index);

        let (outer_value_sum, outer_index_sum) = rayon_core::join(
            || outer_contexts.iter().map(|record| record.value).sum::<usize>(),
            || outer_contexts.iter().map(|record| record.index).sum::<usize>(),
        );

        SwitchInstallSummary {
            outer_before,
            outer_after,
            inner_index_before,
            outer_contexts,
            inner,
            outer_reduction: outer_value_sum + outer_index_sum,
        }
    });

    assert!(summary.outer_before < outer_threads);
    assert_eq!(summary.outer_after, summary.outer_before);
    assert_eq!(summary.inner_index_before, None);

    assert!(summary.inner.installer_index < inner_threads);
    assert_eq!(summary.inner.outer_index_seen_inside_inner, None);
    assert_eq!(summary.inner.contexts.len(), inner_threads);
    assert_eq!(
        summary
            .inner
            .contexts
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(inner_threads)
    );

    for record in &summary.inner.contexts {
        assert_eq!(record.label, "inner-broadcast");
        assert_eq!(record.num_threads, inner_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(
            record.worker_name,
            format!("install-switch-inner-worker-{}", record.index)
        );
        assert_eq!(record.value, record.index + 1 + inner_threads * 100);
    }

    assert_eq!(
        summary.inner.reduction,
        summary
            .inner
            .contexts
            .iter()
            .map(|record| record.value + record.index)
            .sum::<usize>()
    );

    assert_eq!(summary.outer_contexts.len(), outer_threads);
    assert_eq!(
        summary
            .outer_contexts
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(outer_threads)
    );

    for record in &summary.outer_contexts {
        assert_eq!(record.label, "outer-broadcast");
        assert_eq!(record.num_threads, outer_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(
            record.worker_name,
            format!("install-switch-outer-worker-{}", record.index)
        );
        assert_eq!(record.value, record.index * 10 + outer_threads * 1000);
    }

    assert_eq!(
        summary.outer_reduction,
        summary
            .outer_contexts
            .iter()
            .map(|record| record.value + record.index)
            .sum::<usize>()
    );

    assert_eq!(rayon_core::ThreadPool::current_thread_index(outer_ref), None);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(inner_ref), None);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_install_propagates_panic_after_scoped_work_and_pool_recovers_with_later_install() {
    let thread_count = 3usize;
    let task_count = thread_count * 3 + 2usize;
    let expected_inputs: BTreeSet<_> = (0usize..task_count).collect();

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("install-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let pool_ref = &pool;
    let pre_panic_records = Mutex::new(Vec::<PrePanicRecord>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: () = rayon_core::ThreadPool::install(pool_ref, || {
            let installer_index = rayon_core::current_thread_index()
                .expect("installed panic test should start on a pool worker");
            assert!(installer_index < thread_count);
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            rayon_core::scope(|scope| {
                for input in 0usize..task_count {
                    let pre_panic_records_ref = &pre_panic_records;

                    rayon_core::Scope::spawn(scope, move |_| {
                        let worker_index = rayon_core::current_thread_index()
                            .expect("pre-panic scoped work should run on a Rayon worker");
                        assert!(worker_index < thread_count);
                        assert_eq!(rayon_core::current_num_threads(), thread_count);

                        let (square, cube) =
                            rayon_core::join(move || input * input, move || input * input * input);

                        pre_panic_records_ref
                            .lock()
                            .expect("pre-panic record mutex should not be poisoned")
                            .push(PrePanicRecord {
                                input,
                                worker_index,
                                num_threads: rayon_core::current_num_threads(),
                                value: square + cube,
                                pending_status_available:
                                    rayon_core::current_thread_has_pending_tasks().is_some(),
                            });
                    });
                }
            });

            panic!("intentional ThreadPool::install panic after scoped work completed");
        });
    }));

    let payload = panic_result
        .expect_err("a panic in the installed closure should propagate to the caller");
    let panic_message = panic_payload_to_string(&*payload);
    assert!(
        panic_message.contains("ThreadPool::install panic after scoped work"),
        "unexpected propagated panic payload: {panic_message:?}"
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

    let mut completed = pre_panic_records
        .into_inner()
        .expect("pre-panic record mutex should not be poisoned");
    completed.sort_by_key(|record| record.input);

    assert_eq!(completed.len(), task_count);
    assert_eq!(
        completed
            .iter()
            .map(|record| record.input)
            .collect::<BTreeSet<_>>(),
        expected_inputs
    );

    for record in &completed {
        assert!(record.worker_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(
            record.value,
            record.input * record.input + record.input * record.input * record.input
        );
        assert!(
            record.pending_status_available,
            "pre-panic scoped work should observe pending-task status"
        );
    }

    let completed_by_input: BTreeMap<usize, PrePanicRecord> = completed
        .iter()
        .cloned()
        .map(|record| (record.input, record))
        .collect();
    assert_eq!(completed_by_input.len(), task_count);

    let expected_completed_sum: usize = completed.iter().map(|record| record.value).sum();
    let recovery_records = Mutex::new(Vec::<PanicRecoveryRecord>::new());

    let recovery_summary = rayon_core::ThreadPool::install(pool_ref, || {
        let installer_index = rayon_core::current_thread_index()
            .expect("recovery install should run on a Rayon worker");
        assert!(installer_index < thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let mut recovery_seeds = rayon_core::broadcast(|context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            (index, (index + 5) * (num_threads + 17))
        });

        recovery_seeds.sort_by_key(|entry| entry.0);

        assert_eq!(recovery_seeds.len(), thread_count);
        assert_eq!(
            recovery_seeds
                .iter()
                .map(|(index, _)| *index)
                .collect::<BTreeSet<_>>(),
            expected_worker_indices(thread_count)
        );

        let broadcast_seed_sum: usize = recovery_seeds.iter().map(|(_, seed)| *seed).sum();

        let scope_return = rayon_core::scope_fifo(|scope| {
            for record in completed.iter().cloned() {
                let recovery_records_ref = &recovery_records;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    let executing_index = rayon_core::current_thread_index()
                        .expect("recovery FIFO work should run on a Rayon worker");
                    assert!(executing_index < thread_count);

                    let input = record.input;
                    let original_worker_index = record.worker_index;

                    let (left, right) = rayon_core::join(
                        move || record.value + input,
                        move || original_worker_index + executing_index + thread_count,
                    );

                    recovery_records_ref
                        .lock()
                        .expect("recovery record mutex should not be poisoned")
                        .push(PanicRecoveryRecord {
                            input,
                            original_worker_index,
                            executing_index,
                            value: left + right,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });
                });
            }

            expected_completed_sum + broadcast_seed_sum
        });

        let recovery_snapshot = recovery_records
            .lock()
            .expect("recovery record mutex should not be poisoned")
            .clone();

        let (recovery_sum, recomputed_sum) = rayon_core::join(
            || recovery_snapshot.iter().map(|record| record.value).sum::<usize>(),
            || {
                recovery_snapshot
                    .iter()
                    .map(|record| {
                        let original = completed_by_input
                            .get(&record.input)
                            .expect("original pre-panic record should exist");
                        original.value
                            + original.input
                            + original.worker_index
                            + record.executing_index
                            + thread_count
                    })
                    .sum::<usize>()
            },
        );

        PanicRecoverySummary {
            installer_index,
            broadcast_seed_sum,
            scope_return,
            recovery_sum,
            recomputed_sum,
        }
    });

    assert!(recovery_summary.installer_index < thread_count);
    assert_eq!(
        recovery_summary.broadcast_seed_sum,
        (0usize..thread_count)
            .map(|index| (index + 5) * (thread_count + 17))
            .sum::<usize>()
    );
    assert_eq!(
        recovery_summary.scope_return,
        expected_completed_sum + recovery_summary.broadcast_seed_sum
    );
    assert_eq!(
        recovery_summary.recovery_sum,
        recovery_summary.recomputed_sum
    );

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| record.input);

    assert_eq!(recovery_records.len(), task_count);
    assert_eq!(
        recovery_records
            .iter()
            .map(|record| record.input)
            .collect::<BTreeSet<_>>(),
        expected_inputs
    );

    for record in &recovery_records {
        assert!(record.executing_index < thread_count);
        assert!(
            record.pending_status_available,
            "recovery FIFO work should observe pending-task status"
        );

        let original = completed_by_input
            .get(&record.input)
            .expect("recovery record should correspond to a pre-panic record");

        assert_eq!(record.original_worker_index, original.worker_index);
        assert_eq!(
            record.value,
            original.value
                + original.input
                + original.worker_index
                + record.executing_index
                + thread_count
        );
    }

    assert_eq!(
        recovery_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>(),
        recovery_summary.recovery_sum
    );

    assert_eq!(rayon_core::ThreadPool::current_thread_index(pool_ref), None);
}