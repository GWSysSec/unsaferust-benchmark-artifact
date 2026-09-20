use rayon_core::{BroadcastContext, Scope, ScopeFifo, ThreadPool, ThreadPoolBuilder};

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlobalBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    joined_checksum: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedRecord {
    origin_index: usize,
    executing_index: usize,
    num_threads: usize,
    seed: usize,
    broadcast_checksum: usize,
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
struct CustomPoolBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    name: Option<String>,
    seed: usize,
    nested_value: usize,
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
struct CustomPoolFollowupRecord {
    origin_index: usize,
    seed: usize,
    nested_value: usize,
    executing_index: usize,
    value: usize,
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
struct RecoveryScopedRecord {
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
fn free_broadcast_from_external_thread_feeds_scope_and_second_broadcast() {
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "the integration-test thread should not start as a Rayon worker"
    );

    let mut observations = rayon_core::broadcast(|context| {
        let index = BroadcastContext::index(&context);
        let num_threads = BroadcastContext::num_threads(&context);

        assert!(index < num_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), num_threads);

        let seed = (index + 1) * (num_threads + 31);
        let (left, right) =
            rayon_core::join(move || seed + index, move || num_threads * 10);

        GlobalBroadcastRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed,
            joined_checksum: left + right,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    observations.sort_by_key(|record| record.index);

    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());
    assert_eq!(observations.len(), global_threads);
    assert_eq!(
        observations
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for (expected_index, record) in observations.iter().enumerate() {
        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(expected_index));
        assert_eq!(record.seed, (expected_index + 1) * (global_threads + 31));
        assert_eq!(
            record.joined_checksum,
            record.seed + record.index + global_threads * 10
        );
        assert!(
            record.pending_status_available,
            "rayon_core::broadcast workers should be able to query pending-task status"
        );
    }

    let checksum_by_index: BTreeMap<usize, usize> = observations
        .iter()
        .map(|record| (record.index, record.joined_checksum))
        .collect();
    assert_eq!(checksum_by_index.len(), global_threads);

    let expected_checksum_sum: usize = observations
        .iter()
        .map(|record| record.joined_checksum)
        .sum();

    let scoped_records = Mutex::new(Vec::<ScopedRecord>::new());

    let scope_return = rayon_core::scope(|scope| {
        for record in observations.iter().cloned() {
            let scoped_records = &scoped_records;

            Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped work driven by broadcast output should run on a Rayon worker");

                assert!(executing_index < record.num_threads);
                assert_eq!(rayon_core::current_num_threads(), record.num_threads);

                let origin_index = record.index;
                let seed = record.seed;
                let broadcast_checksum = record.joined_checksum;
                let num_threads = record.num_threads;

                let (left, right) = rayon_core::join(
                    move || broadcast_checksum + seed,
                    move || origin_index + executing_index + num_threads,
                );

                scoped_records
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(ScopedRecord {
                        origin_index,
                        executing_index,
                        num_threads,
                        seed,
                        broadcast_checksum,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        expected_checksum_sum
    });

    assert_eq!(scope_return, expected_checksum_sum);

    let mut scoped_records = scoped_records
        .into_inner()
        .expect("scoped record mutex should not be poisoned");
    scoped_records.sort_by_key(|record| record.origin_index);

    assert_eq!(scoped_records.len(), global_threads);
    assert_eq!(
        scoped_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for record in &scoped_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(
            record.broadcast_checksum,
            checksum_by_index[&record.origin_index]
        );
        assert_eq!(
            record.value,
            record.broadcast_checksum
                + record.seed
                + record.origin_index
                + record.executing_index
                + global_threads
        );
        assert!(record.pending_status_available);
    }

    let scoped_by_origin: BTreeMap<usize, ScopedRecord> = scoped_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(scoped_by_origin.len(), global_threads);

    let mut confirmation = rayon_core::broadcast(|context| {
        let index = BroadcastContext::index(&context);
        let num_threads = BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let scoped = scoped_by_origin
            .get(&index)
            .expect("second broadcast should find the scoped output for its worker index");
        let original_checksum = checksum_by_index[&index];
        let scoped_value = scoped.value;

        let (left, right) = rayon_core::join(move || scoped_value, move || original_checksum);

        ConfirmationRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            total: left + right,
        }
    });

    confirmation.sort_by_key(|record| record.index);

    assert_eq!(confirmation.len(), global_threads);
    for record in &confirmation {
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));

        let scoped = scoped_by_origin
            .get(&record.index)
            .expect("confirmation should correspond to a scoped record");
        assert_eq!(
            record.total,
            scoped.value + checksum_by_index[&record.index]
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_broadcast_uses_current_custom_pool_when_called_from_pool_worker() {
    let thread_count = 3usize;

    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("free-broadcast-current-pool-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let pool_ref = &pool;

    assert_eq!(ThreadPool::current_num_threads(pool_ref), thread_count);
    assert_eq!(ThreadPool::current_thread_index(pool_ref), None);
    assert_eq!(ThreadPool::current_thread_has_pending_tasks(pool_ref), None);

    let ((mut records, broadcast_branch), sibling_branch) = ThreadPool::join(
        pool_ref,
        || {
            let calling_worker = ThreadPool::current_thread_index(pool_ref)
                .expect("ThreadPool::join branch should run inside the custom pool");
            assert!(calling_worker < thread_count);

            let mut records = rayon_core::broadcast(|context| {
                let index = BroadcastContext::index(&context);
                let num_threads = BroadcastContext::num_threads(&context);

                assert_eq!(
                    num_threads, thread_count,
                    "free broadcast called from a custom pool worker should use that pool"
                );
                assert_eq!(rayon_core::current_num_threads(), thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));

                let expected_name = format!("free-broadcast-current-pool-worker-{index}");
                let name = std::thread::current().name().map(str::to_owned);
                assert_eq!(name.as_deref(), Some(expected_name.as_str()));

                let seed = (index + 2) * (num_threads + 41);
                let (left, right) =
                    rayon_core::join(move || seed * 2, move || index + num_threads);

                CustomPoolBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    name,
                    seed,
                    nested_value: left + right,
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                }
            });

            records.sort_by_key(|record| record.index);
            let value = records.iter().map(|record| record.nested_value).sum();

            (
                records,
                BranchReport {
                    branch: "broadcast-caller",
                    worker_index: calling_worker,
                    num_threads: ThreadPool::current_num_threads(pool_ref),
                    value,
                    pending_status_available:
                        ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some(),
                },
            )
        },
        || {
            let worker_index = ThreadPool::current_thread_index(pool_ref)
                .expect("sibling ThreadPool::join branch should run inside the custom pool");
            assert!(worker_index < thread_count);

            let (left, right) = rayon_core::join(
                move || worker_index + 1,
                move || ThreadPool::current_num_threads(pool_ref) * 100,
            );

            BranchReport {
                branch: "sibling",
                worker_index,
                num_threads: ThreadPool::current_num_threads(pool_ref),
                value: left + right,
                pending_status_available:
                    ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some(),
            }
        },
    );

    assert_eq!(broadcast_branch.branch, "broadcast-caller");
    assert_eq!(sibling_branch.branch, "sibling");

    for branch in [&broadcast_branch, &sibling_branch] {
        assert!(branch.worker_index < thread_count);
        assert_eq!(branch.num_threads, thread_count);
        assert!(
            branch.pending_status_available,
            "ThreadPool::join branches should observe worker-local pending-task status"
        );
    }

    assert_eq!(records.len(), thread_count);
    assert_eq!(
        records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(thread_count)
    );

    let expected_nested_sum: usize = records
        .iter()
        .map(|record| record.nested_value)
        .sum();
    assert_eq!(broadcast_branch.value, expected_nested_sum);
    assert_eq!(
        sibling_branch.value,
        sibling_branch.worker_index + 1 + thread_count * 100
    );

    for record in &records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));

        let expected_name = format!("free-broadcast-current-pool-worker-{}", record.index);
        assert_eq!(record.name.as_deref(), Some(expected_name.as_str()));

        assert_eq!(record.seed, (record.index + 2) * (thread_count + 41));
        assert_eq!(
            record.nested_value,
            record.seed * 2 + record.index + thread_count
        );
        assert!(record.pending_status_available);
    }

    let followup_records = Mutex::new(Vec::<CustomPoolFollowupRecord>::new());
    let initiating_worker = broadcast_branch.worker_index;
    let sibling_value = sibling_branch.value;

    let fifo_return = ThreadPool::scope_fifo(pool_ref, |scope| {
        let body_index = ThreadPool::current_thread_index(pool_ref)
            .expect("ThreadPool::scope_fifo body should run inside the custom pool");
        assert!(body_index < thread_count);

        for record in records.iter().cloned() {
            let followup_records = &followup_records;

            ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("FIFO follow-up work should run inside the custom pool");

                assert!(executing_index < thread_count);
                assert_eq!(ThreadPool::current_num_threads(pool_ref), thread_count);

                let origin_index = record.index;
                let seed = record.seed;
                let nested_value = record.nested_value;

                let (left, right) = rayon_core::join(
                    move || nested_value + seed,
                    move || origin_index + executing_index + initiating_worker + sibling_value,
                );

                followup_records
                    .lock()
                    .expect("follow-up record mutex should not be poisoned")
                    .push(CustomPoolFollowupRecord {
                        origin_index,
                        seed,
                        nested_value,
                        executing_index,
                        value: left + right,
                    });
            });
        }

        expected_nested_sum + sibling_value
    });

    assert_eq!(fifo_return, expected_nested_sum + sibling_value);

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
        expected_worker_indices(thread_count)
    );

    let record_by_index: BTreeMap<usize, CustomPoolBroadcastRecord> = records
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();

    for followup in &followup_records {
        assert!(followup.origin_index < thread_count);
        assert!(followup.executing_index < thread_count);

        let broadcast_record = record_by_index
            .get(&followup.origin_index)
            .expect("FIFO follow-up should correspond to a broadcast record");

        assert_eq!(followup.seed, broadcast_record.seed);
        assert_eq!(followup.nested_value, broadcast_record.nested_value);
        assert_eq!(
            followup.value,
            broadcast_record.nested_value
                + broadcast_record.seed
                + followup.origin_index
                + followup.executing_index
                + initiating_worker
                + sibling_value
        );
    }

    let (observed_sum, recomputed_sum) = ThreadPool::join(
        pool_ref,
        || followup_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            followup_records
                .iter()
                .map(|record| {
                    let broadcast_record = record_by_index
                        .get(&record.origin_index)
                        .expect("broadcast record should exist during recomputation");

                    broadcast_record.nested_value
                        + broadcast_record.seed
                        + record.origin_index
                        + record.executing_index
                        + initiating_worker
                        + sibling_value
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_broadcast_panic_propagates_and_custom_pool_recovers_for_later_work() {
    let thread_count = 3usize;

    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("free-broadcast-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let pool_ref = &pool;
    let panic_hits = AtomicUsize::new(0);
    let non_panic_hits = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: ((), usize) = ThreadPool::join(
            pool_ref,
            || {
                let _: Vec<usize> = rayon_core::broadcast(|context| {
                    let index = BroadcastContext::index(&context);
                    let num_threads = BroadcastContext::num_threads(&context);

                    assert_eq!(num_threads, thread_count);
                    assert_eq!(rayon_core::current_thread_index(), Some(index));

                    if index == 1 {
                        panic_hits.fetch_add(1, Ordering::SeqCst);
                        panic!("intentional rayon_core::broadcast panic from worker 1");
                    }

                    non_panic_hits.fetch_add(1, Ordering::SeqCst);
                    index
                });
            },
            || {
                let worker_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("sibling join branch should run inside the custom pool");
                assert!(worker_index < thread_count);
                worker_index
            },
        );
    }));

    let payload = panic_result
        .expect_err("a panic inside rayon_core::broadcast should propagate to the caller");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional rayon_core::broadcast panic from worker 1"),
        "unexpected broadcast panic payload: {panic_message:?}"
    );
    assert_eq!(panic_hits.load(Ordering::SeqCst), 1);
    assert!(
        non_panic_hits.load(Ordering::SeqCst) <= thread_count - 1,
        "non-panicking broadcast workers may or may not all run before the panic is observed"
    );

    assert_eq!(
        ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the caller should still not be a custom-pool worker"
    );

    let recovery_runs = AtomicUsize::new(0);

    let (mut recovery, pool_threads_seen) = ThreadPool::join(
        pool_ref,
        || {
            rayon_core::broadcast(|context| {
                let index = BroadcastContext::index(&context);
                let num_threads = BroadcastContext::num_threads(&context);

                recovery_runs.fetch_add(1, Ordering::SeqCst);

                assert_eq!(num_threads, thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                RecoveryBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed: (index + 5) * (num_threads + 17),
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                }
            })
        },
        || ThreadPool::current_num_threads(pool_ref),
    );

    assert_eq!(pool_threads_seen, thread_count);
    assert_eq!(recovery_runs.load(Ordering::SeqCst), thread_count);

    recovery.sort_by_key(|record| record.index);

    assert_eq!(recovery.len(), thread_count);
    assert_eq!(
        recovery
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(thread_count)
    );

    for record in &recovery {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, (record.index + 5) * (thread_count + 17));
        assert!(
            record.pending_status_available,
            "recovery broadcast should still run inside Rayon workers"
        );
    }

    let expected_seed_sum: usize = recovery.iter().map(|record| record.seed).sum();
    let non_panic_count_before_recovery = non_panic_hits.load(Ordering::SeqCst);
    let recovery_scoped = Mutex::new(Vec::<RecoveryScopedRecord>::new());

    let scope_return = ThreadPool::scope(pool_ref, |scope| {
        for record in recovery.iter().cloned() {
            let recovery_scoped = &recovery_scoped;

            Scope::spawn(scope, move |_| {
                let executing_index = ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery scoped work should run in the custom pool");

                assert!(executing_index < thread_count);
                assert_eq!(ThreadPool::current_num_threads(pool_ref), thread_count);

                let origin_index = record.index;
                let seed = record.seed;

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || thread_count + executing_index,
                );

                recovery_scoped
                    .lock()
                    .expect("recovery scoped mutex should not be poisoned")
                    .push(RecoveryScopedRecord {
                        origin_index,
                        seed,
                        executing_index,
                        value: left + right,
                    });
            });
        }

        expected_seed_sum + non_panic_count_before_recovery
    });

    assert_eq!(
        scope_return,
        expected_seed_sum + non_panic_count_before_recovery
    );

    let mut recovery_scoped = recovery_scoped
        .into_inner()
        .expect("recovery scoped mutex should not be poisoned");
    recovery_scoped.sort_by_key(|record| record.origin_index);

    assert_eq!(recovery_scoped.len(), thread_count);
    assert_eq!(
        recovery_scoped
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(thread_count)
    );

    let seed_by_index: BTreeMap<usize, usize> = recovery
        .iter()
        .map(|record| (record.index, record.seed))
        .collect();

    for record in &recovery_scoped {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(seed_by_index.get(&record.origin_index), Some(&record.seed));
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
    }

    let (observed_value_sum, recomputed_value_sum) = ThreadPool::join(
        pool_ref,
        || recovery_scoped.iter().map(|record| record.value).sum::<usize>(),
        || {
            recovery_scoped
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

    assert_eq!(observed_value_sum, recomputed_value_sum);
}