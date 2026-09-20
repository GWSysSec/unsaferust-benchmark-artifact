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
struct InPlaceParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InPlaceChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InPlaceBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InPlaceBroadcastChildRecord {
    origin_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug)]
struct LocalOutcome {
    trace: Rc<RefCell<Vec<String>>>,
    scheduled_parent_jobs: usize,
    scheduled_broadcast_jobs: usize,
    expected_parent_sum: usize,
    expected_broadcast_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BodyPanicQueuedRecord {
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpawnPanicRecoveryRecord {
    stage: usize,
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
fn free_in_place_scope_runs_body_on_caller_and_drives_nested_broadcast_pipeline() {
    let caller_thread = std::thread::current().id();
    let global_threads = rayon_core::current_num_threads();

    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "the integration-test thread should not begin as a Rayon worker"
    );
    assert_eq!(rayon_core::current_thread_has_pending_tasks(), None);

    let mut seeds = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert!(index < num_threads);
        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), num_threads);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 19),
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
        assert_eq!(
            record.seed,
            (expected_index + 1) * (global_threads + 19)
        );
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_parent_sum: usize = seeds
        .iter()
        .map(|record| record.seed + record.index + record.num_threads)
        .sum();
    let expected_broadcast_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed * 3 + index + global_threads * 5)
        .sum();

    let parent_records = Mutex::new(Vec::<InPlaceParentRecord>::new());
    let child_records = Mutex::new(Vec::<InPlaceChildRecord>::new());
    let broadcast_records = Mutex::new(Vec::<InPlaceBroadcastRecord>::new());
    let broadcast_child_records = Mutex::new(Vec::<InPlaceBroadcastChildRecord>::new());

    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);
    let broadcast_started = AtomicUsize::new(0);
    let broadcast_child_started = AtomicUsize::new(0);

    let local_trace = Rc::new(RefCell::new(Vec::<String>::new()));

    let outcome = rayon_core::in_place_scope(|scope| {
        assert_eq!(
            std::thread::current().id(),
            caller_thread,
            "rayon_core::in_place_scope should run its body on the calling thread"
        );
        assert_eq!(
            rayon_core::current_thread_index(),
            None,
            "the in-place body itself should not be migrated to a Rayon worker"
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

            rayon_core::Scope::spawn(scope, move |nested_scope| {
                parent_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("parent work spawned by in_place_scope should run on a Rayon worker");
                assert!(executing_index < global_threads);
                assert_eq!(rayon_core::current_num_threads(), global_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (left, right) =
                    rayon_core::join(move || seed + origin_index, move || num_threads);
                let parent_value = left + right;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(InPlaceParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads: rayon_core::current_num_threads(),
                        value: parent_value,
                        pending_status_available: rayon_core::current_thread_has_pending_tasks()
                            .is_some(),
                    });

                rayon_core::Scope::spawn(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index = rayon_core::current_thread_index()
                        .expect("nested child work should run on a Rayon worker");
                    assert!(child_executing_index < global_threads);

                    let (parent_component, worker_component) = rayon_core::join(
                        move || parent_value * 2,
                        move || child_executing_index,
                    );

                    child_records_ref
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(InPlaceChildRecord {
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

        rayon_core::Scope::spawn_broadcast(scope, move |scope, context| {
            broadcast_started_ref.fetch_add(1, Ordering::SeqCst);

            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, global_threads);
            assert!(index < num_threads);
            assert_eq!(rayon_core::current_thread_index(), Some(index));
            assert_eq!(rayon_core::current_num_threads(), global_threads);

            let seed = seed_by_index_ref[index];
            let (seed_component, index_component) =
                rayon_core::join(move || seed * 3, move || index + num_threads * 5);
            let broadcast_value = seed_component + index_component;

            broadcast_records_ref
                .lock()
                .expect("broadcast record mutex should not be poisoned")
                .push(InPlaceBroadcastRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    seed,
                    value: broadcast_value,
                    pending_status_available: rayon_core::current_thread_has_pending_tasks()
                        .is_some(),
                });

            rayon_core::Scope::spawn(scope, move |_| {
                broadcast_child_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("broadcast child work should run on a Rayon worker");
                assert!(executing_index < num_threads);

                broadcast_child_records_ref
                    .lock()
                    .expect("broadcast child record mutex should not be poisoned")
                    .push(InPlaceBroadcastChildRecord {
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
            scheduled_parent_jobs: seeds.len(),
            scheduled_broadcast_jobs: global_threads,
            expected_parent_sum,
            expected_broadcast_sum,
        }
    });

    assert!(
        Rc::ptr_eq(&outcome.trace, &local_trace),
        "rayon_core::in_place_scope should be able to return caller-local non-Send data"
    );
    assert_eq!(outcome.scheduled_parent_jobs, global_threads);
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
            "parent work should be able to query worker-local pending-task status"
        );
    }

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.value)
            .sum::<usize>(),
        expected_parent_sum
    );

    let parent_by_origin: BTreeMap<usize, InPlaceParentRecord> = parent_records
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
            .expect("child record should correspond to a parent record");

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
            seed_by_index[record.index] * 3 + record.index + global_threads * 5
        );
        assert!(
            record.pending_status_available,
            "broadcast work should observe worker-local pending-task status"
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
            seed_by_index[record.origin_index] * 3
                + record.origin_index
                + global_threads * 5
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
fn free_in_place_scope_propagates_body_panic_after_running_previously_spawned_jobs_and_recovers() {
    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);

    let task_count = global_threads + 4;
    let started = AtomicUsize::new(0);
    let completed = Mutex::new(Vec::<BodyPanicQueuedRecord>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::in_place_scope(|scope| {
            assert_eq!(rayon_core::current_thread_index(), None);

            for input in 0usize..task_count {
                let started_ref = &started;
                let completed_ref = &completed;

                rayon_core::Scope::spawn(scope, move |_| {
                    started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::current_thread_index()
                        .expect("queued work should run on a Rayon worker");
                    assert!(worker_index < global_threads);
                    assert_eq!(rayon_core::current_num_threads(), global_threads);

                    let (square, cube) =
                        rayon_core::join(move || input * input, move || input * input * input);

                    completed_ref
                        .lock()
                        .expect("completed record mutex should not be poisoned")
                        .push(BodyPanicQueuedRecord {
                            input,
                            worker_index,
                            num_threads: rayon_core::current_num_threads(),
                            value: square + cube,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });
                });
            }

            panic!("intentional rayon_core::in_place_scope body panic after scheduling work");
        });
    }));

    let payload = panic_result
        .expect_err("a panic in the in_place_scope body should propagate to the caller");
    let panic_message = panic_payload_to_string(&*payload);
    assert!(
        panic_message.contains("in_place_scope body panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );

    assert_eq!(
        started.load(Ordering::SeqCst),
        task_count,
        "in_place_scope should wait for all work spawned before the body panic"
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
            "queued work should observe worker-local pending-task status"
        );
    }

    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "after unwinding, the caller should still not be a Rayon worker"
    );

    let completed_by_input: BTreeMap<usize, BodyPanicQueuedRecord> = completed
        .iter()
        .cloned()
        .map(|record| (record.input, record))
        .collect();
    let expected_completed_sum: usize = completed.iter().map(|record| record.value).sum();

    let recovery_records = Mutex::new(Vec::<BodyPanicRecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::in_place_scope(|scope| {
        assert_eq!(rayon_core::current_thread_index(), None);

        for record in completed.iter().cloned() {
            let recovery_records_ref = &recovery_records;
            let recovery_started_ref = &recovery_started;

            rayon_core::Scope::spawn(scope, move |_| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery work should run on a Rayon worker");
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
            .expect("recovery record should correspond to a queued record");

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

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_in_place_scope_propagates_spawned_task_panic_and_later_accepts_nested_work() {
    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);

    let panic_started = AtomicUsize::new(0);

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::in_place_scope(|scope| {
            let panic_started_ref = &panic_started;

            rayon_core::Scope::spawn(scope, move |_| {
                panic_started_ref.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::current_thread_index()
                    .expect("panicking scoped work should run on a Rayon worker");
                assert!(worker_index < global_threads);
                assert!(
                    rayon_core::current_thread_has_pending_tasks().is_some(),
                    "panicking scoped work should observe worker-local pending-task status"
                );

                panic!("intentional spawned panic from rayon_core::in_place_scope worker {worker_index}");
            });

            123usize
        })
    }));

    let payload = panic_result
        .expect_err("a panic in work spawned by in_place_scope should propagate to the caller");
    let panic_message = panic_payload_to_string(&*payload);
    assert!(
        panic_message.contains("intentional spawned panic from rayon_core::in_place_scope"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);
    assert_eq!(rayon_core::current_thread_index(), None);

    let mut recovery_seeds = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 3) * (num_threads + 23),
        }
    });

    recovery_seeds.sort_by_key(|record| record.index);

    assert_eq!(recovery_seeds.len(), global_threads);
    assert_eq!(
        recovery_seeds
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for record in &recovery_seeds {
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, (record.index + 3) * (global_threads + 23));
    }

    let seed_by_index: Vec<_> = recovery_seeds
        .iter()
        .map(|record| record.seed)
        .collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let recovery_records = Mutex::new(Vec::<SpawnPanicRecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::in_place_scope(|scope| {
        for seed_record in recovery_seeds.iter().cloned() {
            let recovery_records_ref = &recovery_records;
            let recovery_started_ref = &recovery_started;

            rayon_core::Scope::spawn(scope, move |nested_scope| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let parent_executing_index = rayon_core::current_thread_index()
                    .expect("recovery parent work should run on a Rayon worker");
                assert!(parent_executing_index < global_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || parent_executing_index + global_threads,
                );
                let parent_value = left + right;

                recovery_records_ref
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(SpawnPanicRecoveryRecord {
                        stage: 0,
                        origin_index,
                        seed,
                        executing_index: parent_executing_index,
                        value: parent_value,
                    });

                rayon_core::Scope::spawn(nested_scope, move |_| {
                    recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index = rayon_core::current_thread_index()
                        .expect("recovery child work should run on a Rayon worker");
                    assert!(child_executing_index < global_threads);

                    let (from_parent, from_child_worker) =
                        rayon_core::join(move || parent_value, move || child_executing_index);

                    recovery_records_ref
                        .lock()
                        .expect("recovery record mutex should not be poisoned")
                        .push(SpawnPanicRecoveryRecord {
                            stage: 1,
                            origin_index,
                            seed,
                            executing_index: child_executing_index,
                            value: from_parent + from_child_worker,
                        });
                });
            });
        }

        expected_seed_sum
    });

    assert_eq!(recovery_return, expected_seed_sum);
    assert_eq!(recovery_started.load(Ordering::SeqCst), global_threads * 2);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| (record.stage, record.origin_index));

    assert_eq!(recovery_records.len(), global_threads * 2);

    let parent_records: Vec<_> = recovery_records
        .iter()
        .filter(|record| record.stage == 0)
        .cloned()
        .collect();
    let child_records: Vec<_> = recovery_records
        .iter()
        .filter(|record| record.stage == 1)
        .cloned()
        .collect();

    assert_eq!(parent_records.len(), global_threads);
    assert_eq!(child_records.len(), global_threads);

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );
    assert_eq!(
        child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for record in &parent_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.value,
            record.seed + record.origin_index + record.executing_index + global_threads
        );
    }

    let parent_by_origin: BTreeMap<usize, SpawnPanicRecoveryRecord> = parent_records
        .into_iter()
        .map(|record| (record.origin_index, record))
        .collect();

    for record in &child_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("child recovery record should correspond to a parent record");

        assert_eq!(record.seed, parent.seed);
        assert_eq!(record.value, parent.value + record.executing_index);
    }

    let (observed_child_sum, recomputed_child_sum) = rayon_core::join(
        || child_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            child_records
                .iter()
                .map(|record| {
                    let parent = parent_by_origin
                        .get(&record.origin_index)
                        .expect("parent record should exist during recomputation");

                    parent.value + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_child_sum, recomputed_child_sum);
}