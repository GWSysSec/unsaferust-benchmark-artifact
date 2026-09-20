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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    broadcast_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoChildRecord {
    origin_index: usize,
    executing_index: usize,
    num_threads: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoGrandchildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
    origin_index: usize,
    seed: usize,
    num_threads: usize,
    executing_index: usize,
    value: usize,
}

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn scope_fifo_spawn_broadcast_uses_prior_broadcast_data_and_waits_for_fifo_descendants() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-fifo-spawn-broadcast-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "the integration-test thread should not be a worker in this custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None,
        "pending-task status should be unavailable outside a Rayon worker"
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 23),
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
        assert_eq!(seed.seed, (expected_index + 1) * (thread_count + 23));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|seed| seed.seed).collect();
    let expected_broadcast_value_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed + index + thread_count * 10)
        .sum();

    let broadcast_records = Mutex::new(Vec::<FifoBroadcastRecord>::new());
    let child_records = Mutex::new(Vec::<FifoChildRecord>::new());
    let grandchild_records = Mutex::new(Vec::<FifoGrandchildRecord>::new());
    let child_started = AtomicUsize::new(0);
    let grandchild_started = AtomicUsize::new(0);

    let scope_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        rayon_core::ScopeFifo::spawn_broadcast(scope, |scope, context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert!(index < num_threads);
            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_num_threads(), thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let seed = seed_by_index[index];
            let (left, right) = rayon_core::join(
                move || seed + index,
                move || num_threads * 10,
            );
            let broadcast_value = left + right;

            broadcast_records
                .lock()
                .expect("broadcast record mutex should not be poisoned")
                .push(FifoBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    broadcast_value,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                });

            let child_records = &child_records;
            let grandchild_records = &grandchild_records;
            let child_started = &child_started;
            let grandchild_started = &grandchild_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |nested_scope| {
                child_started.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("child FIFO work should execute on a Rayon worker");

                assert!(executing_index < num_threads);
                assert_eq!(rayon_core::current_num_threads(), num_threads);

                let (value_from_broadcast, value_from_worker) = rayon_core::join(
                    move || broadcast_value,
                    move || executing_index + num_threads,
                );
                let child_value = value_from_broadcast + value_from_worker;

                child_records
                    .lock()
                    .expect("child record mutex should not be poisoned")
                    .push(FifoChildRecord {
                        origin_index: index,
                        executing_index,
                        num_threads,
                        value: child_value,
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    grandchild_started.fetch_add(1, Ordering::SeqCst);

                    let grandchild_executing_index = rayon_core::current_thread_index()
                        .expect("grandchild FIFO work should execute on a Rayon worker");

                    assert!(grandchild_executing_index < num_threads);

                    let (doubled_child_value, worker_component) = rayon_core::join(
                        move || child_value * 2,
                        move || grandchild_executing_index,
                    );

                    grandchild_records
                        .lock()
                        .expect("grandchild record mutex should not be poisoned")
                        .push(FifoGrandchildRecord {
                            origin_index: index,
                            parent_executing_index: executing_index,
                            executing_index: grandchild_executing_index,
                            value: doubled_child_value + worker_component,
                        });
                });
            });
        });

        expected_broadcast_value_sum
    });

    assert_eq!(scope_return, expected_broadcast_value_sum);
    assert_eq!(child_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(grandchild_started.load(Ordering::SeqCst), thread_count);

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
            record.broadcast_value,
            seed_by_index[record.index] + record.index + thread_count * 10
        );
        assert!(
            record.pending_status_available,
            "broadcast work should be able to query worker pending-task status"
        );
    }

    assert_eq!(
        broadcast_records
            .iter()
            .map(|record| record.broadcast_value)
            .sum::<usize>(),
        expected_broadcast_value_sum
    );

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

    for child in &child_records {
        assert!(child.origin_index < thread_count);
        assert!(child.executing_index < thread_count);
        assert_eq!(child.num_threads, thread_count);

        let broadcast_value =
            seed_by_index[child.origin_index] + child.origin_index + thread_count * 10;

        assert_eq!(
            child.value,
            broadcast_value + child.executing_index + thread_count
        );
    }

    let child_by_origin: BTreeMap<usize, FifoChildRecord> = child_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(child_by_origin.len(), thread_count);

    let mut grandchild_records = grandchild_records
        .into_inner()
        .expect("grandchild record mutex should not be poisoned");
    grandchild_records.sort_by_key(|record| record.origin_index);

    assert_eq!(grandchild_records.len(), thread_count);
    assert_eq!(
        grandchild_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for grandchild in &grandchild_records {
        assert!(grandchild.origin_index < thread_count);
        assert!(grandchild.executing_index < thread_count);

        let child = child_by_origin
            .get(&grandchild.origin_index)
            .expect("each grandchild record should correspond to a child FIFO record");

        assert_eq!(grandchild.parent_executing_index, child.executing_index);
        assert_eq!(
            grandchild.value,
            child.value * 2 + grandchild.executing_index
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn scope_fifo_spawn_broadcast_panic_propagates_and_pool_recovers_for_more_fifo_broadcast_work() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-fifo-spawn-broadcast-panic-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    let panic_hits = AtomicUsize::new(0);
    let non_panic_hits = AtomicUsize::new(0);
    let non_panic_indices = Mutex::new(Vec::<usize>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
            rayon_core::ScopeFifo::spawn_broadcast(scope, |_, context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(num_threads, thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));

                if index == 1 {
                    panic_hits.fetch_add(1, Ordering::SeqCst);
                    panic!("intentional ScopeFifo::spawn_broadcast panic from worker 1");
                } else {
                    non_panic_hits.fetch_add(1, Ordering::SeqCst);
                    non_panic_indices
                        .lock()
                        .expect("non-panic index mutex should not be poisoned")
                        .push(index);
                }
            });

            9000usize
        })
    }));

    let payload = panic_result.expect_err(
        "a panic in a ScopeFifo::spawn_broadcast worker should propagate out of scope_fifo",
    );

    let panic_message = if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_owned()
    };

    assert!(
        panic_message.contains("intentional ScopeFifo::spawn_broadcast panic from worker 1"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_hits.load(Ordering::SeqCst), 1);
    assert!(non_panic_hits.load(Ordering::SeqCst) <= thread_count - 1);

    let seen_before_recovery = non_panic_indices
        .lock()
        .expect("non-panic index mutex should not be poisoned")
        .clone();

    assert!(
        seen_before_recovery
            .iter()
            .all(|index| *index < thread_count && *index != 1),
        "only non-panicking worker indices should be recorded before recovery"
    );
    assert_eq!(
        seen_before_recovery.iter().copied().collect::<BTreeSet<_>>().len(),
        seen_before_recovery.len(),
        "each non-panicking worker should be recorded at most once"
    );

    let count_after_panic = non_panic_hits.load(Ordering::SeqCst);

    let mut recovery_seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, (index + 2) * (num_threads + 31))
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

    let seed_by_index: Vec<_> = recovery_seeds.iter().map(|(_, seed)| *seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
    let recovery_nested_jobs = AtomicUsize::new(0);

    let recovery_return = rayon_core::ThreadPool::in_place_scope_fifo(&pool, |scope| {
        rayon_core::ScopeFifo::spawn_broadcast(scope, |scope, context| {
            let origin_index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(origin_index));

            let seed = seed_by_index[origin_index];
            let recovery_records = &recovery_records;
            let recovery_nested_jobs = &recovery_nested_jobs;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                recovery_nested_jobs.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery FIFO work should execute on a Rayon worker");

                assert!(executing_index < num_threads);

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || num_threads + executing_index,
                );

                recovery_records
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        origin_index,
                        seed,
                        num_threads,
                        executing_index,
                        value: left + right,
                    });
            });
        });

        expected_seed_sum + count_after_panic
    });

    assert_eq!(recovery_return, expected_seed_sum + count_after_panic);
    assert_eq!(recovery_nested_jobs.load(Ordering::SeqCst), thread_count);

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
        assert_eq!(record.num_threads, thread_count);
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::ThreadPool::join(
        &pool,
        || recovery_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            recovery_records
                .iter()
                .map(|record| {
                    seed_by_index[record.origin_index]
                        + record.origin_index
                        + thread_count
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}