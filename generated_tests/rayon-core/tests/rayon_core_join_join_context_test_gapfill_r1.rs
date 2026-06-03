use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleBranchRecord {
    label: &'static str,
    worker_index: Option<usize>,
    num_threads: usize,
    migrated: bool,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleFollowupRecord {
    slot: usize,
    worker_index: Option<usize>,
    num_threads: usize,
    left_migrated: bool,
    right_migrated: bool,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleSummary {
    body_index: Option<usize>,
    body_threads: usize,
    branch_total: usize,
    scheduled_followups: usize,
    pending_status_available: bool,
}

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
    scheduled_jobs: usize,
    seed_sum: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextPipelineRecord {
    origin_index: usize,
    seed: usize,
    caller_index: usize,
    num_threads: usize,
    left_worker: usize,
    right_worker: usize,
    left_migrated: bool,
    right_migrated: bool,
    left_value: usize,
    right_value: usize,
    combined: usize,
    caller_pending_status_available: bool,
    left_pending_status_available: bool,
    right_pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextFollowupRecord {
    origin_index: usize,
    executing_index: usize,
    left_worker: usize,
    right_worker: usize,
    left_migrated: bool,
    right_migrated: bool,
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
fn module_path_join_context_single_worker_reports_stable_non_migrated_context_and_feeds_fifo_work()
{
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("join-context-single-worker-{index}"))
        .build()
        .expect("single-worker pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let branch_records = Mutex::new(Vec::<SingleBranchRecord>::new());
    let followup_records = Mutex::new(Vec::<SingleFollowupRecord>::new());

    let summary = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        assert_eq!(rayon_core::current_thread_index(), Some(0));
        assert_eq!(rayon_core::current_num_threads(), 1);

        let ((left_value, left_record), (right_value, right_record)) =
            rayon_core::join_context(
                |context| {
                    let migrated = rayon_core::FnContext::migrated(&context);
                    assert!(
                        !migrated,
                        "left branch on a single-worker pool should not migrate"
                    );
                    assert_eq!(rayon_core::current_thread_index(), Some(0));

                    let (inner_left, inner_right) = rayon_core::join_context(
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            assert_eq!(rayon_core::current_thread_index(), Some(0));
                            11usize
                        },
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            assert_eq!(rayon_core::current_thread_index(), Some(0));
                            31usize
                        },
                    );

                    let value = inner_left + inner_right + rayon_core::current_num_threads();

                    (
                        value,
                        SingleBranchRecord {
                            label: "left",
                            worker_index: rayon_core::current_thread_index(),
                            num_threads: rayon_core::current_num_threads(),
                            migrated,
                            value,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        },
                    )
                },
                |context| {
                    let migrated = rayon_core::FnContext::migrated(&context);
                    assert!(
                        !migrated,
                        "right branch cannot be stolen in a single-worker pool"
                    );
                    assert_eq!(rayon_core::current_thread_index(), Some(0));

                    let (inner_left, inner_right) = rayon_core::join_context(
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            7usize
                        },
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            19usize
                        },
                    );

                    let value = inner_left * 2 + inner_right + rayon_core::current_num_threads();

                    (
                        value,
                        SingleBranchRecord {
                            label: "right",
                            worker_index: rayon_core::current_thread_index(),
                            num_threads: rayon_core::current_num_threads(),
                            migrated,
                            value,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        },
                    )
                },
            );

        branch_records
            .lock()
            .expect("branch record mutex should not be poisoned")
            .extend([left_record, right_record]);

        for (slot, base_value) in [(0usize, left_value), (1usize, right_value)] {
            let followup_records = &followup_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                assert_eq!(rayon_core::current_thread_index(), Some(0));
                assert_eq!(rayon_core::current_num_threads(), 1);

                let ((left_migrated, left), (right_migrated, right)) =
                    rayon_core::join_context(
                        |context| {
                            let migrated = rayon_core::FnContext::migrated(&context);
                            assert!(!migrated);
                            (migrated, base_value + slot)
                        },
                        |context| {
                            let migrated = rayon_core::FnContext::migrated(&context);
                            assert!(!migrated);
                            (migrated, base_value * 2 + slot)
                        },
                    );

                followup_records
                    .lock()
                    .expect("follow-up record mutex should not be poisoned")
                    .push(SingleFollowupRecord {
                        slot,
                        worker_index: rayon_core::current_thread_index(),
                        num_threads: rayon_core::current_num_threads(),
                        left_migrated,
                        right_migrated,
                        value: left + right,
                    });
            });
        }

        SingleSummary {
            body_index: rayon_core::current_thread_index(),
            body_threads: rayon_core::current_num_threads(),
            branch_total: left_value + right_value,
            scheduled_followups: 2,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    assert_eq!(summary.body_index, Some(0));
    assert_eq!(summary.body_threads, 1);
    assert_eq!(summary.branch_total, 43 + 34);
    assert_eq!(summary.scheduled_followups, 2);
    assert!(summary.pending_status_available);

    let mut branch_records = branch_records
        .into_inner()
        .expect("branch record mutex should not be poisoned");
    branch_records.sort_by_key(|record| record.label);

    assert_eq!(branch_records.len(), 2);
    assert_eq!(branch_records[0].label, "left");
    assert_eq!(branch_records[0].worker_index, Some(0));
    assert_eq!(branch_records[0].num_threads, 1);
    assert!(!branch_records[0].migrated);
    assert_eq!(branch_records[0].value, 43);
    assert!(branch_records[0].pending_status_available);

    assert_eq!(branch_records[1].label, "right");
    assert_eq!(branch_records[1].worker_index, Some(0));
    assert_eq!(branch_records[1].num_threads, 1);
    assert!(!branch_records[1].migrated);
    assert_eq!(branch_records[1].value, 34);
    assert!(branch_records[1].pending_status_available);

    let mut followup_records = followup_records
        .into_inner()
        .expect("follow-up record mutex should not be poisoned");
    followup_records.sort_by_key(|record| record.slot);

    assert_eq!(followup_records.len(), 2);

    for record in &followup_records {
        assert_eq!(record.worker_index, Some(0));
        assert_eq!(record.num_threads, 1);
        assert!(!record.left_migrated);
        assert!(!record.right_migrated);
    }

    assert_eq!(followup_records[0].value, 43 * 3);
    assert_eq!(followup_records[1].value, 34 * 3 + 2);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn module_path_join_context_uses_current_custom_pool_and_drives_followup_reductions() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("join-context-pipeline-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 29),
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 29));
    }

    let seed_sum: usize = seeds.iter().map(|record| record.seed).sum();
    let pipeline_records = Mutex::new(Vec::<ContextPipelineRecord>::new());

    let scope_summary = rayon_core::ThreadPool::scope(&pool, |scope| {
        let body_index = rayon_core::current_thread_index()
            .expect("ThreadPool::scope body should run in the custom pool");
        assert!(body_index < thread_count);

        for seed_record in seeds.iter().cloned() {
            let pipeline_records = &pipeline_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let caller_index = rayon_core::current_thread_index()
                    .expect("scoped work should run on a Rayon worker");
                assert!(caller_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let caller_pending_status_available =
                    rayon_core::current_thread_has_pending_tasks().is_some();

                let origin_index = seed_record.index;
                let seed = seed_record.seed;

                let ((left_worker, left_migrated, left_pending, left_value), (
                    right_worker,
                    right_migrated,
                    right_pending,
                    right_value,
                )) = rayon_core::join_context(
                    move |context| {
                        let migrated = rayon_core::FnContext::migrated(&context);
                        let worker = rayon_core::current_thread_index()
                            .expect("left join_context branch should run on a worker");

                        assert!(
                            !migrated,
                            "the left join_context branch should run immediately"
                        );
                        assert_eq!(worker, caller_index);
                        assert!(worker < thread_count);

                        (
                            worker,
                            migrated,
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                            seed + origin_index + worker,
                        )
                    },
                    move |context| {
                        let migrated = rayon_core::FnContext::migrated(&context);
                        let worker = rayon_core::current_thread_index()
                            .expect("right join_context branch should run on a worker");

                        assert!(worker < thread_count);
                        if migrated {
                            assert_ne!(
                                worker, caller_index,
                                "a migrated join_context branch should be stolen by another worker"
                            );
                        } else {
                            assert_eq!(
                                worker, caller_index,
                                "a non-migrated join_context branch should stay on the caller"
                            );
                        }

                        (
                            worker,
                            migrated,
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                            seed * 2
                                + caller_index
                                + worker
                                + thread_count
                                + usize::from(migrated),
                        )
                    },
                );

                pipeline_records
                    .lock()
                    .expect("pipeline record mutex should not be poisoned")
                    .push(ContextPipelineRecord {
                        origin_index,
                        seed,
                        caller_index,
                        num_threads: thread_count,
                        left_worker,
                        right_worker,
                        left_migrated,
                        right_migrated,
                        left_value,
                        right_value,
                        combined: left_value + right_value,
                        caller_pending_status_available,
                        left_pending_status_available: left_pending,
                        right_pending_status_available: right_pending,
                    });
            });
        }

        ScopeSummary {
            body_index,
            body_threads: rayon_core::current_num_threads(),
            scheduled_jobs: seeds.len(),
            seed_sum,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    assert!(scope_summary.body_index < thread_count);
    assert_eq!(scope_summary.body_threads, thread_count);
    assert_eq!(scope_summary.scheduled_jobs, thread_count);
    assert_eq!(scope_summary.seed_sum, seed_sum);
    assert!(scope_summary.pending_status_available);

    let mut pipeline_records = pipeline_records
        .into_inner()
        .expect("pipeline record mutex should not be poisoned");
    pipeline_records.sort_by_key(|record| record.origin_index);

    assert_eq!(pipeline_records.len(), thread_count);
    assert_eq!(
        pipeline_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &pipeline_records {
        assert!(record.origin_index < thread_count);
        assert!(record.caller_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.left_worker, record.caller_index);
        assert!(!record.left_migrated);
        assert!(record.right_worker < thread_count);

        if record.right_migrated {
            assert_ne!(record.right_worker, record.caller_index);
        } else {
            assert_eq!(record.right_worker, record.caller_index);
        }

        assert!(record.caller_pending_status_available);
        assert!(record.left_pending_status_available);
        assert!(record.right_pending_status_available);

        assert_eq!(record.seed, seeds[record.origin_index].seed);
        assert_eq!(
            record.left_value,
            record.seed + record.origin_index + record.left_worker
        );
        assert_eq!(
            record.right_value,
            record.seed * 2
                + record.caller_index
                + record.right_worker
                + thread_count
                + usize::from(record.right_migrated)
        );
        assert_eq!(record.combined, record.left_value + record.right_value);
    }

    let expected_combined_sum: usize = pipeline_records
        .iter()
        .map(|record| record.combined)
        .sum();

    let followup_records = Mutex::new(Vec::<ContextFollowupRecord>::new());

    let followup_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for record in pipeline_records.iter().cloned() {
            let followup_records = &followup_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("follow-up FIFO work should run in the custom pool");
                assert!(executing_index < thread_count);

                let origin_index = record.origin_index;
                let seed = record.seed;
                let combined = record.combined;

                let ((left_worker, left_migrated, left_value), (
                    right_worker,
                    right_migrated,
                    right_value,
                )) = rayon_core::join_context(
                    move |context| {
                        let migrated = rayon_core::FnContext::migrated(&context);
                        let worker = rayon_core::current_thread_index()
                            .expect("left follow-up join_context branch should run on a worker");

                        assert!(!migrated);
                        assert_eq!(worker, executing_index);

                        (worker, migrated, combined + worker)
                    },
                    move |context| {
                        let migrated = rayon_core::FnContext::migrated(&context);
                        let worker = rayon_core::current_thread_index()
                            .expect("right follow-up join_context branch should run on a worker");

                        assert!(worker < thread_count);
                        if migrated {
                            assert_ne!(worker, executing_index);
                        } else {
                            assert_eq!(worker, executing_index);
                        }

                        (
                            worker,
                            migrated,
                            seed
                                + origin_index
                                + executing_index
                                + worker
                                + usize::from(migrated),
                        )
                    },
                );

                followup_records
                    .lock()
                    .expect("follow-up record mutex should not be poisoned")
                    .push(ContextFollowupRecord {
                        origin_index,
                        executing_index,
                        left_worker,
                        right_worker,
                        left_migrated,
                        right_migrated,
                        value: left_value + right_value,
                    });
            });
        }

        expected_combined_sum
    });

    assert_eq!(followup_return, expected_combined_sum);

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
        assert_eq!(record.left_worker, record.executing_index);
        assert!(!record.left_migrated);

        if record.right_migrated {
            assert_ne!(record.right_worker, record.executing_index);
        } else {
            assert_eq!(record.right_worker, record.executing_index);
        }

        let source = pipeline_records
            .iter()
            .find(|candidate| candidate.origin_index == record.origin_index)
            .expect("follow-up record should correspond to a pipeline record");

        assert_eq!(
            record.value,
            source.combined
                + record.left_worker
                + source.seed
                + source.origin_index
                + record.executing_index
                + record.right_worker
                + usize::from(record.right_migrated)
        );
    }

    let (observed_pipeline_sum, observed_followup_sum) = rayon_core::ThreadPool::join(
        &pool,
        || pipeline_records.iter().map(|record| record.combined).sum::<usize>(),
        || followup_records.iter().map(|record| record.value).sum::<usize>(),
    );

    assert_eq!(observed_pipeline_sum, expected_combined_sum);
    assert_eq!(
        observed_followup_sum,
        followup_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>()
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn module_path_join_context_branch_panic_propagates_and_pool_recovers_for_later_context_joins() {
    let thread_count = 3usize;

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("join-context-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let left_started = AtomicUsize::new(0);
    let right_started = AtomicUsize::new(0);
    let left_value = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::scope(&pool, |_| {
            let _: (usize, usize) = rayon_core::join_context(
                |context| {
                    left_started.fetch_add(1, Ordering::SeqCst);

                    assert!(
                        !rayon_core::FnContext::migrated(&context),
                        "left panic-test branch should run directly"
                    );

                    let worker = rayon_core::current_thread_index()
                        .expect("left panic-test branch should run on a worker");
                    assert!(worker < thread_count);

                    let (left, right) = rayon_core::join_context(
                        |_| 13usize,
                        |_| 29usize,
                    );

                    let value = left + right + worker;
                    left_value.store(value, Ordering::SeqCst);
                    value
                },
                |context| -> usize {
                    right_started.fetch_add(1, Ordering::SeqCst);

                    let worker = rayon_core::current_thread_index()
                        .expect("right panic-test branch should run on a worker");
                    assert!(worker < thread_count);

                    let _migrated = rayon_core::FnContext::migrated(&context);
                    panic!(
                        "intentional panic from rayon_core::join::join_context right branch on worker {worker}"
                    );
                },
            );

            0usize
        });
    }));

    let payload = panic_result
        .expect_err("panic from rayon_core::join::join_context branch should propagate");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("rayon_core::join::join_context right branch"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(left_started.load(Ordering::SeqCst), 1);
    assert_eq!(right_started.load(Ordering::SeqCst), 1);
    assert!(
        left_value.load(Ordering::SeqCst) >= 42,
        "non-panicking join_context branch should complete before propagation"
    );

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "after unwinding, the external caller should still not be a pool worker"
    );

    let mut recovery_seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

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

    let seed_values: Vec<_> = recovery_seeds.iter().map(|(_, seed)| *seed).collect();
    let expected_sum: usize = seed_values.iter().sum();
    let expected_min = seed_values
        .iter()
        .copied()
        .min()
        .expect("recovery seeds should be nonempty");
    let expected_max = seed_values
        .iter()
        .copied()
        .max()
        .expect("recovery seeds should be nonempty");

    let ((sum_worker, sum_threads, recovered_sum), (
        extrema_worker,
        extrema_threads,
        recovered_min,
        recovered_max,
    )) = rayon_core::ThreadPool::join(
        &pool,
        || {
            let caller = rayon_core::current_thread_index()
                .expect("recovery sum branch should run in the custom pool");
            assert!(caller < thread_count);

            let split = seed_values.len() / 2;
            let (left_half, right_half) = seed_values.split_at(split);

            let (left_sum, right_sum) = rayon_core::join_context(
                |context| {
                    assert!(!rayon_core::FnContext::migrated(&context));
                    assert_eq!(rayon_core::current_thread_index(), Some(caller));
                    left_half.iter().copied().sum::<usize>()
                },
                |context| {
                    let migrated = rayon_core::FnContext::migrated(&context);
                    let worker = rayon_core::current_thread_index()
                        .expect("right recovery sum branch should run on a worker");

                    if migrated {
                        assert_ne!(worker, caller);
                    } else {
                        assert_eq!(worker, caller);
                    }

                    right_half.iter().copied().sum::<usize>()
                },
            );

            (
                caller,
                rayon_core::current_num_threads(),
                left_sum + right_sum,
            )
        },
        || {
            let caller = rayon_core::current_thread_index()
                .expect("recovery extrema branch should run in the custom pool");
            assert!(caller < thread_count);

            let (minimum, maximum) = rayon_core::join_context(
                |context| {
                    assert!(!rayon_core::FnContext::migrated(&context));
                    seed_values.iter().copied().min()
                },
                |context| {
                    let worker = rayon_core::current_thread_index()
                        .expect("right recovery extrema branch should run on a worker");
                    assert!(worker < thread_count);

                    let _migrated = rayon_core::FnContext::migrated(&context);
                    seed_values.iter().copied().max()
                },
            );

            (
                caller,
                rayon_core::current_num_threads(),
                minimum.expect("minimum should be present"),
                maximum.expect("maximum should be present"),
            )
        },
    );

    assert!(sum_worker < thread_count);
    assert!(extrema_worker < thread_count);
    assert_eq!(sum_threads, thread_count);
    assert_eq!(extrema_threads, thread_count);
    assert_eq!(recovered_sum, expected_sum);
    assert_eq!(recovered_min, expected_min);
    assert_eq!(recovered_max, expected_max);

    let (combined, recomputed) = rayon_core::ThreadPool::join(
        &pool,
        || recovered_sum + recovered_min + recovered_max,
        || expected_sum + expected_min + expected_max,
    );

    assert_eq!(combined, recomputed);
}