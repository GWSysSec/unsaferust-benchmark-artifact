use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChunkRecord {
    chunk_index: usize,
    start_index: usize,
    len: usize,
    first: i64,
    last: i64,
    local_sum: i64,
    local_weighted_sum: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PoolSeed {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PoolScopeSummary {
    body_index: usize,
    body_threads: usize,
    scheduled_jobs: usize,
    seed_sum: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PoolScopedJoinRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    left_worker: usize,
    right_worker: usize,
    left_value: usize,
    right_value: usize,
    combined: usize,
    pending_status_available: bool,
}

fn partition<T: PartialOrd + Send>(values: &mut [T]) -> usize {
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

fn parallel_quick_sort<T: PartialOrd + Send>(values: &mut [T]) {
    if values.len() <= 1 {
        return;
    }

    let pivot = partition(values);
    let (left, pivot_and_right) = values.split_at_mut(pivot);
    let (_, right) = pivot_and_right.split_at_mut(1);

    rayon_core::join(|| parallel_quick_sort(left), || parallel_quick_sort(right));
}

fn parallel_sum(values: &[i64]) -> i64 {
    if values.len() <= 4 {
        values.iter().copied().sum()
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let (left_sum, right_sum) = rayon_core::join(|| parallel_sum(left), || parallel_sum(right));
        left_sum + right_sum
    }
}

fn parallel_weighted_sum(values: &[i64], base_index: usize) -> i64 {
    if values.len() <= 4 {
        values
            .iter()
            .enumerate()
            .map(|(offset, value)| (base_index + offset + 1) as i64 * *value)
            .sum()
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let (left_sum, right_sum) = rayon_core::join(
            || parallel_weighted_sum(left, base_index),
            || parallel_weighted_sum(right, base_index + mid),
        );
        left_sum + right_sum
    }
}

fn parallel_is_sorted(values: &[i64]) -> bool {
    if values.len() <= 8 {
        values.windows(2).all(|pair| pair[0] <= pair[1])
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let boundary_ok = left.last().expect("left half should be nonempty")
            <= right.first().expect("right half should be nonempty");

        let (left_sorted, right_sorted) =
            rayon_core::join(|| parallel_is_sorted(left), || parallel_is_sorted(right));

        left_sorted && right_sorted && boundary_ok
    }
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

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_join_recursively_sorts_borrowed_data_and_feeds_scoped_chunk_validation() {
    let mut values = vec![42, -7, 13, 13, 0, 5, -21, 8, 34, 1, -3, 55, 2, 2, 89, -34, 21];
    let original_values = values.clone();

    let mut expected_sorted = original_values.clone();
    expected_sorted.sort();

    let ((), (original_sum, original_weighted_sum)) = rayon_core::join(
        || parallel_quick_sort(&mut values),
        || {
            (
                parallel_sum(&original_values),
                parallel_weighted_sum(&original_values, 0),
            )
        },
    );

    assert_eq!(values, expected_sorted);
    assert_eq!(original_sum, expected_sorted.iter().copied().sum::<i64>());

    let (is_sorted, (sorted_sum, sorted_weighted_sum)) = rayon_core::join(
        || parallel_is_sorted(&values),
        || {
            rayon_core::join(
                || parallel_sum(&values),
                || parallel_weighted_sum(&values, 0),
            )
        },
    );

    assert!(is_sorted);
    assert_eq!(sorted_sum, original_sum);
    assert_eq!(sorted_weighted_sum, parallel_weighted_sum(&expected_sorted, 0));
    assert_ne!(
        original_weighted_sum, sorted_weighted_sum,
        "test data should be meaningfully reordered by the join-driven quicksort"
    );

    let chunk_size = (sorted_weighted_sum.rem_euclid(7) as usize + 3).min(values.len());
    assert!(chunk_size > 0);

    let chunk_records = Mutex::new(Vec::<ChunkRecord>::new());

    let scope_return = rayon_core::scope(|scope| {
        for (chunk_index, chunk) in values.chunks(chunk_size).enumerate() {
            let start_index = chunk_index * chunk_size;
            let chunk_records_ref = &chunk_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let ((first, last), (local_sum, local_weighted_sum)) = rayon_core::join(
                    || {
                        (
                            *chunk.first().expect("chunk should be nonempty"),
                            *chunk.last().expect("chunk should be nonempty"),
                        )
                    },
                    || {
                        rayon_core::join(
                            || parallel_sum(chunk),
                            || parallel_weighted_sum(chunk, start_index),
                        )
                    },
                );

                assert!(first <= last);

                chunk_records_ref
                    .lock()
                    .expect("chunk record mutex should not be poisoned")
                    .push(ChunkRecord {
                        chunk_index,
                        start_index,
                        len: chunk.len(),
                        first,
                        last,
                        local_sum,
                        local_weighted_sum,
                    });
            });
        }

        sorted_sum + sorted_weighted_sum
    });

    assert_eq!(scope_return, sorted_sum + sorted_weighted_sum);

    let mut chunk_records = chunk_records
        .into_inner()
        .expect("chunk record mutex should not be poisoned");
    chunk_records.sort_by_key(|record| record.chunk_index);

    assert_eq!(chunk_records.len(), values.chunks(chunk_size).count());

    let mut covered = 0usize;
    for record in &chunk_records {
        assert_eq!(record.start_index, covered);
        assert!(record.len > 0);
        assert!(record.first <= record.last);

        let chunk = &values[record.start_index..record.start_index + record.len];
        assert_eq!(record.first, *chunk.first().expect("record chunk should exist"));
        assert_eq!(record.last, *chunk.last().expect("record chunk should exist"));
        assert_eq!(record.local_sum, chunk.iter().copied().sum::<i64>());
        assert_eq!(
            record.local_weighted_sum,
            parallel_weighted_sum(chunk, record.start_index)
        );

        covered += record.len;
    }

    assert_eq!(covered, values.len());
    assert_eq!(
        chunk_records
            .iter()
            .map(|record| record.local_sum)
            .sum::<i64>(),
        sorted_sum
    );
    assert_eq!(
        chunk_records
            .iter()
            .map(|record| record.local_weighted_sum)
            .sum::<i64>(),
        sorted_weighted_sum
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_join_called_from_custom_pool_worker_uses_current_pool_and_drives_followup_reduction() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("free-join-current-pool-worker-{index}"))
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
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        PoolSeed {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 2) * (num_threads + 19),
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

    for record in &seeds {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, (record.index + 2) * (thread_count + 19));
    }

    let expected_seed_sum: usize = seeds.iter().map(|record| record.seed).sum();
    let scoped_records = Mutex::new(Vec::<PoolScopedJoinRecord>::new());

    let summary = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        let body_index = rayon_core::current_thread_index()
            .expect("ThreadPool::scope body should run inside the custom pool");

        assert!(body_index < thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        for seed_record in seeds.iter().cloned() {
            let scoped_records_ref = &scoped_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped work should run inside the custom pool");

                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let pending_status_available =
                    rayon_core::current_thread_has_pending_tasks().is_some();

                let ((left_value, left_worker), (right_value, right_worker)) =
                    rayon_core::join(
                        move || {
                            let worker = rayon_core::current_thread_index()
                                .expect("left free-join branch should run on a pool worker");
                            assert!(worker < thread_count);
                            assert_eq!(rayon_core::current_num_threads(), thread_count);

                            (seed + origin_index + worker, worker)
                        },
                        move || {
                            let worker = rayon_core::current_thread_index()
                                .expect("right free-join branch should run on a pool worker");
                            assert!(worker < thread_count);
                            assert_eq!(rayon_core::current_num_threads(), thread_count);

                            (seed * 2 + executing_index + worker + thread_count, worker)
                        },
                    );

                scoped_records_ref
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(PoolScopedJoinRecord {
                        origin_index,
                        seed,
                        executing_index,
                        left_worker,
                        right_worker,
                        left_value,
                        right_value,
                        combined: left_value + right_value,
                        pending_status_available,
                    });
            });
        }

        PoolScopeSummary {
            body_index,
            body_threads: rayon_core::current_num_threads(),
            scheduled_jobs: seeds.len(),
            seed_sum: expected_seed_sum,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    assert!(summary.body_index < thread_count);
    assert_eq!(summary.body_threads, thread_count);
    assert_eq!(summary.scheduled_jobs, thread_count);
    assert_eq!(summary.seed_sum, expected_seed_sum);
    assert!(summary.pending_status_available);

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
        expected_indices
    );

    for record in &scoped_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert!(record.left_worker < thread_count);
        assert!(record.right_worker < thread_count);
        assert!(record.pending_status_available);

        assert_eq!(record.seed, seeds[record.origin_index].seed);
        assert_eq!(
            record.left_value,
            record.seed + record.origin_index + record.left_worker
        );
        assert_eq!(
            record.right_value,
            record.seed * 2 + record.executing_index + record.right_worker + thread_count
        );
        assert_eq!(record.combined, record.left_value + record.right_value);
    }

    let by_origin: BTreeMap<usize, PoolScopedJoinRecord> = scoped_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

    assert_eq!(by_origin.len(), thread_count);

    let (observed_total, recomputed_total) = rayon_core::join(
        || scoped_records.iter().map(|record| record.combined).sum::<usize>(),
        || {
            by_origin
                .values()
                .map(|record| {
                    record.seed
                        + record.origin_index
                        + record.left_worker
                        + record.seed * 2
                        + record.executing_index
                        + record.right_worker
                        + thread_count
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_total, recomputed_total);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_join_propagates_branch_panic_and_later_nested_joins_recover() {
    let left_started = AtomicUsize::new(0);
    let right_started = AtomicUsize::new(0);
    let left_value = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: (usize, usize) = rayon_core::join(
            || {
                left_started.fetch_add(1, Ordering::SeqCst);

                let (six, seven) = rayon_core::join(|| 6usize, || 7usize);
                let value = six * seven;

                left_value.store(value, Ordering::SeqCst);
                value
            },
            || -> usize {
                right_started.fetch_add(1, Ordering::SeqCst);
                panic!("intentional panic from rayon_core::join right branch");
            },
        );
    }));

    let payload = panic_result.expect_err("panic in a join branch should propagate");
    let message = panic_payload_to_string(&*payload);

    assert!(
        message.contains("intentional panic from rayon_core::join right branch"),
        "unexpected panic payload: {message:?}"
    );
    assert_eq!(left_started.load(Ordering::SeqCst), 1);
    assert_eq!(right_started.load(Ordering::SeqCst), 1);
    assert_eq!(left_value.load(Ordering::SeqCst), 42);

    let recovery_values: Vec<i64> = (1..=24).map(|value| value as i64).collect();

    let ((sum, weighted_sum), (even_sum, odd_sum)) = rayon_core::join(
        || {
            rayon_core::join(
                || parallel_sum(&recovery_values),
                || parallel_weighted_sum(&recovery_values, 0),
            )
        },
        || {
            rayon_core::join(
                || {
                    recovery_values
                        .iter()
                        .copied()
                        .filter(|value| value % 2 == 0)
                        .sum::<i64>()
                },
                || {
                    recovery_values
                        .iter()
                        .copied()
                        .filter(|value| value % 2 != 0)
                        .sum::<i64>()
                },
            )
        },
    );

    assert_eq!(sum, recovery_values.iter().copied().sum::<i64>());
    assert_eq!(weighted_sum, parallel_weighted_sum(&recovery_values, 0));
    assert_eq!(even_sum + odd_sum, sum);
    assert_eq!(even_sum, 156);
    assert_eq!(odd_sum, 144);

    let (combined, recomputed) = rayon_core::join(
        || sum + weighted_sum + even_sum + odd_sum,
        || {
            recovery_values
                .iter()
                .enumerate()
                .map(|(index, value)| (index + 1) as i64 * *value)
                .sum::<i64>()
                + recovery_values.iter().copied().sum::<i64>() * 2
        },
    );

    assert_eq!(combined, recomputed);
}