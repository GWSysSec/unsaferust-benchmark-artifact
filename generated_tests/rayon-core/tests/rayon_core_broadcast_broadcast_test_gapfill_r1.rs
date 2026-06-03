use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ModuleBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: Option<String>,
    seed: usize,
    joined_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BranchReport {
    branch: &'static str,
    worker_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedDerivedRecord {
    origin_index: usize,
    seed: usize,
    broadcast_joined_value: usize,
    executing_index: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfirmationRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryFifoRecord {
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

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn module_broadcast_called_from_pool_worker_uses_current_pool_and_feeds_scoped_work() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("module-broadcast-current-pool-worker-{index}"))
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

    let ((mut records, caller_report), sibling_report) = rayon_core::ThreadPool::join(
        pool_ref,
        || {
            let calling_worker = rayon_core::ThreadPool::current_thread_index(pool_ref)
                .expect("broadcast caller branch should run inside the custom pool");
            assert!(calling_worker < thread_count);

            let mut records = rayon_core::broadcast(|context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(
                    num_threads, thread_count,
                    "rayon_core::broadcast should use the current custom pool"
                );
                assert!(index < num_threads);
                assert_eq!(rayon_core::current_num_threads(), thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));

                let expected_name = format!("module-broadcast-current-pool-worker-{index}");
                let worker_name = std::thread::current().name().map(str::to_owned);
                assert_eq!(worker_name.as_deref(), Some(expected_name.as_str()));

                let seed = (index + 1) * (num_threads + 83);
                let (left, right) =
                    rayon_core::join(move || seed + index, move || num_threads * 10);

                ModuleBroadcastRecord {
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

            records.sort_by_key(|record| record.index);

            let value = records
                .iter()
                .map(|record| record.joined_value)
                .sum::<usize>()
                + calling_worker;

            (
                records,
                BranchReport {
                    branch: "module-broadcast-caller",
                    worker_index: calling_worker,
                    num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                    value,
                    pending_status_available:
                        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                            .is_some(),
                },
            )
        },
        || {
            let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                .expect("sibling join branch should run inside the custom pool");
            assert!(worker_index < thread_count);

            let (left, right) =
                rayon_core::join(move || worker_index + 1, move || thread_count * 1000);

            BranchReport {
                branch: "sibling",
                worker_index,
                num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                value: left + right,
                pending_status_available:
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some(),
            }
        },
    );

    records.sort_by_key(|record| record.index);

    assert_eq!(caller_report.branch, "module-broadcast-caller");
    assert_eq!(sibling_report.branch, "sibling");

    for report in [&caller_report, &sibling_report] {
        assert!(report.worker_index < thread_count);
        assert_eq!(report.num_threads, thread_count);
        assert!(
            report.pending_status_available,
            "ThreadPool::join branch should observe worker-local pending-task status"
        );
    }

    assert_eq!(
        sibling_report.value,
        sibling_report.worker_index + 1 + thread_count * 1000
    );

    assert_eq!(records.len(), thread_count);
    assert_eq!(
        records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (expected_index, record) in records.iter().enumerate() {
        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(expected_index));

        let expected_name = format!("module-broadcast-current-pool-worker-{expected_index}");
        assert_eq!(record.worker_name.as_deref(), Some(expected_name.as_str()));

        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 83));
        assert_eq!(
            record.joined_value,
            record.seed + record.index + thread_count * 10
        );
        assert!(
            record.pending_status_available,
            "rayon_core::broadcast worker should observe pending-task status"
        );
    }

    let expected_joined_sum: usize = records.iter().map(|record| record.joined_value).sum();
    assert_eq!(
        caller_report.value,
        expected_joined_sum + caller_report.worker_index
    );

    let record_by_index: BTreeMap<usize, ModuleBroadcastRecord> = records
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();
    assert_eq!(record_by_index.len(), thread_count);

    let scoped_records = Mutex::new(Vec::<ScopedDerivedRecord>::new());
    let sibling_value = sibling_report.value;

    let scope_return = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        for record in records.iter().cloned() {
            let scoped_records_ref = &scoped_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("scoped follow-up work should run inside the custom pool");

                assert!(executing_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                let origin_index = record.index;
                let seed = record.seed;
                let broadcast_joined_value = record.joined_value;

                let (left, right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || broadcast_joined_value + seed,
                    move || origin_index + executing_index + sibling_value,
                );

                scoped_records_ref
                    .lock()
                    .expect("scoped records mutex should not be poisoned")
                    .push(ScopedDerivedRecord {
                        origin_index,
                        seed,
                        broadcast_joined_value,
                        executing_index,
                        value: left + right,
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    });
            });
        }

        expected_joined_sum + caller_report.value + sibling_value
    });

    assert_eq!(
        scope_return,
        expected_joined_sum + caller_report.value + sibling_value
    );

    let mut scoped_records = scoped_records
        .into_inner()
        .expect("scoped records mutex should not be poisoned");
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

        let original = record_by_index
            .get(&record.origin_index)
            .expect("scoped record should correspond to a module broadcast record");

        assert_eq!(record.seed, original.seed);
        assert_eq!(record.broadcast_joined_value, original.joined_value);
        assert_eq!(
            record.value,
            original.joined_value
                + original.seed
                + record.origin_index
                + record.executing_index
                + sibling_value
        );
        assert!(
            record.pending_status_available,
            "scoped follow-up work should observe pending-task status"
        );
    }

    let scoped_by_origin: BTreeMap<usize, ScopedDerivedRecord> = scoped_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(scoped_by_origin.len(), thread_count);

    let joined_by_index: BTreeMap<usize, usize> = records
        .iter()
        .map(|record| (record.index, record.joined_value))
        .collect();

    let (mut confirmations, confirmation_thread_count) = rayon_core::ThreadPool::join(
        pool_ref,
        || {
            rayon_core::broadcast(|context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(num_threads, thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));

                let scoped = scoped_by_origin
                    .get(&index)
                    .expect("confirmation broadcast should find scoped output by worker index");
                let scoped_value = scoped.value;
                let original_joined = joined_by_index[&index];

                let (left, right) =
                    rayon_core::join(move || scoped_value, move || original_joined);

                ConfirmationRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    total: left + right,
                }
            })
        },
        || rayon_core::ThreadPool::current_num_threads(pool_ref),
    );

    assert_eq!(confirmation_thread_count, thread_count);
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

        let scoped = scoped_by_origin
            .get(&record.index)
            .expect("confirmation should correspond to scoped output");

        assert_eq!(record.total, scoped.value + joined_by_index[&record.index]);
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn module_broadcast_panic_propagates_and_pool_recovers_with_later_module_broadcast() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("module-broadcast-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let pool_ref = &pool;

    let panic_hits = AtomicUsize::new(0);
    let non_panic_hits = AtomicUsize::new(0);
    let non_panic_mask = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: (Vec<usize>, usize) = rayon_core::ThreadPool::join(
            pool_ref,
            || {
                rayon_core::broadcast(|context| {
                    let index = rayon_core::BroadcastContext::index(&context);
                    let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                    assert_eq!(num_threads, thread_count);
                    assert_eq!(rayon_core::current_thread_index(), Some(index));
                    assert_eq!(rayon_core::current_num_threads(), thread_count);

                    if index == 1 {
                        panic_hits.fetch_add(1, Ordering::SeqCst);
                        panic!("intentional rayon_core::broadcast panic from worker 1");
                    }

                    non_panic_hits.fetch_add(1, Ordering::SeqCst);
                    non_panic_mask.fetch_or(1usize << index, Ordering::SeqCst);

                    index + num_threads
                })
            },
            || {
                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("sibling branch should run inside the custom pool");
                assert!(worker_index < thread_count);

                rayon_core::ThreadPool::current_num_threads(pool_ref)
            },
        );
    }));

    let payload = panic_result
        .expect_err("a panic inside rayon_core::broadcast should propagate");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("rayon_core::broadcast panic from worker 1"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_hits.load(Ordering::SeqCst), 1);

    let non_panic_count = non_panic_hits.load(Ordering::SeqCst);
    assert!(non_panic_count <= thread_count - 1);

    let mask = non_panic_mask.load(Ordering::SeqCst);
    assert_eq!(
        mask & (1usize << 1),
        0,
        "the panicking worker should not be recorded as a non-panicking worker"
    );
    assert_eq!(mask & !((1usize << thread_count) - 1), 0);
    assert!(
        mask.count_ones() as usize <= non_panic_count,
        "recorded non-panicking indices should be a subset of non-panicking executions"
    );

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the external caller should still not be a custom-pool worker"
    );

    let (mut recovery_records, observed_pool_threads) = rayon_core::ThreadPool::join(
        pool_ref,
        || {
            rayon_core::broadcast(|context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(num_threads, thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                RecoveryBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed: (index + 3) * (num_threads + 29),
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                }
            })
        },
        || rayon_core::ThreadPool::current_num_threads(pool_ref),
    );

    assert_eq!(observed_pool_threads, thread_count);

    recovery_records.sort_by_key(|record| record.index);

    assert_eq!(recovery_records.len(), thread_count);
    assert_eq!(
        recovery_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &recovery_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, (record.index + 3) * (thread_count + 29));
        assert!(
            record.pending_status_available,
            "recovery module broadcast should still run on Rayon workers"
        );
    }

    let seed_by_index: BTreeMap<usize, usize> = recovery_records
        .iter()
        .map(|record| (record.index, record.seed))
        .collect();
    assert_eq!(seed_by_index.len(), thread_count);

    let expected_seed_sum: usize = recovery_records.iter().map(|record| record.seed).sum();
    let fifo_records = Mutex::new(Vec::<RecoveryFifoRecord>::new());

    let fifo_return = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
        for record in recovery_records.iter().cloned() {
            let fifo_records_ref = &fifo_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery FIFO work should run inside the custom pool");

                assert!(executing_index < record.num_threads);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    record.num_threads
                );

                let origin_index = record.index;
                let seed = record.seed;
                let num_threads = record.num_threads;

                let (left, right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || num_threads + executing_index,
                );

                fifo_records_ref
                    .lock()
                    .expect("FIFO records mutex should not be poisoned")
                    .push(RecoveryFifoRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        value: left + right,
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    });
            });
        }

        expected_seed_sum + non_panic_count
    });

    assert_eq!(fifo_return, expected_seed_sum + non_panic_count);

    let mut fifo_records = fifo_records
        .into_inner()
        .expect("FIFO records mutex should not be poisoned");
    fifo_records.sort_by_key(|record| record.origin_index);

    assert_eq!(fifo_records.len(), thread_count);
    assert_eq!(
        fifo_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &fifo_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(seed_by_index.get(&record.origin_index), Some(&record.seed));
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "FIFO recovery work should observe worker-local pending-task status"
        );
    }

    let (observed_fifo_sum, recomputed_fifo_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || fifo_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            fifo_records
                .iter()
                .map(|record| {
                    seed_by_index[&record.origin_index]
                        + record.origin_index
                        + thread_count
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_fifo_sum, recomputed_fifo_sum);

    let (mut final_check, final_thread_count) = rayon_core::ThreadPool::join(
        pool_ref,
        || {
            rayon_core::broadcast(|context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(num_threads, thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));

                (
                    index,
                    num_threads,
                    rayon_core::current_thread_index(),
                    index * 10 + num_threads,
                )
            })
        },
        || rayon_core::ThreadPool::current_num_threads(pool_ref),
    );

    assert_eq!(final_thread_count, thread_count);
    final_check.sort_by_key(|record| record.0);

    assert_eq!(
        final_check
            .iter()
            .map(|(index, _, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, num_threads, current_index, value) in final_check {
        assert_eq!(num_threads, thread_count);
        assert_eq!(current_index, Some(index));
        assert_eq!(value, index * 10 + thread_count);
    }
}