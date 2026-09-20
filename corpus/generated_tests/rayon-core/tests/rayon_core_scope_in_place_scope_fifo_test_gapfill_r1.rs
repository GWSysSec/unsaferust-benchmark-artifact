use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
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
struct FifoParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FifoBroadcastChildRecord {
    origin_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug)]
struct LocalOutcome {
    trace: Rc<RefCell<Vec<String>>>,
    scheduled_fifo_jobs: usize,
    scheduled_broadcast_jobs: usize,
    expected_parent_sum: usize,
    expected_broadcast_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OrderSummary {
    body_index: Option<usize>,
    body_threads: usize,
    pending_status_available: bool,
    scheduled_jobs: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct QueuedBeforeBodyPanicRecord {
    input: usize,
    worker_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BodyPanicRecoveryRecord {
    input: usize,
    original_worker_index: usize,
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
fn module_scope_in_place_scope_fifo_runs_on_external_caller_and_chains_fifo_broadcast_work() {
    let caller_thread = std::thread::current().id();
    let global_threads = rayon_core::current_num_threads();

    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());
    assert_eq!(rayon_core::current_thread_index(), None);
    assert_eq!(rayon_core::current_thread_has_pending_tasks(), None);

    let mut seeds = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), global_threads);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 17),
        }
    });

    seeds.sort_by_key(|record| record.index);

    let expected_indices = expected_worker_indices(global_threads);
    assert_eq!(seeds.len(), global_threads);
    assert_eq!(
        seeds
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (expected_index, record) in seeds.iter().enumerate() {
        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(expected_index));
        assert_eq!(record.seed, (expected_index + 1) * (global_threads + 17));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_parent_sum: usize = seeds
        .iter()
        .map(|record| record.seed + record.index + record.num_threads)
        .sum();
    let expected_broadcast_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed * 2 + index + global_threads * 3)
        .sum();

    let parent_records = Mutex::new(Vec::<FifoParentRecord>::new());
    let child_records = Mutex::new(Vec::<FifoChildRecord>::new());
    let broadcast_records = Mutex::new(Vec::<FifoBroadcastRecord>::new());
    let broadcast_child_records = Mutex::new(Vec::<FifoBroadcastChildRecord>::new());

    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);
    let broadcast_started = AtomicUsize::new(0);
    let broadcast_child_started = AtomicUsize::new(0);

    let local_trace = Rc::new(RefCell::new(Vec::<String>::new()));

    let outcome = rayon_core::in_place_scope_fifo(|scope| {
        assert_eq!(
            std::thread::current().id(),
            caller_thread,
            "rayon_core::in_place_scope_fifo should run its body on the calling thread"
        );
        assert_eq!(
            rayon_core::current_thread_index(),
            None,
            "the in-place FIFO body should not be migrated to a Rayon worker"
        );
        assert_eq!(rayon_core::current_thread_has_pending_tasks(), None);
        assert_eq!(rayon_core::current_num_threads(), global_threads);

        local_trace
            .borrow_mut()
            .push(format!("body-started-with-{}-seeds", seeds.len()));

        for seed_record in seeds.iter().cloned() {
            let parent_records_ref = &parent_records;
            let child_records_ref = &child_records;
            let parent_started_ref = &parent_started;
            let child_started_ref = &child_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |nested_scope| {
                parent_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO parent work should run on a Rayon worker");
                assert!(executing_index < global_threads);
                assert_eq!(rayon_core::current_num_threads(), global_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (seed_component, thread_component) =
                    rayon_core::join(move || seed + origin_index, move || num_threads);
                let parent_value = seed_component + thread_component;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(FifoParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads: rayon_core::current_num_threads(),
                        value: parent_value,
                        pending_status_available: rayon_core::current_thread_has_pending_tasks()
                            .is_some(),
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index = rayon_core::current_thread_index()
                        .expect("nested FIFO child should run on a Rayon worker");
                    assert!(child_executing_index < global_threads);

                    let (parent_component, worker_component) = rayon_core::join(
                        move || parent_value * 2,
                        move || child_executing_index,
                    );

                    child_records_ref
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(FifoChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            value: parent_component + worker_component,
                        });
                });
            });
        }

        let seed_by_index_ref = &seed_by_index;
        let broadcast_records_ref = &broadcast_records;
        let broadcast_child_records_ref = &broadcast_child_records;
        let broadcast_started_ref = &broadcast_started;
        let broadcast_child_started_ref = &broadcast_child_started;

        rayon_core::ScopeFifo::spawn_broadcast(scope, move |scope, context| {
            broadcast_started_ref.fetch_add(1, Ordering::SeqCst);

            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, global_threads);
            assert!(index < num_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), global_threads);

            let seed = seed_by_index_ref[index];
            let (seed_component, index_component) =
                rayon_core::join(move || seed * 2, move || index + num_threads * 3);
            let broadcast_value = seed_component + index_component;

            broadcast_records_ref
                .lock()
                .expect("broadcast record mutex should not be poisoned")
                .push(FifoBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    value: broadcast_value,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                });

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                broadcast_child_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("FIFO child spawned by broadcast should run on a Rayon worker");
                assert!(executing_index < num_threads);

                broadcast_child_records_ref
                    .lock()
                    .expect("broadcast child record mutex should not be poisoned")
                    .push(FifoBroadcastChildRecord {
                        origin_index: index,
                        executing_index,
                        value: broadcast_value + num_threads + executing_index,
                    });
            });
        });

        local_trace.borrow_mut().push(format!(
            "body-spawned-parent-sum-{expected_parent_sum}-broadcast-sum-{expected_broadcast_sum}"
        ));

        LocalOutcome {
            trace: Rc::clone(&local_trace),
            scheduled_fifo_jobs: seeds.len(),
            scheduled_broadcast_jobs: global_threads,
            expected_parent_sum,
            expected_broadcast_sum,
        }
    });

    assert!(Rc::ptr_eq(&outcome.trace, &local_trace));
    assert_eq!(outcome.scheduled_fifo_jobs, global_threads);
    assert_eq!(outcome.scheduled_broadcast_jobs, global_threads);
    assert_eq!(outcome.expected_parent_sum, expected_parent_sum);
    assert_eq!(outcome.expected_broadcast_sum, expected_broadcast_sum);
    assert_eq!(
        outcome.trace.borrow().clone(),
        vec![
            format!("body-started-with-{global_threads}-seeds"),
            format!(
                "body-spawned-parent-sum-{expected_parent_sum}-broadcast-sum-{expected_broadcast_sum}"
            )
        ]
    );

    assert_eq!(parent_started.load(Ordering::SeqCst), global_threads);
    assert_eq!(child_started.load(Ordering::SeqCst), global_threads);
    assert_eq!(broadcast_started.load(Ordering::SeqCst), global_threads);
    assert_eq!(
        broadcast_child_started.load(Ordering::SeqCst),
        global_threads
    );

    let mut parent_records = parent_records
        .into_inner()
        .expect("parent record mutex should not be poisoned");
    parent_records.sort_by_key(|record| record.origin_index);

    assert_eq!(parent_records.len(), global_threads);
    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &parent_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + global_threads
        );
        assert!(
            record.pending_status_available,
            "FIFO parent work should observe worker-local pending-task status"
        );
    }

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>(),
        expected_parent_sum
    );

    let parent_by_origin: BTreeMap<usize, FifoParentRecord> = parent_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

    let mut child_records = child_records
        .into_inner()
        .expect("child record mutex should not be poisoned");
    child_records.sort_by_key(|record| record.origin_index);

    assert_eq!(child_records.len(), global_threads);
    assert_eq!(
        child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &child_records {
        assert!(record.origin_index < global_threads);
        assert!(record.parent_executing_index < global_threads);
        assert!(record.executing_index < global_threads);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("child record should correspond to a FIFO parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(record.value, parent.value * 2 + record.executing_index);
    }

    let mut broadcast_records = broadcast_records
        .into_inner()
        .expect("broadcast record mutex should not be poisoned");
    broadcast_records.sort_by_key(|record| record.index);

    assert_eq!(broadcast_records.len(), global_threads);
    assert_eq!(
        broadcast_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &broadcast_records {
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.value,
            seed_by_index[record.index] * 2 + record.index + global_threads * 3
        );
        assert!(
            record.pending_status_available,
            "broadcast work spawned from in_place_scope_fifo should observe pending-task status"
        );
    }

    assert_eq!(
        broadcast_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>(),
        expected_broadcast_sum
    );

    let mut broadcast_child_records = broadcast_child_records
        .into_inner()
        .expect("broadcast child record mutex should not be poisoned");
    broadcast_child_records.sort_by_key(|record| record.origin_index);

    assert_eq!(broadcast_child_records.len(), global_threads);
    assert_eq!(
        broadcast_child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &broadcast_child_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(
            record.value,
            seed_by_index[record.origin_index] * 2
                + record.origin_index
                + global_threads * 3
                + global_threads
                + record.executing_index
        );
    }

    let (observed_parent_sum, observed_broadcast_sum) = rayon_core::join(
        || parent_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            broadcast_records
                .iter()
                .map(|record| record.value)
                .sum::<usize>()
        },
    );

    assert_eq!(observed_parent_sum, expected_parent_sum);
    assert_eq!(observed_broadcast_sum, expected_broadcast_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn module_scope_in_place_scope_fifo_uses_current_single_worker_pool_and_preserves_fifo_order() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("module-scope-in-place-fifo-worker-{index}"))
        .build()
        .expect("single-worker custom Rayon pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let observed_order = Mutex::new(Vec::<usize>::new());

    let summary = rayon_core::ThreadPool::scope(&pool, |_| {
        assert_eq!(rayon_core::current_thread_index(), Some(0));
        assert_eq!(rayon_core::current_num_threads(), 1);

        rayon_core::in_place_scope_fifo(|scope| {
            assert_eq!(
                rayon_core::current_thread_index(),
                Some(0),
                "in_place_scope_fifo called from a pool worker should keep the body on that worker"
            );
            assert_eq!(rayon_core::current_num_threads(), 1);

            for value in 0usize..10 {
                let observed_order_ref = &observed_order;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    assert_eq!(rayon_core::current_thread_index(), Some(0));
                    assert_eq!(rayon_core::current_num_threads(), 1);
                    assert!(
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                        "FIFO work should be able to query worker-local pending-task status"
                    );

                    let (left, right) = rayon_core::join(move || value, move || value * 10);

                    observed_order_ref
                        .lock()
                        .expect("observed-order mutex should not be poisoned")
                        .push(left + right);
                });
            }

            OrderSummary {
                body_index: rayon_core::current_thread_index(),
                body_threads: rayon_core::current_num_threads(),
                pending_status_available: rayon_core::current_thread_has_pending_tasks()
                    .is_some(),
                scheduled_jobs: 10,
            }
        })
    });

    assert_eq!(summary.body_index, Some(0));
    assert_eq!(summary.body_threads, 1);
    assert!(summary.pending_status_available);
    assert_eq!(summary.scheduled_jobs, 10);

    let observed_order = observed_order
        .into_inner()
        .expect("observed-order mutex should not be poisoned");

    let expected_order: Vec<_> = (0usize..summary.scheduled_jobs)
        .map(|value| value + value * 10)
        .collect();

    assert_eq!(
        observed_order, expected_order,
        "FIFO jobs queued by the module-path in-place body should execute in queue order on one worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn module_scope_in_place_scope_fifo_body_panic_waits_for_prequeued_work_and_later_recovers() {
    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);

    let task_count = (global_threads + 4).min(12);
    let started = AtomicUsize::new(0);
    let completed = Mutex::new(Vec::<QueuedBeforeBodyPanicRecord>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::in_place_scope_fifo(|scope| {
            assert_eq!(
                rayon_core::current_thread_index(),
                None,
                "the body-panic test should run the in-place body on the external caller"
            );

            for input in 0usize..task_count {
                let started_ref = &started;
                let completed_ref = &completed;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::current_thread_index()
                        .expect("prequeued FIFO work should run on a Rayon worker");
                    assert!(worker_index < global_threads);
                    assert_eq!(rayon_core::current_num_threads(), global_threads);

                    let (square, cube) =
                        rayon_core::join(move || input * input, move || input * input * input);

                    completed_ref
                        .lock()
                        .expect("completed record mutex should not be poisoned")
                        .push(QueuedBeforeBodyPanicRecord {
                            input,
                            worker_index,
                            num_threads: rayon_core::current_num_threads(),
                            value: square + cube,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });
                });
            }

            panic!(
                "intentional rayon_core::scope::in_place_scope_fifo body panic after scheduling FIFO work"
            );
        });
    }));

    let payload = panic_result.expect_err(
        "a body panic should propagate out of rayon_core::scope::in_place_scope_fifo",
    );
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("scope::in_place_scope_fifo body panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );

    assert_eq!(
        started.load(Ordering::SeqCst),
        task_count,
        "in_place_scope_fifo should wait for all FIFO work spawned before a body panic"
    );

    let mut completed = completed
        .into_inner()
        .expect("completed record mutex should not be poisoned");
    completed.sort_by_key(|record| record.input);

    assert_eq!(completed.len(), task_count);
    assert_eq!(
        completed
            .iter()
            .map(|record| record.input)
            .collect::<BTreeSet<_>>(),
        (0usize..task_count).collect::<BTreeSet<_>>()
    );

    for record in &completed {
        assert!(record.worker_index < global_threads);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(
            record.value,
            record.input * record.input + record.input * record.input * record.input
        );
        assert!(
            record.pending_status_available,
            "prequeued FIFO work should observe worker-local pending-task status"
        );
    }

    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "after unwinding, the caller should still not be a Rayon worker"
    );
    assert_eq!(rayon_core::current_thread_has_pending_tasks(), None);

    let completed_by_input: BTreeMap<usize, QueuedBeforeBodyPanicRecord> = completed
        .iter()
        .cloned()
        .map(|record| (record.input, record))
        .collect();
    assert_eq!(completed_by_input.len(), task_count);

    let expected_completed_sum: usize = completed.iter().map(|record| record.value).sum();
    let recovery_records = Mutex::new(Vec::<BodyPanicRecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::in_place_scope_fifo(|scope| {
        assert_eq!(rayon_core::current_thread_index(), None);

        for record in completed.iter().cloned() {
            let recovery_records_ref = &recovery_records;
            let recovery_started_ref = &recovery_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery FIFO work should run on a Rayon worker");
                assert!(executing_index < global_threads);

                let input = record.input;
                let original_worker_index = record.worker_index;

                let (left, right) = rayon_core::join(
                    move || record.value + input,
                    move || original_worker_index + executing_index + global_threads,
                );

                recovery_records_ref
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(BodyPanicRecoveryRecord {
                        input,
                        original_worker_index,
                        executing_index,
                        value: left + right,
                    });
            });
        }

        expected_completed_sum
    });

    assert_eq!(recovery_return, expected_completed_sum);
    assert_eq!(recovery_started.load(Ordering::SeqCst), task_count);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| record.input);

    assert_eq!(recovery_records.len(), task_count);
    assert_eq!(
        recovery_records
            .iter()
            .map(|record| record.input)
            .collect::<BTreeSet<_>>(),
        (0usize..task_count).collect::<BTreeSet<_>>()
    );

    for record in &recovery_records {
        assert!(record.executing_index < global_threads);

        let original = completed_by_input
            .get(&record.input)
            .expect("recovery record should correspond to a pre-panic record");

        assert_eq!(record.original_worker_index, original.worker_index);
        assert_eq!(
            record.value,
            original.value
                + original.input
                + original.worker_index
                + record.executing_index
                + global_threads
        );
    }

    let (observed_recovery_sum, recomputed_recovery_sum) = rayon_core::join(
        || recovery_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            recovery_records
                .iter()
                .map(|record| {
                    let original = completed_by_input
                        .get(&record.input)
                        .expect("original record should exist during recomputation");

                    original.value
                        + original.input
                        + original.worker_index
                        + record.executing_index
                        + global_threads
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_recovery_sum, recomputed_recovery_sum);
}