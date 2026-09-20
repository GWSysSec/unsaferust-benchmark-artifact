use std::collections::BTreeSet;
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
struct JoinBranchReport {
    branch: &'static str,
    worker_index: usize,
    num_threads: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedJoinRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    joined_left: usize,
    joined_right: usize,
    combined: usize,
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
fn thread_pool_join_consumes_broadcast_results_and_feeds_scoped_follow_up_work() {
    let thread_count = 4usize;

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("thread-pool-join-pipeline-worker-{index}"))
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

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 7),
        }
    });

    seeds.sort_by_key(|record| record.index);

    assert_eq!(seeds.len(), thread_count);
    assert_eq!(
        seeds
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(thread_count)
    );

    for (expected_index, record) in seeds.iter().enumerate() {
        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(expected_index));
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 7));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();
    let expected_weighted_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| (index + 1) * *seed)
        .sum();

    let ((seed_report, observed_seed_sum), (weighted_report, observed_weighted_sum)) =
        rayon_core::ThreadPool::join(
            pool_ref,
            || {
                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("left ThreadPool::join branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                let split = seed_by_index.len() / 2;
                let (left_half, right_half) = seed_by_index.split_at(split);
                let (left_sum, right_sum) = rayon_core::join(
                    || left_half.iter().copied().sum::<usize>(),
                    || right_half.iter().copied().sum::<usize>(),
                );

                (
                    JoinBranchReport {
                        branch: "seed-sum",
                        worker_index,
                        num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    },
                    left_sum + right_sum,
                )
            },
            || {
                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("right ThreadPool::join branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                let (even_weighted, odd_weighted) = rayon_core::join(
                    || {
                        seed_by_index
                            .iter()
                            .enumerate()
                            .step_by(2)
                            .map(|(index, seed)| (index + 1) * *seed)
                            .sum::<usize>()
                    },
                    || {
                        seed_by_index
                            .iter()
                            .enumerate()
                            .skip(1)
                            .step_by(2)
                            .map(|(index, seed)| (index + 1) * *seed)
                            .sum::<usize>()
                    },
                );

                (
                    JoinBranchReport {
                        branch: "weighted-sum",
                        worker_index,
                        num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    },
                    even_weighted + odd_weighted,
                )
            },
        );

    assert_eq!(seed_report.branch, "seed-sum");
    assert_eq!(weighted_report.branch, "weighted-sum");

    for report in [&seed_report, &weighted_report] {
        assert!(report.worker_index < thread_count);
        assert_eq!(report.num_threads, thread_count);
        assert!(
            report.pending_status_available,
            "ThreadPool::join branch should be able to query pending-task status"
        );
    }

    assert_eq!(observed_seed_sum, expected_seed_sum);
    assert_eq!(observed_weighted_sum, expected_weighted_sum);

    let scoped_records = Mutex::new(Vec::<ScopedJoinRecord>::new());
    let seed_branch_worker = seed_report.worker_index;
    let weighted_branch_worker = weighted_report.worker_index;

    let scoped_return = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        for record in seeds.iter().cloned() {
            let scoped_records = &scoped_records;
            let origin_index = record.index;
            let seed = record.seed;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("scoped follow-up work should run inside the custom pool");
                assert!(executing_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );
                assert!(
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some()
                );

                let (joined_left, joined_right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || {
                        let inner_index =
                            rayon_core::ThreadPool::current_thread_index(pool_ref)
                                .expect("nested left join branch should run in the pool");
                        assert!(inner_index < thread_count);
                        seed + origin_index + observed_seed_sum
                    },
                    move || {
                        let inner_index =
                            rayon_core::ThreadPool::current_thread_index(pool_ref)
                                .expect("nested right join branch should run in the pool");
                        assert!(inner_index < thread_count);
                        observed_weighted_sum
                            + executing_index
                            + seed_branch_worker
                            + weighted_branch_worker
                    },
                );

                scoped_records
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(ScopedJoinRecord {
                        origin_index,
                        seed,
                        executing_index,
                        joined_left,
                        joined_right,
                        combined: joined_left + joined_right,
                    });
            });
        }

        observed_seed_sum + observed_weighted_sum
    });

    assert_eq!(scoped_return, expected_seed_sum + expected_weighted_sum);

    let mut scoped_records = scoped_records
        .into_inner()
        .expect("scoped record mutex should not be poisoned");
    scoped_records.sort_by_key(|record| record.origin_index);

    assert_eq!(scoped_records.len(), thread_count);
    assert_eq!(
        scoped_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(thread_count)
    );

    for record in &scoped_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.joined_left,
            record.seed + record.origin_index + expected_seed_sum
        );
        assert_eq!(
            record.joined_right,
            expected_weighted_sum
                + record.executing_index
                + seed_branch_worker
                + weighted_branch_worker
        );
        assert_eq!(record.combined, record.joined_left + record.joined_right);
    }

    let expected_combined_sum: usize = scoped_records.iter().map(|record| record.combined).sum();

    let (left_total, right_total) = rayon_core::ThreadPool::join(
        pool_ref,
        || scoped_records.iter().map(|record| record.joined_left).sum::<usize>(),
        || {
            scoped_records
                .iter()
                .map(|record| record.joined_right)
                .sum::<usize>()
        },
    );

    assert_eq!(left_total + right_total, expected_combined_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_join_propagates_branch_panic_and_pool_recovers_for_later_joins() {
    let thread_count = 3usize;

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("thread-pool-join-panic-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");
    let pool_ref = &pool;

    let left_started = AtomicUsize::new(0);
    let right_started = AtomicUsize::new(0);
    let left_value = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: (usize, usize) = rayon_core::ThreadPool::join(
            pool_ref,
            || {
                left_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("non-panicking join branch should run in the custom pool");
                assert!(worker_index < thread_count);

                let (six, seven) = rayon_core::join(|| 6usize, || 7usize);
                let value = six * seven;
                left_value.store(value, Ordering::SeqCst);
                value
            },
            || -> usize {
                right_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking join branch should run in the custom pool");
                assert!(worker_index < thread_count);

                panic!("intentional ThreadPool::join panic from worker {worker_index}");
            },
        );
    }));

    let payload =
        panic_result.expect_err("a panic in a ThreadPool::join branch should propagate");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional ThreadPool::join panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(left_started.load(Ordering::SeqCst), 1);
    assert_eq!(right_started.load(Ordering::SeqCst), 1);
    assert_eq!(left_value.load(Ordering::SeqCst), 42);

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the caller should still not be a pool worker"
    );

    let mut recovery_seeds = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, (index + 2) * (num_threads + 5))
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

    let seed_by_index: Vec<_> = recovery_seeds.iter().map(|(_, seed)| *seed).collect();
    let expected_sum: usize = seed_by_index.iter().sum();
    let expected_min = seed_by_index
        .iter()
        .copied()
        .min()
        .expect("recovery broadcast should produce at least one seed");
    let expected_max = seed_by_index
        .iter()
        .copied()
        .max()
        .expect("recovery broadcast should produce at least one seed");

    let ((sum_report, observed_sum), (extrema_report, (observed_min, observed_max))) =
        rayon_core::ThreadPool::join(
            pool_ref,
            || {
                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery sum join branch should run in the custom pool");
                assert!(worker_index < thread_count);

                let split = seed_by_index.len() / 2;
                let (left_half, right_half) = seed_by_index.split_at(split);
                let (left_sum, right_sum) = rayon_core::join(
                    || left_half.iter().copied().sum::<usize>(),
                    || right_half.iter().copied().sum::<usize>(),
                );

                (
                    JoinBranchReport {
                        branch: "recovery-sum",
                        worker_index,
                        num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    },
                    left_sum + right_sum,
                )
            },
            || {
                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery extrema join branch should run in the custom pool");
                assert!(worker_index < thread_count);

                let (minimum, maximum) = rayon_core::join(
                    || seed_by_index.iter().copied().min(),
                    || seed_by_index.iter().copied().max(),
                );

                (
                    JoinBranchReport {
                        branch: "recovery-extrema",
                        worker_index,
                        num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    },
                    (minimum, maximum),
                )
            },
        );

    assert_eq!(sum_report.branch, "recovery-sum");
    assert_eq!(extrema_report.branch, "recovery-extrema");

    for report in [&sum_report, &extrema_report] {
        assert!(report.worker_index < thread_count);
        assert_eq!(report.num_threads, thread_count);
        assert!(
            report.pending_status_available,
            "recovery join branch should observe worker-local pending-task status"
        );
    }

    assert_eq!(observed_sum, expected_sum);

    let observed_min = observed_min.expect("minimum should be present after recovery broadcast");
    let observed_max = observed_max.expect("maximum should be present after recovery broadcast");
    assert_eq!(observed_min, expected_min);
    assert_eq!(observed_max, expected_max);

    let (combined_total, worker_pair_label) = rayon_core::ThreadPool::join(
        pool_ref,
        || observed_sum + observed_min + observed_max,
        || {
            format!(
                "{}->{}",
                sum_report.worker_index, extrema_report.worker_index
            )
        },
    );

    assert_eq!(combined_total, expected_sum + expected_min + expected_max);
    assert_eq!(
        worker_pair_label,
        format!(
            "{}->{}",
            sum_report.worker_index, extrema_report.worker_index
        )
    );
}