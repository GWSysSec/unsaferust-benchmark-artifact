use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkerSeed {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NestedRecord {
    origin_index: usize,
    executing_index: usize,
    worker_threads: usize,
    joined_value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
    origin_index: usize,
    num_threads: usize,
    executing_index: usize,
    value: usize,
}

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn scope_spawn_broadcast_uses_broadcast_seed_data_and_waits_for_nested_scope_work() {
    let thread_count = 4usize;

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-spawn-broadcast-seed-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "the test thread should not itself be a worker in this pool"
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        WorkerSeed {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 17),
        }
    });

    seeds.sort_by_key(|seed| seed.index);

    let expected_indices = expected_worker_indices(thread_count);
    let observed_seed_indices: BTreeSet<_> = seeds.iter().map(|seed| seed.index).collect();
    assert_eq!(observed_seed_indices, expected_indices);

    for (position, seed) in seeds.iter().enumerate() {
        assert_eq!(seed.index, position);
        assert_eq!(seed.num_threads, thread_count);
        assert_eq!(seed.current_index, Some(seed.index));
        assert_eq!(seed.seed, (seed.index + 1) * (thread_count + 17));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|seed| seed.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let broadcast_records = Mutex::new(Vec::<BroadcastRecord>::new());
    let nested_records = Mutex::new(Vec::<NestedRecord>::new());
    let nested_started = AtomicUsize::new(0);

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        rayon_core::Scope::spawn_broadcast(scope, |scope, context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert!(index < num_threads);
            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_num_threads(), thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let seed = seed_by_index[index];
            let pending_status_available =
                rayon_core::current_thread_has_pending_tasks().is_some();

            broadcast_records
                .lock()
                .expect("broadcast record mutex should not be poisoned")
                .push(BroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    pending_status_available,
                });

            let nested_records = &nested_records;
            let nested_started = &nested_started;

            rayon_core::Scope::spawn(scope, move |_| {
                nested_started.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("nested scoped work should execute on a Rayon worker");

                assert!(executing_index < num_threads);
                assert_eq!(rayon_core::current_num_threads(), num_threads);

                let (left, right) = rayon_core::join(
                    move || seed + index,
                    move || num_threads * 100 + executing_index,
                );

                nested_records
                    .lock()
                    .expect("nested record mutex should not be poisoned")
                    .push(NestedRecord {
                        origin_index: index,
                        executing_index,
                        worker_threads: rayon_core::current_num_threads(),
                        joined_value: left + right,
                    });
            });
        });

        seed_by_index.iter().copied().sum::<usize>()
    });

    assert_eq!(scope_return, expected_seed_sum);
    assert_eq!(nested_started.load(Ordering::SeqCst), thread_count);

    let mut broadcast_records = broadcast_records
        .into_inner()
        .expect("broadcast record mutex should not be poisoned");
    broadcast_records.sort_by_key(|record| record.index);

    assert_eq!(broadcast_records.len(), thread_count);

    let observed_broadcast_indices: BTreeSet<_> =
        broadcast_records.iter().map(|record| record.index).collect();
    assert_eq!(observed_broadcast_indices, expected_indices);

    for record in &broadcast_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert!(
            record.pending_status_available,
            "broadcast work should be able to query worker pending-task status"
        );
    }

    let broadcast_seed_sum: usize = broadcast_records.iter().map(|record| record.seed).sum();
    assert_eq!(broadcast_seed_sum, expected_seed_sum);

    let mut nested_records = nested_records
        .into_inner()
        .expect("nested record mutex should not be poisoned");
    nested_records.sort_by_key(|record| record.origin_index);

    assert_eq!(nested_records.len(), thread_count);

    let observed_nested_origins: BTreeSet<_> =
        nested_records.iter().map(|record| record.origin_index).collect();
    assert_eq!(observed_nested_origins, expected_indices);

    for record in &nested_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.worker_threads, thread_count);

        assert_eq!(
            record.joined_value,
            seed_by_index[record.origin_index]
                + record.origin_index
                + thread_count * 100
                + record.executing_index
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn scope_spawn_broadcast_panic_propagates_and_pool_recovers_for_follow_up_broadcast_scope() {
    let thread_count = 3usize;

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("scope-spawn-broadcast-panic-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    let panic_hits = AtomicUsize::new(0);
    let non_panic_hits = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::ThreadPool::scope(&pool, |scope| {
            rayon_core::Scope::spawn_broadcast(scope, |_, context| {
                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(num_threads, thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));

                if index == 0 {
                    panic_hits.fetch_add(1, Ordering::SeqCst);
                    panic!("intentional Scope::spawn_broadcast panic from worker 0");
                } else {
                    non_panic_hits.fetch_add(1, Ordering::SeqCst);
                }
            });
        });
    }));

    let payload = panic_result
        .expect_err("a panic in a Scope::spawn_broadcast worker should propagate out of scope");

    let panic_message = if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_owned()
    };

    assert!(
        panic_message.contains("intentional Scope::spawn_broadcast panic from worker 0"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_hits.load(Ordering::SeqCst), 1);
    assert!(non_panic_hits.load(Ordering::SeqCst) <= thread_count - 1);

    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
    let recovery_count = AtomicUsize::new(0);
    let non_panic_count_after_panic = non_panic_hits.load(Ordering::SeqCst);

    let recovery_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        rayon_core::Scope::spawn_broadcast(scope, |scope, context| {
            let origin_index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(origin_index));

            recovery_count.fetch_add(1, Ordering::SeqCst);

            let recovery_records = &recovery_records;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery nested work should execute on a Rayon worker");

                assert!(executing_index < num_threads);

                let (origin_score, worker_score) = rayon_core::join(
                    move || origin_index * 10,
                    move || num_threads + executing_index,
                );

                recovery_records
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        origin_index,
                        num_threads,
                        executing_index,
                        value: origin_score + worker_score,
                    });
            });
        });

        1000usize + non_panic_count_after_panic
    });

    assert_eq!(recovery_return, 1000 + non_panic_count_after_panic);
    assert_eq!(recovery_count.load(Ordering::SeqCst), thread_count);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| record.origin_index);

    assert_eq!(recovery_records.len(), thread_count);

    let expected_indices = expected_worker_indices(thread_count);
    let observed_indices: BTreeSet<_> = recovery_records
        .iter()
        .map(|record| record.origin_index)
        .collect();
    assert_eq!(observed_indices, expected_indices);

    for record in &recovery_records {
        assert_eq!(record.num_threads, thread_count);
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(
            record.value,
            record.origin_index * 10 + thread_count + record.executing_index
        );
    }
}