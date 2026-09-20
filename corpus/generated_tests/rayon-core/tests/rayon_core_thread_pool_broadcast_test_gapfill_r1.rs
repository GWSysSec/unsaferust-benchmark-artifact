use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
struct BroadcastSeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: Option<String>,
    seed: usize,
    joined_value: usize,
    pending_status_available: bool,
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
    pending_status_available: bool,
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
fn thread_pool_broadcast_collects_worker_context_and_feeds_scoped_confirmation_pipeline() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let builder = rayon_core::ThreadPoolBuilder::new();
    let builder = rayon_core::ThreadPoolBuilder::num_threads(builder, thread_count);
    let builder = rayon_core::ThreadPoolBuilder::thread_name(builder, |index| {
        format!("thread-pool-broadcast-pipeline-worker-{index}")
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("custom Rayon thread pool should build");
    let pool_ref = &pool;

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(pool_ref),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "the integration-test thread should not be a worker in the custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None,
        "pending-task status should be unavailable outside the custom pool"
    );

    let mut broadcast_records = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert!(index < num_threads);
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(
            rayon_core::ThreadPool::current_thread_index(pool_ref),
            Some(index)
        );

        let expected_name = format!("thread-pool-broadcast-pipeline-worker-{index}");
        let worker_name = std::thread::current().name().map(str::to_owned);
        assert_eq!(worker_name.as_deref(), Some(expected_name.as_str()));

        let seed = (index + 1) * (num_threads + 97);
        let (seed_component, thread_component) =
            rayon_core::join(move || seed + index, move || num_threads * 10);

        BroadcastSeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            worker_name,
            seed,
            joined_value: seed_component + thread_component,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    broadcast_records.sort_by_key(|record| record.index);

    assert_eq!(broadcast_records.len(), thread_count);
    assert_eq!(
        broadcast_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (expected_index, record) in broadcast_records.iter().enumerate() {
        let expected_name = format!("thread-pool-broadcast-pipeline-worker-{expected_index}");

        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(expected_index));
        assert_eq!(record.worker_name.as_deref(), Some(expected_name.as_str()));
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 97));
        assert_eq!(
            record.joined_value,
            record.seed + record.index + thread_count * 10
        );
        assert!(
            record.pending_status_available,
            "ThreadPool::broadcast work should observe worker-local pending-task status"
        );
    }

    let seed_by_index: Vec<_> = broadcast_records
        .iter()
        .map(|record| record.seed)
        .collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();
    let expected_joined_sum: usize = broadcast_records
        .iter()
        .map(|record| record.joined_value)
        .sum();

    let broadcast_by_index: BTreeMap<usize, BroadcastSeedRecord> = broadcast_records
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();
    assert_eq!(broadcast_by_index.len(), thread_count);

    let scoped_started = AtomicUsize::new(0);
    let scoped_records = Mutex::new(Vec::<ScopedDerivedRecord>::new());

    let scope_summary = rayon_core::ThreadPool::scope(pool_ref, |scope| {
        let body_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
            .expect("ThreadPool::scope body should run inside the custom pool");
        assert!(body_index < thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(body_index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let pending_status_available =
            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();

        for record in broadcast_records.iter().cloned() {
            let scoped_records_ref = &scoped_records;
            let scoped_started_ref = &scoped_started;

            rayon_core::Scope::spawn(scope, move |_| {
                scoped_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("scoped follow-up work should run in the custom pool");
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
                    move || origin_index + executing_index + thread_count,
                );

                scoped_records_ref
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
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

        ScopeSummary {
            body_index,
            body_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
            scheduled_jobs: broadcast_records.len(),
            seed_sum: expected_seed_sum,
            pending_status_available,
        }
    });

    assert!(scope_summary.body_index < thread_count);
    assert_eq!(scope_summary.body_threads, thread_count);
    assert_eq!(scope_summary.scheduled_jobs, thread_count);
    assert_eq!(scope_summary.seed_sum, expected_seed_sum);
    assert!(scope_summary.pending_status_available);
    assert_eq!(scoped_started.load(Ordering::SeqCst), thread_count);

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
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.broadcast_joined_value,
            broadcast_by_index[&record.origin_index].joined_value
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
            "scoped work derived from ThreadPool::broadcast should observe pending-task status"
        );
    }

    let scoped_by_origin: BTreeMap<usize, ScopedDerivedRecord> = scoped_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(scoped_by_origin.len(), thread_count);

    let mut confirmations = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let scoped = scoped_by_origin
            .get(&index)
            .expect("confirmation broadcast should find scoped output for worker index");
        let original = broadcast_by_index
            .get(&index)
            .expect("confirmation broadcast should find original broadcast output");

        let scoped_value = scoped.value;
        let original_joined = original.joined_value;
        let (left, right) =
            rayon_core::join(move || scoped_value, move || original_joined + num_threads);

        ConfirmationRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            total: left + right,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

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
        assert!(
            record.pending_status_available,
            "confirmation ThreadPool::broadcast should run on pool workers"
        );
        assert_eq!(
            record.total,
            scoped_by_origin[&record.index].value
                + broadcast_by_index[&record.index].joined_value
                + thread_count
        );
    }

    let (observed_joined_sum, recomputed_joined_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || broadcast_records.iter().map(|record| record.joined_value).sum::<usize>(),
        || {
            broadcast_records
                .iter()
                .map(|record| record.seed + record.index + thread_count * 10)
                .sum::<usize>()
        },
    );

    assert_eq!(observed_joined_sum, expected_joined_sum);
    assert_eq!(observed_joined_sum, recomputed_joined_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_broadcast_propagates_worker_panic_and_pool_recovers_for_later_fifo_work() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let builder = rayon_core::ThreadPoolBuilder::new();
    let builder = rayon_core::ThreadPoolBuilder::num_threads(builder, thread_count);
    let builder = rayon_core::ThreadPoolBuilder::thread_name(builder, |index| {
        format!("thread-pool-broadcast-panic-worker-{index}")
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("custom Rayon thread pool should build");
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

    let panic_hits = AtomicUsize::new(0);
    let non_panic_hits = AtomicUsize::new(0);
    let non_panic_mask = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: Vec<usize> = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            if index == 1 {
                panic_hits.fetch_add(1, Ordering::SeqCst);
                panic!("intentional ThreadPool::broadcast panic from worker 1");
            }

            non_panic_hits.fetch_add(1, Ordering::SeqCst);
            non_panic_mask.fetch_or(1usize << index, Ordering::SeqCst);

            index + num_threads
        });
    }));

    let payload = panic_result
        .expect_err("panic in ThreadPool::broadcast worker should propagate to the caller");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("ThreadPool::broadcast panic from worker 1"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_hits.load(Ordering::SeqCst), 1);
    assert!(non_panic_hits.load(Ordering::SeqCst) <= thread_count - 1);

    let mask = non_panic_mask.load(Ordering::SeqCst);
    assert_eq!(
        mask & (1usize << 1),
        0,
        "the panicking worker should not be recorded as a non-panicking worker"
    );
    assert_eq!(mask & !((1usize << thread_count) - 1), 0);

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the external caller should still not be a pool worker"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None
    );

    let mut recovery_records = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        RecoveryBroadcastRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 3) * (num_threads + 23),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

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
        assert_eq!(record.seed, (record.index + 3) * (thread_count + 23));
        assert!(
            record.pending_status_available,
            "recovery ThreadPool::broadcast should run on Rayon workers"
        );
    }

    let seed_by_origin: BTreeMap<usize, usize> = recovery_records
        .iter()
        .map(|record| (record.index, record.seed))
        .collect();
    assert_eq!(seed_by_origin.len(), thread_count);

    let expected_seed_sum: usize = recovery_records.iter().map(|record| record.seed).sum();
    let fifo_started = AtomicUsize::new(0);
    let fifo_records = Mutex::new(Vec::<RecoveryFifoRecord>::new());

    let scope_fifo_return = rayon_core::ThreadPool::scope_fifo(pool_ref, |scope| {
        let body_index = rayon_core::current_thread_index()
            .expect("ThreadPool::scope_fifo body should run inside the custom pool");
        assert!(body_index < thread_count);

        for record in recovery_records.iter().cloned() {
            let fifo_started_ref = &fifo_started;
            let fifo_records_ref = &fifo_records;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                fifo_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery FIFO work should run on a Rayon worker");
                assert!(executing_index < record.num_threads);
                assert_eq!(rayon_core::current_num_threads(), record.num_threads);

                let origin_index = record.index;
                let seed = record.seed;
                let num_threads = record.num_threads;

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || num_threads + executing_index,
                );

                fifo_records_ref
                    .lock()
                    .expect("FIFO recovery record mutex should not be poisoned")
                    .push(RecoveryFifoRecord {
                        origin_index,
                        seed,
                        executing_index,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        expected_seed_sum + non_panic_hits.load(Ordering::SeqCst)
    });

    assert_eq!(
        scope_fifo_return,
        expected_seed_sum + non_panic_hits.load(Ordering::SeqCst)
    );
    assert_eq!(fifo_started.load(Ordering::SeqCst), thread_count);

    let mut fifo_records = fifo_records
        .into_inner()
        .expect("FIFO recovery record mutex should not be poisoned");
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
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "FIFO recovery work should observe worker-local pending-task status"
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::ThreadPool::join(
        pool_ref,
        || fifo_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            fifo_records
                .iter()
                .map(|record| {
                    seed_by_origin[&record.origin_index]
                        + record.origin_index
                        + thread_count
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);

    let mut final_check = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
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
    });

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