use rayon_core::{BroadcastContext, Scope, ThreadPool, ThreadPoolBuilder};

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChunkValidation {
    chunk_index: usize,
    start_index: usize,
    len: usize,
    first: i64,
    last: i64,
    local_sum: i64,
    local_weighted_sum: i64,
    worker_index: Option<usize>,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PoolSeed {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PoolJoinReport {
    label: &'static str,
    worker_index: usize,
    num_threads: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedJoinRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    left_value: usize,
    right_value: usize,
    combined: usize,
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

fn partition(values: &mut [i64]) -> usize {
    let pivot = values.len() - 1;
    let mut store = 0;

    for index in 0..pivot {
        if values[index] <= values[pivot] {
            values.swap(store, index);
            store += 1;
        }
    }

    values.swap(store, pivot);
    store
}

fn join_quick_sort(values: &mut [i64]) {
    if values.len() <= 1 {
        return;
    }

    let pivot = partition(values);
    let (left, pivot_and_right) = values.split_at_mut(pivot);
    let (_, right) = pivot_and_right.split_at_mut(1);

    rayon_core::join(|| join_quick_sort(left), || join_quick_sort(right));
}

fn join_sum(values: &[i64]) -> i64 {
    if values.len() <= 4 {
        values.iter().copied().sum()
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let (left_sum, right_sum) = rayon_core::join(|| join_sum(left), || join_sum(right));

        left_sum + right_sum
    }
}

fn join_weighted_sum(values: &[i64], base_index: usize) -> i64 {
    if values.len() <= 4 {
        values
            .iter()
            .enumerate()
            .map(|(offset, value)| (base_index + offset + 1) as i64 * *value)
            .sum()
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let (left_weighted, right_weighted) = rayon_core::join(
            || join_weighted_sum(left, base_index),
            || join_weighted_sum(right, base_index + mid),
        );

        left_weighted + right_weighted
    }
}

fn join_min_max(values: &[i64]) -> Option<(i64, i64)> {
    if values.is_empty() {
        None
    } else if values.len() <= 4 {
        Some((
            values.iter().copied().min().expect("nonempty slice"),
            values.iter().copied().max().expect("nonempty slice"),
        ))
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let (left_range, right_range) =
            rayon_core::join(|| join_min_max(left), || join_min_max(right));

        match (left_range, right_range) {
            (Some((left_min, left_max)), Some((right_min, right_max))) => {
                Some((left_min.min(right_min), left_max.max(right_max)))
            }
            (Some(range), None) | (None, Some(range)) => Some(range),
            (None, None) => None,
        }
    }
}

fn join_is_sorted(values: &[i64]) -> bool {
    if values.len() <= 8 {
        values.windows(2).all(|pair| pair[0] <= pair[1])
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let boundary_ok = left.last().expect("left half should be nonempty")
            <= right.first().expect("right half should be nonempty");

        let (left_sorted, right_sorted) =
            rayon_core::join(|| join_is_sorted(left), || join_is_sorted(right));

        left_sorted && right_sorted && boundary_ok
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_join_recursively_sorts_borrowed_data_then_scoped_workers_validate_chunks() {
    let mut values = vec![
        42, -7, 13, 13, 0, 5, -21, 8, 34, 1, -3, 55, 2, 2, 89, -34, 21, 13, -8,
    ];
    let original_values = values.clone();

    let mut expected_sorted = original_values.clone();
    expected_sorted.sort();

    let ((), ((original_sum, original_weighted_sum), original_range)) = rayon_core::join(
        || join_quick_sort(&mut values),
        || {
            rayon_core::join(
                || {
                    (
                        join_sum(&original_values),
                        join_weighted_sum(&original_values, 0),
                    )
                },
                || join_min_max(&original_values).expect("test input should be nonempty"),
            )
        },
    );

    assert_eq!(values, expected_sorted);
    assert_eq!(original_sum, expected_sorted.iter().copied().sum::<i64>());
    assert_eq!(
        original_range,
        (
            *expected_sorted.first().expect("sorted values should be nonempty"),
            *expected_sorted.last().expect("sorted values should be nonempty"),
        )
    );

    let ((sorted_ok, sorted_range), (sorted_sum, sorted_weighted_sum)) = rayon_core::join(
        || {
            (
                join_is_sorted(&values),
                join_min_max(&values).expect("sorted values should be nonempty"),
            )
        },
        || rayon_core::join(|| join_sum(&values), || join_weighted_sum(&values, 0)),
    );

    assert!(sorted_ok);
    assert_eq!(sorted_range, original_range);
    assert_eq!(sorted_sum, original_sum);
    assert_eq!(
        sorted_weighted_sum,
        join_weighted_sum(&expected_sorted, 0)
    );
    assert_ne!(
        original_weighted_sum, sorted_weighted_sum,
        "the intentionally unsorted input should be meaningfully reordered by join_quick_sort"
    );

    let chunk_size = (sorted_weighted_sum.rem_euclid(5) as usize) + 3;
    let chunk_validations = Mutex::new(Vec::<ChunkValidation>::new());

    let scope_return = rayon_core::scope(|scope| {
        for (chunk_index, chunk) in values.chunks(chunk_size).enumerate() {
            let start_index = chunk_index * chunk_size;
            let chunk_validations = &chunk_validations;

            Scope::spawn(scope, move |_| {
                let ((first, last), (local_sum, local_weighted_sum)) = rayon_core::join(
                    || {
                        (
                            *chunk.first().expect("chunk should be nonempty"),
                            *chunk.last().expect("chunk should be nonempty"),
                        )
                    },
                    || {
                        rayon_core::join(
                            || join_sum(chunk),
                            || join_weighted_sum(chunk, start_index),
                        )
                    },
                );

                assert!(first <= last);

                let worker_index = rayon_core::current_thread_index();
                if let Some(index) = worker_index {
                    assert!(index < rayon_core::current_num_threads());
                }

                chunk_validations
                    .lock()
                    .expect("chunk validation mutex should not be poisoned")
                    .push(ChunkValidation {
                        chunk_index,
                        start_index,
                        len: chunk.len(),
                        first,
                        last,
                        local_sum,
                        local_weighted_sum,
                        worker_index,
                        pending_status_available: rayon_core::current_thread_has_pending_tasks()
                            .is_some(),
                    });
            });
        }

        sorted_sum + sorted_weighted_sum
    });

    assert_eq!(scope_return, sorted_sum + sorted_weighted_sum);

    let mut chunk_validations = chunk_validations
        .into_inner()
        .expect("chunk validation mutex should not be poisoned");
    chunk_validations.sort_by_key(|record| record.chunk_index);

    assert_eq!(chunk_validations.len(), values.chunks(chunk_size).count());

    let mut covered = 0usize;
    for record in &chunk_validations {
        assert_eq!(record.start_index, covered);
        assert!(record.len > 0);

        let chunk = &values[record.start_index..record.start_index + record.len];

        assert_eq!(record.first, *chunk.first().expect("record chunk exists"));
        assert_eq!(record.last, *chunk.last().expect("record chunk exists"));
        assert_eq!(record.local_sum, chunk.iter().copied().sum::<i64>());
        assert_eq!(
            record.local_weighted_sum,
            join_weighted_sum(chunk, record.start_index)
        );

        if record.worker_index.is_some() {
            assert!(
                record.pending_status_available,
                "scoped chunk work running on a Rayon worker should observe pending-task status"
            );
        }

        covered += record.len;
    }

    assert_eq!(covered, values.len());
    assert_eq!(
        chunk_validations
            .iter()
            .map(|record| record.local_sum)
            .sum::<i64>(),
        sorted_sum
    );
    assert_eq!(
        chunk_validations
            .iter()
            .map(|record| record.local_weighted_sum)
            .sum::<i64>(),
        sorted_weighted_sum
    );

    let chunk_sum_total: i64 = chunk_validations
        .iter()
        .map(|record| record.local_sum)
        .sum();
    let chunk_weighted_total: i64 = chunk_validations
        .iter()
        .map(|record| record.local_weighted_sum)
        .sum();

    let mut confirmations = rayon_core::broadcast(|context| {
        let index = BroadcastContext::index(&context);
        let num_threads = BroadcastContext::num_threads(&context);

        assert!(index < num_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let (left, right) = rayon_core::join(
            move || chunk_sum_total,
            move || chunk_weighted_total + index as i64 + num_threads as i64,
        );

        (
            index,
            num_threads,
            rayon_core::current_thread_index(),
            left + right,
        )
    });

    confirmations.sort_by_key(|record| record.0);

    let global_threads = rayon_core::current_num_threads();
    assert_eq!(confirmations.len(), global_threads);
    assert_eq!(
        confirmations
            .iter()
            .map(|(index, _, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for (index, num_threads, current_index, total) in confirmations {
        assert_eq!(num_threads, global_threads);
        assert_eq!(current_index, Some(index));
        assert_eq!(
            total,
            chunk_sum_total + chunk_weighted_total + index as i64 + global_threads as i64
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_join_reduces_broadcast_data_feeds_scope_and_recovers_after_branch_panic() {
    let thread_count = 4usize;

    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("thread-pool-join-target-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");
    let pool_ref = &pool;

    assert_eq!(ThreadPool::current_num_threads(pool_ref), thread_count);
    assert_eq!(ThreadPool::current_thread_index(pool_ref), None);
    assert_eq!(
        ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None,
        "outside the custom pool, pending-task status should be unavailable"
    );

    let mut seeds = ThreadPool::broadcast(pool_ref, |context| {
        let index = BroadcastContext::index(&context);
        let num_threads = BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        PoolSeed {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 17),
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 17));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();
    let expected_weighted_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| (index + 1) * *seed)
        .sum();

    let ((sum_report, observed_seed_sum), (weighted_report, observed_weighted_sum)) =
        ThreadPool::join(
            pool_ref,
            || {
                let worker_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("left ThreadPool::join branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                let split = seed_by_index.len() / 2;
                let (left_half, right_half) = seed_by_index.split_at(split);

                let (left_sum, right_sum) = ThreadPool::join(
                    pool_ref,
                    || left_half.iter().copied().sum::<usize>(),
                    || right_half.iter().copied().sum::<usize>(),
                );

                (
                    PoolJoinReport {
                        label: "seed-sum",
                        worker_index,
                        num_threads: ThreadPool::current_num_threads(pool_ref),
                        pending_status_available:
                            ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some(),
                    },
                    left_sum + right_sum,
                )
            },
            || {
                let worker_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("right ThreadPool::join branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                let (even_weighted, odd_weighted) = ThreadPool::join(
                    pool_ref,
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
                    PoolJoinReport {
                        label: "weighted-sum",
                        worker_index,
                        num_threads: ThreadPool::current_num_threads(pool_ref),
                        pending_status_available:
                            ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some(),
                    },
                    even_weighted + odd_weighted,
                )
            },
        );

    assert_eq!(sum_report.label, "seed-sum");
    assert_eq!(weighted_report.label, "weighted-sum");

    for report in [&sum_report, &weighted_report] {
        assert!(report.worker_index < thread_count);
        assert_eq!(report.num_threads, thread_count);
        assert!(
            report.pending_status_available,
            "ThreadPool::join branches should observe worker-local pending-task status"
        );
    }

    assert_eq!(observed_seed_sum, expected_seed_sum);
    assert_eq!(observed_weighted_sum, expected_weighted_sum);

    let scoped_records = Mutex::new(Vec::<ScopedJoinRecord>::new());

    let scope_return = ThreadPool::scope(pool_ref, |scope| {
        for seed_record in seeds.iter().cloned() {
            let scoped_records = &scoped_records;

            Scope::spawn(scope, move |_| {
                let executing_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("scoped follow-up should run inside the custom pool");
                assert!(executing_index < thread_count);
                assert_eq!(ThreadPool::current_num_threads(pool_ref), thread_count);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let pending_status_available =
                    ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();

                let (left_value, right_value) = ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index + observed_seed_sum,
                    move || {
                        observed_weighted_sum
                            + executing_index
                            + sum_report.worker_index
                            + weighted_report.worker_index
                    },
                );

                scoped_records
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(ScopedJoinRecord {
                        origin_index,
                        seed,
                        executing_index,
                        left_value,
                        right_value,
                        combined: left_value + right_value,
                        pending_status_available,
                    });
            });
        }

        observed_seed_sum + observed_weighted_sum
    });

    assert_eq!(scope_return, expected_seed_sum + expected_weighted_sum);

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

    let scoped_by_origin: BTreeMap<usize, ScopedJoinRecord> = scoped_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

    for record in &scoped_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert!(record.pending_status_available);

        assert_eq!(
            record.left_value,
            record.seed + record.origin_index + expected_seed_sum
        );
        assert_eq!(
            record.right_value,
            expected_weighted_sum
                + record.executing_index
                + sum_report.worker_index
                + weighted_report.worker_index
        );
        assert_eq!(record.combined, record.left_value + record.right_value);
    }

    let (observed_scoped_sum, recomputed_scoped_sum) = ThreadPool::join(
        pool_ref,
        || scoped_records.iter().map(|record| record.combined).sum::<usize>(),
        || {
            scoped_by_origin
                .values()
                .map(|record| record.left_value + record.right_value)
                .sum::<usize>()
        },
    );

    assert_eq!(observed_scoped_sum, recomputed_scoped_sum);

    let left_started = AtomicUsize::new(0);
    let right_started = AtomicUsize::new(0);
    let left_value = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: (usize, usize) = ThreadPool::join(
            pool_ref,
            || {
                left_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("non-panicking branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                let (nineteen, twenty_three) =
                    ThreadPool::join(pool_ref, || 19usize, || 23usize);
                let value = nineteen + twenty_three + worker_index;
                left_value.store(value, Ordering::SeqCst);

                value
            },
            || -> usize {
                right_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                panic!("intentional panic from ThreadPool::join branch on worker {worker_index}");
            },
        );
    }));

    let payload = panic_result.expect_err("ThreadPool::join branch panic should propagate");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional panic from ThreadPool::join branch"),
        "unexpected panic payload: {panic_message:?}"
    );
    assert_eq!(left_started.load(Ordering::SeqCst), 1);
    assert_eq!(right_started.load(Ordering::SeqCst), 1);
    assert!(
        left_value.load(Ordering::SeqCst) >= 42,
        "the non-panicking join branch should complete before the panic is propagated"
    );
    assert_eq!(
        ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the external caller should still not be a custom-pool worker"
    );

    let mut recovery_seeds = ThreadPool::broadcast(pool_ref, |context| {
        let index = BroadcastContext::index(&context);
        let num_threads = BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, (index + 3) * (num_threads + 11))
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

    let recovery_values: Vec<_> = recovery_seeds.iter().map(|(_, seed)| *seed).collect();
    let expected_recovery_sum: usize = recovery_values.iter().sum();
    let expected_recovery_min = recovery_values
        .iter()
        .copied()
        .min()
        .expect("recovery broadcast should produce seeds");
    let expected_recovery_max = recovery_values
        .iter()
        .copied()
        .max()
        .expect("recovery broadcast should produce seeds");

    let ((sum_worker, recovered_sum), (range_worker, (recovered_min, recovered_max))) =
        ThreadPool::join(
            pool_ref,
            || {
                let worker_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery sum branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                let split = recovery_values.len() / 2;
                let (left, right) = recovery_values.split_at(split);
                let (left_sum, right_sum) = ThreadPool::join(
                    pool_ref,
                    || left.iter().copied().sum::<usize>(),
                    || right.iter().copied().sum::<usize>(),
                );

                (worker_index, left_sum + right_sum)
            },
            || {
                let worker_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery range branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                let (minimum, maximum) = ThreadPool::join(
                    pool_ref,
                    || recovery_values.iter().copied().min(),
                    || recovery_values.iter().copied().max(),
                );

                (
                    worker_index,
                    (
                        minimum.expect("minimum should be present"),
                        maximum.expect("maximum should be present"),
                    ),
                )
            },
        );

    assert!(sum_worker < thread_count);
    assert!(range_worker < thread_count);
    assert_eq!(recovered_sum, expected_recovery_sum);
    assert_eq!(recovered_min, expected_recovery_min);
    assert_eq!(recovered_max, expected_recovery_max);

    let (combined, recomputed) = ThreadPool::join(
        pool_ref,
        || recovered_sum + recovered_min + recovered_max + left_value.load(Ordering::SeqCst),
        || {
            expected_recovery_sum
                + expected_recovery_min
                + expected_recovery_max
                + left_value.load(Ordering::SeqCst)
        },
    );

    assert_eq!(combined, recomputed);
}