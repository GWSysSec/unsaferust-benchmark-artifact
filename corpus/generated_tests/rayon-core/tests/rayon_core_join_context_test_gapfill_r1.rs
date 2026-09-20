use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExternalBranchRecord {
    branch: &'static str,
    worker_index: Option<usize>,
    num_threads: usize,
    same_thread_as_caller: bool,
    migrated: bool,
    total: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExternalValidationRecord {
    branch: &'static str,
    executing_index: usize,
    left_worker: usize,
    right_worker: usize,
    left_migrated: bool,
    right_migrated: bool,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleWorkerBranchRecord {
    branch: &'static str,
    worker_index: Option<usize>,
    num_threads: usize,
    migrated: bool,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleWorkerFollowupRecord {
    slot: usize,
    worker_index: Option<usize>,
    num_threads: usize,
    left_migrated: bool,
    right_migrated: bool,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SingleWorkerSummary {
    body_index: Option<usize>,
    body_threads: usize,
    branch_total: usize,
    scheduled_followups: usize,
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
fn free_join_context_from_external_thread_reports_migration_and_feeds_scoped_validation() {
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "the integration-test thread should begin outside any Rayon worker"
    );

    let caller_thread = std::thread::current().id();
    let values: Vec<usize> = (1..=24).collect();
    let split = values.len() / 2;
    let (left_values, right_values) = values.split_at(split);

    let expected_left_sum: usize = left_values.iter().copied().sum();
    let expected_right_weighted: usize = right_values
        .iter()
        .enumerate()
        .map(|(offset, value)| (split + offset + 1) * *value)
        .sum();

    let ((left_record, left_sum), (right_record, right_weighted)) =
        rayon_core::join_context(
            |context| {
                let migrated = rayon_core::FnContext::migrated(&context);
                let worker_index = rayon_core::current_thread_index();
                let num_threads = rayon_core::current_num_threads();
                let same_thread_as_caller = std::thread::current().id() == caller_thread;

                let (first_half, second_half) = left_values.split_at(left_values.len() / 2);
                let (first_sum, second_sum) = rayon_core::join_context(
                    |_| first_half.iter().copied().sum::<usize>(),
                    |_| second_half.iter().copied().sum::<usize>(),
                );
                let total = first_sum + second_sum;

                (
                    ExternalBranchRecord {
                        branch: "left-sum",
                        worker_index,
                        num_threads,
                        same_thread_as_caller,
                        migrated,
                        total,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    },
                    total,
                )
            },
            |context| {
                let migrated = rayon_core::FnContext::migrated(&context);
                let worker_index = rayon_core::current_thread_index();
                let num_threads = rayon_core::current_num_threads();
                let same_thread_as_caller = std::thread::current().id() == caller_thread;

                let (first_half, second_half) = right_values.split_at(right_values.len() / 2);
                let (first_weighted, second_weighted) = rayon_core::join_context(
                    |_| {
                        first_half
                            .iter()
                            .enumerate()
                            .map(|(offset, value)| (split + offset + 1) * *value)
                            .sum::<usize>()
                    },
                    |_| {
                        second_half
                            .iter()
                            .enumerate()
                            .map(|(offset, value)| {
                                (split + first_half.len() + offset + 1) * *value
                            })
                            .sum::<usize>()
                    },
                );
                let total = first_weighted + second_weighted;

                (
                    ExternalBranchRecord {
                        branch: "right-weighted",
                        worker_index,
                        num_threads,
                        same_thread_as_caller,
                        migrated,
                        total,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    },
                    total,
                )
            },
        );

    assert_eq!(left_sum, expected_left_sum);
    assert_eq!(right_weighted, expected_right_weighted);
    assert_eq!(left_record.total, expected_left_sum);
    assert_eq!(right_record.total, expected_right_weighted);

    for record in [&left_record, &right_record] {
        assert!(record.num_threads > 0);
        assert!(record.num_threads <= rayon_core::max_num_threads());
        assert_eq!(
            record.migrated, !record.same_thread_as_caller,
            "FnContext::migrated should describe whether the branch ran on the caller thread"
        );

        if let Some(worker_index) = record.worker_index {
            assert!(worker_index < record.num_threads);
            assert!(
                record.pending_status_available,
                "Rayon worker branches should be able to query pending-task status"
            );
            assert!(
                !record.same_thread_as_caller,
                "an external caller should not also be reported as a Rayon worker"
            );
        } else {
            assert!(
                !record.pending_status_available,
                "pending-task status should be unavailable on a non-worker caller thread"
            );
        }
    }

    let validation_records = Mutex::new(Vec::<ExternalValidationRecord>::new());

    let scope_return = rayon_core::scope(|scope| {
        for record in [left_record.clone(), right_record.clone()] {
            let validation_records = &validation_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped validation should execute on a Rayon worker");
                assert!(executing_index < rayon_core::current_num_threads());

                let branch = record.branch;
                let base = record.total;

                let ((left_worker, left_migrated, left_value), (
                    right_worker,
                    right_migrated,
                    right_value,
                )) = rayon_core::join_context(
                    move |context| {
                        let migrated = rayon_core::FnContext::migrated(&context);
                        let worker = rayon_core::current_thread_index()
                            .expect("left validation branch should run on a Rayon worker");

                        assert!(
                            !migrated,
                            "left join_context branch should run directly on the caller worker"
                        );
                        assert_eq!(worker, executing_index);

                        (worker, migrated, base + executing_index + worker)
                    },
                    move |context| {
                        let migrated = rayon_core::FnContext::migrated(&context);
                        let worker = rayon_core::current_thread_index()
                            .expect("right validation branch should run on a Rayon worker");

                        if migrated {
                            assert_ne!(
                                worker, executing_index,
                                "a migrated right branch should be stolen by another worker"
                            );
                        } else {
                            assert_eq!(
                                worker, executing_index,
                                "a non-migrated right branch should stay on the caller worker"
                            );
                        }

                        (worker, migrated, base * 2 + worker + usize::from(migrated))
                    },
                );

                validation_records
                    .lock()
                    .expect("validation mutex should not be poisoned")
                    .push(ExternalValidationRecord {
                        branch,
                        executing_index,
                        left_worker,
                        right_worker,
                        left_migrated,
                        right_migrated,
                        value: left_value + right_value,
                    });
            });
        }

        left_record.total + right_record.total
    });

    assert_eq!(scope_return, expected_left_sum + expected_right_weighted);

    let mut validation_records = validation_records
        .into_inner()
        .expect("validation mutex should not be poisoned");
    validation_records.sort_by_key(|record| record.branch);

    assert_eq!(validation_records.len(), 2);

    for record in &validation_records {
        assert_eq!(record.left_worker, record.executing_index);
        assert!(!record.left_migrated);

        if record.right_migrated {
            assert_ne!(record.right_worker, record.executing_index);
        } else {
            assert_eq!(record.right_worker, record.executing_index);
        }

        let source_total = match record.branch {
            "left-sum" => expected_left_sum,
            "right-weighted" => expected_right_weighted,
            unexpected => panic!("unexpected validation branch label: {unexpected}"),
        };

        assert_eq!(
            record.value,
            source_total
                + record.executing_index
                + record.left_worker
                + source_total * 2
                + record.right_worker
                + usize::from(record.right_migrated)
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn join_context_single_worker_pool_has_stable_non_migrated_context_and_drives_fifo_followups() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("join-context-single-worker-gapfill-{index}"))
        .build()
        .expect("single-worker pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let branch_records = Mutex::new(Vec::<SingleWorkerBranchRecord>::new());
    let followup_records = Mutex::new(Vec::<SingleWorkerFollowupRecord>::new());

    let summary = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        assert_eq!(rayon_core::current_thread_index(), Some(0));
        assert_eq!(rayon_core::current_num_threads(), 1);

        let ((left_value, left_record), (right_value, right_record)) =
            rayon_core::join_context(
                |context| {
                    let migrated = rayon_core::FnContext::migrated(&context);
                    assert!(!migrated);
                    assert_eq!(rayon_core::current_thread_index(), Some(0));

                    let (a, b) = rayon_core::join_context(
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            assert_eq!(rayon_core::current_thread_index(), Some(0));
                            10usize
                        },
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            assert_eq!(rayon_core::current_thread_index(), Some(0));
                            20usize
                        },
                    );

                    let value = a + b + rayon_core::current_num_threads();

                    (
                        value,
                        SingleWorkerBranchRecord {
                            branch: "left",
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
                    assert!(!migrated);
                    assert_eq!(rayon_core::current_thread_index(), Some(0));

                    let (a, b) = rayon_core::join_context(
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            5usize
                        },
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            7usize
                        },
                    );

                    let value = a * b + rayon_core::current_num_threads();

                    (
                        value,
                        SingleWorkerBranchRecord {
                            branch: "right",
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
            .expect("branch mutex should not be poisoned")
            .extend([left_record, right_record]);

        for (slot, base_value) in [(0usize, left_value), (1usize, right_value)] {
            let followup_records = &followup_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                assert_eq!(rayon_core::current_thread_index(), Some(0));
                assert_eq!(rayon_core::current_num_threads(), 1);

                let ((left_migrated, left), (right_migrated, right)) =
                    rayon_core::join_context(
                        move |context| {
                            let migrated = rayon_core::FnContext::migrated(&context);
                            assert!(!migrated);
                            (migrated, base_value + slot)
                        },
                        move |context| {
                            let migrated = rayon_core::FnContext::migrated(&context);
                            assert!(!migrated);
                            (migrated, base_value * 2 + slot)
                        },
                    );

                followup_records
                    .lock()
                    .expect("follow-up mutex should not be poisoned")
                    .push(SingleWorkerFollowupRecord {
                        slot,
                        worker_index: rayon_core::current_thread_index(),
                        num_threads: rayon_core::current_num_threads(),
                        left_migrated,
                        right_migrated,
                        value: left + right,
                    });
            });
        }

        SingleWorkerSummary {
            body_index: rayon_core::current_thread_index(),
            body_threads: rayon_core::current_num_threads(),
            branch_total: left_value + right_value,
            scheduled_followups: 2,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    assert_eq!(summary.body_index, Some(0));
    assert_eq!(summary.body_threads, 1);
    assert_eq!(summary.branch_total, 31 + 36);
    assert_eq!(summary.scheduled_followups, 2);
    assert!(summary.pending_status_available);

    let mut branch_records = branch_records
        .into_inner()
        .expect("branch mutex should not be poisoned");
    branch_records.sort_by_key(|record| record.branch);

    assert_eq!(branch_records.len(), 2);
    assert_eq!(branch_records[0].branch, "left");
    assert_eq!(branch_records[0].worker_index, Some(0));
    assert_eq!(branch_records[0].num_threads, 1);
    assert!(!branch_records[0].migrated);
    assert_eq!(branch_records[0].value, 31);
    assert!(branch_records[0].pending_status_available);

    assert_eq!(branch_records[1].branch, "right");
    assert_eq!(branch_records[1].worker_index, Some(0));
    assert_eq!(branch_records[1].num_threads, 1);
    assert!(!branch_records[1].migrated);
    assert_eq!(branch_records[1].value, 36);
    assert!(branch_records[1].pending_status_available);

    let mut followup_records = followup_records
        .into_inner()
        .expect("follow-up mutex should not be poisoned");
    followup_records.sort_by_key(|record| record.slot);

    assert_eq!(followup_records.len(), 2);

    for record in &followup_records {
        assert_eq!(record.worker_index, Some(0));
        assert_eq!(record.num_threads, 1);
        assert!(!record.left_migrated);
        assert!(!record.right_migrated);
    }

    assert_eq!(followup_records[0].value, 31 * 3);
    assert_eq!(followup_records[1].value, 36 * 3 + 2);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn join_context_branch_panic_propagates_and_custom_pool_recovers_with_later_context_joins() {
    let thread_count = 3usize;
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("join-context-panic-recovery-gapfill-{index}"))
        .build()
        .expect("custom pool should build");

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
                        "left branch should run immediately on the calling worker"
                    );

                    let worker = rayon_core::current_thread_index()
                        .expect("left panic-test branch should run on a Rayon worker");
                    assert!(worker < thread_count);

                    let (a, b) = rayon_core::join_context(
                        |inner_context| {
                            assert!(!rayon_core::FnContext::migrated(&inner_context));
                            41usize
                        },
                        |inner_context| {
                            let _ = rayon_core::FnContext::migrated(&inner_context);
                            1usize
                        },
                    );

                    let value = a + b + worker;
                    left_value.store(value, Ordering::SeqCst);
                    value
                },
                |context| -> usize {
                    right_started.fetch_add(1, Ordering::SeqCst);

                    let worker = rayon_core::current_thread_index()
                        .expect("right panic-test branch should run on a Rayon worker");
                    assert!(worker < thread_count);

                    let _ = rayon_core::FnContext::migrated(&context);
                    panic!("intentional rayon_core::join_context panic on worker {worker}");
                },
            );

            0usize
        });
    }));

    let payload = panic_result.expect_err("join_context branch panic should propagate");
    let message = panic_payload_to_string(&*payload);

    assert!(
        message.contains("intentional rayon_core::join_context panic"),
        "unexpected panic payload: {message:?}"
    );
    assert_eq!(left_started.load(Ordering::SeqCst), 1);
    assert_eq!(right_started.load(Ordering::SeqCst), 1);
    assert!(
        left_value.load(Ordering::SeqCst) >= 42,
        "the non-panicking branch should complete before panic propagation"
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

    let ((sum_worker, sum_threads, sum_pending, recovered_sum), (
        range_worker,
        range_threads,
        range_pending,
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
                    assert!(worker < thread_count);

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
                rayon_core::current_thread_has_pending_tasks().is_some(),
                left_sum + right_sum,
            )
        },
        || {
            let caller = rayon_core::current_thread_index()
                .expect("recovery range branch should run in the custom pool");
            assert!(caller < thread_count);

            let (minimum, maximum) = rayon_core::join_context(
                |context| {
                    assert!(!rayon_core::FnContext::migrated(&context));
                    assert_eq!(rayon_core::current_thread_index(), Some(caller));
                    seed_values.iter().copied().min()
                },
                |context| {
                    let migrated = rayon_core::FnContext::migrated(&context);
                    let worker = rayon_core::current_thread_index()
                        .expect("right recovery range branch should run on a worker");
                    assert!(worker < thread_count);

                    if migrated {
                        assert_ne!(worker, caller);
                    } else {
                        assert_eq!(worker, caller);
                    }

                    seed_values.iter().copied().max()
                },
            );

            (
                caller,
                rayon_core::current_num_threads(),
                rayon_core::current_thread_has_pending_tasks().is_some(),
                minimum.expect("minimum should be present"),
                maximum.expect("maximum should be present"),
            )
        },
    );

    assert!(sum_worker < thread_count);
    assert!(range_worker < thread_count);
    assert_eq!(sum_threads, thread_count);
    assert_eq!(range_threads, thread_count);
    assert!(sum_pending);
    assert!(range_pending);
    assert_eq!(recovered_sum, expected_sum);
    assert_eq!(recovered_min, expected_min);
    assert_eq!(recovered_max, expected_max);

    let (combined, recomputed) = rayon_core::ThreadPool::join(
        &pool,
        || recovered_sum + recovered_min + recovered_max + left_value.load(Ordering::SeqCst),
        || expected_sum + expected_min + expected_max + left_value.load(Ordering::SeqCst),
    );

    assert_eq!(combined, recomputed);
}