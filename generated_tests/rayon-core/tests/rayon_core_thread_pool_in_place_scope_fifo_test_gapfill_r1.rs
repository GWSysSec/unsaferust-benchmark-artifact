use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;

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
struct ConfirmationRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    total: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct QueuedBeforePanicRecord {
    input: usize,
    worker_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PanicRecoveryRecord {
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
fn thread_pool_in_place_scope_fifo_external_body_drives_fifo_broadcast_pipeline() {
    let thread_count = 3usize;
    let caller_thread = std::thread::current().id();
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("target-thread-pool-in-place-fifo-worker-{index}"))
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
            seed: (index + 1) * (num_threads + 29),
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

    for (expected_index, record) in seeds.iter().enumerate() {
        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(expected_index));
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 29));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_parent_sum: usize = seeds
        .iter()
        .map(|record| record.seed + record.index + record.num_threads)
        .sum();
    let expected_broadcast_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed * 2 + index + thread_count * 5)
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

    let outcome = rayon_core::ThreadPool::in_place_scope_fifo(pool_ref, |scope| {
        assert_eq!(
            std::thread::current().id(),
            caller_thread,
            "ThreadPool::in_place_scope_fifo should run the body on the calling thread"
        );
        assert_eq!(
            rayon_core::ThreadPool::current_thread_index(pool_ref),
            None,
            "external in-place FIFO body should not be migrated into the pool"
        );
        assert_eq!(
            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
            None
        );

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

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("FIFO parent work should run inside the custom pool");
                assert!(executing_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (seed_component, thread_component) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || seed + origin_index,
                    move || num_threads,
                );
                let parent_value = seed_component + thread_component;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(FifoParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                        value: parent_value,
                        pending_status_available:
                            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                .is_some(),
                    });

                rayon_core::ScopeFifo::spawn_fifo(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index =
                        rayon_core::ThreadPool::current_thread_index(pool_ref)
                            .expect("nested FIFO child should run inside the custom pool");
                    assert!(child_executing_index < thread_count);

                    let (doubled_parent, worker_component) = rayon_core::join(
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
                            value: doubled_parent + worker_component,
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

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), thread_count);

            let seed = seed_by_index_ref[index];
            let (doubled_seed, index_component) =
                rayon_core::join(move || seed * 2, move || index + num_threads * 5);
            let broadcast_value = doubled_seed + index_component;

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
                    .expect("FIFO child spawned by broadcast should run in the pool");
                assert!(executing_index < num_threads);

                broadcast_child_records_ref
                    .lock()
                    .expect("broadcast child mutex should not be poisoned")
                    .push(FifoBroadcastChildRecord {
                        origin_index: index,
                        executing_index,
                        value: broadcast_value + num_threads + executing_index,
                    });
            });
        });

        local_trace.borrow_mut().push(format!(
            "body-queued-parent-sum-{expected_parent_sum}-broadcast-sum-{expected_broadcast_sum}"
        ));

        LocalOutcome {
            trace: Rc::clone(&local_trace),
            scheduled_fifo_jobs: seeds.len(),
            scheduled_broadcast_jobs: thread_count,
            expected_parent_sum,
            expected_broadcast_sum,
        }
    });

    assert!(
        Rc::ptr_eq(&outcome.trace, &local_trace),
        "ThreadPool::in_place_scope_fifo should be able to return caller-local non-Send data"
    );
    assert_eq!(outcome.scheduled_fifo_jobs, thread_count);
    assert_eq!(outcome.scheduled_broadcast_jobs, thread_count);
    assert_eq!(outcome.expected_parent_sum, expected_parent_sum);
    assert_eq!(outcome.expected_broadcast_sum, expected_broadcast_sum);
    assert_eq!(
        outcome.trace.borrow().clone(),
        vec![
            format!("body-started-with-{thread_count}-seeds"),
            format!(
                "body-queued-parent-sum-{expected_parent_sum}-broadcast-sum-{expected_broadcast_sum}"
            )
        ]
    );

    assert_eq!(parent_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(child_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(broadcast_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        broadcast_child_started.load(Ordering::SeqCst),
        thread_count
    );

    let mut parent_records = parent_records
        .into_inner()
        .expect("parent record mutex should not be poisoned");
    parent_records.sort_by_key(|record| record.origin_index);

    assert_eq!(parent_records.len(), thread_count);
    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &parent_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + thread_count
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

    assert_eq!(child_records.len(), thread_count);
    assert_eq!(
        child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &child_records {
        assert!(record.origin_index < thread_count);
        assert!(record.parent_executing_index < thread_count);
        assert!(record.executing_index < thread_count);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("child should correspond to a FIFO parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(record.value, parent.value * 2 + record.executing_index);
    }

    let child_by_origin: BTreeMap<usize, FifoChildRecord> = child_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

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
            record.value,
            seed_by_index[record.index] * 2 + record.index + thread_count * 5
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

    let broadcast_by_index: BTreeMap<usize, FifoBroadcastRecord> = broadcast_records
        .iter()
        .cloned()
        .map(|record| (record.index, record))
        .collect();

    let mut broadcast_child_records = broadcast_child_records
        .into_inner()
        .expect("broadcast child mutex should not be poisoned");
    broadcast_child_records.sort_by_key(|record| record.origin_index);

    assert_eq!(broadcast_child_records.len(), thread_count);
    assert_eq!(
        broadcast_child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &broadcast_child_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(
            record.value,
            broadcast_by_index[&record.origin_index].value
                + thread_count
                + record.executing_index
        );
    }

    let mut confirmations = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let parent = parent_by_origin
            .get(&index)
            .expect("confirmation should find parent output by worker index");
        let child = child_by_origin
            .get(&index)
            .expect("confirmation should find child output by worker index");
        let broadcast = broadcast_by_index
            .get(&index)
            .expect("confirmation should find broadcast output by worker index");

        let parent_value = parent.value;
        let child_value = child.value;
        let broadcast_value = broadcast.value;

        let (left, right) =
            rayon_core::join(move || parent_value + child_value, move || broadcast_value);

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
        assert!(record.pending_status_available);
        assert_eq!(
            record.total,
            parent_by_origin[&record.index].value
                + child_by_origin[&record.index].value
                + broadcast_by_index[&record.index].value
        );
    }

    let (observed_parent_sum, observed_broadcast_sum) =
        rayon_core::ThreadPool::join(
            pool_ref,
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
fn thread_pool_in_place_scope_fifo_preserves_fifo_order_when_worker_is_released() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("target-in-place-fifo-order-worker-{index}"))
        .build()
        .expect("single-worker Rayon pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 1);
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);

    let observed_order = Arc::new(Mutex::new(Vec::<usize>::new()));
    let release_worker = Arc::new((Mutex::new(false), Condvar::new()));
    let (started_tx, started_rx) = mpsc::channel::<()>();

    rayon_core::ThreadPool::spawn(&pool, {
        let release_worker = Arc::clone(&release_worker);

        move || {
            assert_eq!(rayon_core::current_thread_index(), Some(0));
            assert_eq!(rayon_core::current_num_threads(), 1);

            started_tx
                .send(())
                .expect("blocking worker should report startup");

            let (lock, condvar) = &*release_worker;
            let mut released = lock
                .lock()
                .expect("release mutex should not be poisoned while blocking worker");
            while !*released {
                released = condvar
                    .wait(released)
                    .expect("release mutex should not be poisoned while waiting");
            }
        }
    });

    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("blocking worker task should start before FIFO jobs are queued");

    let returned = rayon_core::ThreadPool::in_place_scope_fifo(&pool, |scope| {
        assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);

        for value in 0usize..12 {
            let observed_order = Arc::clone(&observed_order);

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                assert_eq!(rayon_core::current_thread_index(), Some(0));
                assert_eq!(rayon_core::current_num_threads(), 1);
                assert!(
                    rayon_core::current_thread_has_pending_tasks().is_some(),
                    "FIFO work should be able to query worker-local pending-task status"
                );

                let (left, right) =
                    rayon_core::join(move || value, move || value * 10);

                observed_order
                    .lock()
                    .expect("observed-order mutex should not be poisoned")
                    .push(left + right);
            });
        }

        assert!(
            observed_order
                .lock()
                .expect("observed-order mutex should not be poisoned")
                .is_empty(),
            "queued FIFO work should not run while the only worker is blocked"
        );

        {
            let (lock, condvar) = &*release_worker;
            let mut released = lock
                .lock()
                .expect("release mutex should not be poisoned when releasing worker");
            *released = true;
            condvar.notify_one();
        }

        "released worker after queuing FIFO jobs"
    });

    assert_eq!(returned, "released worker after queuing FIFO jobs");

    let observed_order = observed_order
        .lock()
        .expect("observed-order mutex should not be poisoned")
        .clone();

    let expected_order: Vec<_> = (0usize..12).map(|value| value + value * 10).collect();

    assert_eq!(
        observed_order, expected_order,
        "ThreadPool::in_place_scope_fifo jobs queued by the caller should run FIFO on one worker"
    );

    let final_context = rayon_core::ThreadPool::broadcast(&pool, |context| {
        (
            rayon_core::BroadcastContext::index(&context),
            rayon_core::BroadcastContext::num_threads(&context),
            rayon_core::current_thread_index(),
        )
    });

    assert_eq!(final_context, vec![(0, 1, Some(0))]);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_in_place_scope_fifo_propagates_body_and_spawned_panics_then_recovers() {
    let thread_count = 3usize;
    let task_count = thread_count * 3 + 1;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("target-in-place-fifo-panic-recovery-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");
    let pool_ref = &pool;

    let started = AtomicUsize::new(0);
    let completed = Mutex::new(Vec::<QueuedBeforePanicRecord>::new());

    let body_panic = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::in_place_scope_fifo(pool_ref, |scope| {
            assert_eq!(rayon_core::ThreadPool::current_thread_index(pool_ref), None);

            for input in 0usize..task_count {
                let started_ref = &started;
                let completed_ref = &completed;

                rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                    started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                        .expect("prequeued FIFO work should run inside the custom pool");
                    assert!(worker_index < thread_count);
                    assert_eq!(
                        rayon_core::ThreadPool::current_num_threads(pool_ref),
                        thread_count
                    );

                    let (square, cube) =
                        rayon_core::join(move || input * input, move || input * input * input);

                    completed_ref
                        .lock()
                        .expect("completed record mutex should not be poisoned")
                        .push(QueuedBeforePanicRecord {
                            input,
                            worker_index,
                            num_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                            value: square + cube,
                            pending_status_available:
                                rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref)
                                    .is_some(),
                        });
                });
            }

            panic!(
                "intentional ThreadPool::in_place_scope_fifo body panic after scheduling FIFO work"
            );
        });
    }));

    let payload = body_panic.expect_err(
        "a body panic should propagate from ThreadPool::in_place_scope_fifo",
    );
    let body_message = panic_payload_to_string(&*payload);

    assert!(
        body_message.contains("ThreadPool::in_place_scope_fifo body panic"),
        "unexpected body panic payload: {body_message:?}"
    );
    assert_eq!(
        started.load(Ordering::SeqCst),
        task_count,
        "in_place_scope_fifo should wait for all FIFO jobs spawned before the body panic"
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
        assert!(record.worker_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
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
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the external caller should still not be a pool worker"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None
    );

    let completed_by_input: BTreeMap<usize, QueuedBeforePanicRecord> = completed
        .iter()
        .cloned()
        .map(|record| (record.input, record))
        .collect();
    let expected_completed_sum: usize = completed.iter().map(|record| record.value).sum();

    let recovery_records = Mutex::new(Vec::<PanicRecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::ThreadPool::in_place_scope_fifo(pool_ref, |scope| {
        assert_eq!(rayon_core::ThreadPool::current_thread_index(pool_ref), None);

        for record in completed.iter().cloned() {
            let recovery_records_ref = &recovery_records;
            let recovery_started_ref = &recovery_started;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery FIFO work should run inside the custom pool");
                assert!(executing_index < thread_count);

                let input = record.input;
                let original_worker_index = record.worker_index;

                let (left, right) = rayon_core::ThreadPool::join(
                    pool_ref,
                    move || record.value + input,
                    move || original_worker_index + executing_index + thread_count,
                );

                recovery_records_ref
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(PanicRecoveryRecord {
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
        assert!(record.executing_index < thread_count);

        let original = completed_by_input
            .get(&record.input)
            .expect("recovery record should correspond to a completed pre-panic record");

        assert_eq!(record.original_worker_index, original.worker_index);
        assert_eq!(
            record.value,
            original.value
                + original.input
                + original.worker_index
                + record.executing_index
                + thread_count
        );
    }

    let (observed_recovery_sum, recomputed_recovery_sum) =
        rayon_core::ThreadPool::join(
            pool_ref,
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
                            + thread_count
                    })
                    .sum::<usize>()
            },
        );

    assert_eq!(observed_recovery_sum, recomputed_recovery_sum);

    let spawned_panic_started = AtomicUsize::new(0);
    let spawned_panic = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::ThreadPool::in_place_scope_fifo(pool_ref, |scope| {
            rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
                spawned_panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking FIFO work should run inside the custom pool");
                assert!(worker_index < thread_count);
                assert!(
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some()
                );

                panic!("intentional spawned FIFO panic from ThreadPool::in_place_scope_fifo");
            });

            17usize
        });
    }));

    let payload = spawned_panic.expect_err(
        "a spawned FIFO task panic should propagate from ThreadPool::in_place_scope_fifo",
    );
    let spawned_message = panic_payload_to_string(&*payload);

    assert!(
        spawned_message.contains("spawned FIFO panic from ThreadPool::in_place_scope_fifo"),
        "unexpected spawned panic payload: {spawned_message:?}"
    );
    assert_eq!(spawned_panic_started.load(Ordering::SeqCst), 1);

    let mut final_broadcast = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (
            index,
            num_threads,
            rayon_core::current_thread_index(),
            index * 100 + num_threads,
        )
    });

    final_broadcast.sort_by_key(|record| record.0);

    assert_eq!(
        final_broadcast
            .iter()
            .map(|(index, _, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, num_threads, current_index, value) in final_broadcast {
        assert_eq!(num_threads, thread_count);
        assert_eq!(current_index, Some(index));
        assert_eq!(value, index * 100 + thread_count);
    }
}