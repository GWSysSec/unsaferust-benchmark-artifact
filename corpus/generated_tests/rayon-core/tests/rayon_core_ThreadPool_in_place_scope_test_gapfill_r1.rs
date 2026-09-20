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
struct ParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    pool_threads: usize,
    joined_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    nested_value: usize,
}

#[derive(Clone, Debug)]
struct LocalOutcome {
    trace: Rc<RefCell<Vec<String>>>,
    seed_checksum: usize,
    spawned_jobs: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
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
fn thread_pool_in_place_scope_runs_caller_body_and_waits_for_nested_pool_work() {
    let thread_count = 4usize;
    let caller_thread = std::thread::current().id();

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("in-place-scope-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    let pool_ref = &pool;

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(pool_ref),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "the integration-test thread should not be a worker in this pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
        None,
        "pending-task status is unavailable outside the custom pool"
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_num_threads(), thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 1) * (num_threads + 13),
        }
    });

    seeds.sort_by_key(|record| record.index);

    let expected_indices = expected_worker_indices(thread_count);
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
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 13));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_parent_sum: usize = seeds
        .iter()
        .map(|record| record.seed + record.index + record.num_threads)
        .sum();

    let parent_records = Mutex::new(Vec::<ParentRecord>::new());
    let child_records = Mutex::new(Vec::<ChildRecord>::new());
    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);

    let local_trace = Rc::new(RefCell::new(Vec::<String>::new()));

    let outcome = rayon_core::ThreadPool::in_place_scope(pool_ref, |scope| {
        assert_eq!(
            std::thread::current().id(),
            caller_thread,
            "ThreadPool::in_place_scope should execute its body on the calling thread"
        );
        assert_eq!(
            rayon_core::ThreadPool::current_thread_index(pool_ref),
            None,
            "the in-place body itself should not be migrated to a pool worker"
        );
        assert_eq!(
            rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref),
            None
        );

        local_trace
            .borrow_mut()
            .push(format!("body-start-{}", seeds.len()));

        for record in seeds.iter().cloned() {
            let origin_index = record.index;
            let seed = record.seed;
            let seed_threads = record.num_threads;

            let parent_records = &parent_records;
            let child_records = &child_records;
            let parent_started = &parent_started;
            let child_started = &child_started;

            rayon_core::Scope::spawn(scope, move |nested_scope| {
                parent_started.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("scoped work spawned by ThreadPool::in_place_scope should run in the pool");
                assert!(executing_index < thread_count);
                assert_eq!(
                    rayon_core::ThreadPool::current_num_threads(pool_ref),
                    thread_count
                );

                let pending_status_available =
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some();

                let (left, right) =
                    rayon_core::join(move || seed + origin_index, move || seed_threads);
                let joined_value = left + right;

                parent_records
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(ParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        pool_threads: rayon_core::ThreadPool::current_num_threads(pool_ref),
                        joined_value,
                        pending_status_available,
                    });

                rayon_core::Scope::spawn(nested_scope, move |_| {
                    child_started.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index =
                        rayon_core::ThreadPool::current_thread_index(pool_ref).expect(
                            "nested scoped work should also run in the custom thread pool",
                        );
                    assert!(child_executing_index < thread_count);

                    let (from_parent, from_child_worker) =
                        rayon_core::join(move || joined_value * 2, move || child_executing_index);

                    child_records
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(ChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            nested_value: from_parent + from_child_worker,
                        });
                });
            });
        }

        local_trace
            .borrow_mut()
            .push(format!("body-spawned-{expected_parent_sum}"));

        LocalOutcome {
            trace: Rc::clone(&local_trace),
            seed_checksum: expected_parent_sum,
            spawned_jobs: seeds.len(),
        }
    });

    assert!(
        Rc::ptr_eq(&outcome.trace, &local_trace),
        "ThreadPool::in_place_scope should be able to return non-Send caller-local data"
    );
    assert_eq!(outcome.seed_checksum, expected_parent_sum);
    assert_eq!(outcome.spawned_jobs, thread_count);
    assert_eq!(
        outcome.trace.borrow().clone(),
        vec![
            format!("body-start-{thread_count}"),
            format!("body-spawned-{expected_parent_sum}")
        ]
    );

    assert_eq!(parent_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(child_started.load(Ordering::SeqCst), thread_count);

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
        assert_eq!(record.pool_threads, thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.joined_value,
            record.seed + record.origin_index + thread_count
        );
        assert!(
            record.pending_status_available,
            "pool worker should be able to report pending-task status"
        );
    }

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.joined_value)
            .sum::<usize>(),
        expected_parent_sum
    );

    let parent_by_origin: BTreeMap<usize, ParentRecord> = parent_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(parent_by_origin.len(), thread_count);

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
            .expect("child record should correspond to a parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(
            record.nested_value,
            parent.joined_value * 2 + record.executing_index
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_in_place_scope_propagates_scoped_panics_and_pool_recovers() {
    let thread_count = 3usize;

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("in-place-scope-recovery-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    let pool_ref = &pool;

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(pool_ref),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(pool_ref), None);

    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<(usize, usize)>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::ThreadPool::in_place_scope(pool_ref, |scope| {
            for value in 0..(thread_count * 2) {
                let completed_before_panic = &completed_before_panic;
                let sibling_started = &sibling_started;

                rayon_core::Scope::spawn(scope, move |_| {
                    sibling_started.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                        .expect("sibling scoped work should run in the custom pool");
                    assert!(worker_index < thread_count);

                    completed_before_panic
                        .lock()
                        .expect("sibling record mutex should not be poisoned")
                        .push((value, worker_index));
                });
            }

            let panic_started = &panic_started;
            rayon_core::Scope::spawn(scope, move |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("panicking scoped work should run in the custom pool");
                assert!(worker_index < thread_count);
                assert!(
                    rayon_core::ThreadPool::current_thread_has_pending_tasks(pool_ref).is_some()
                );

                panic!("intentional ThreadPool::in_place_scope scoped panic");
            });

            777usize
        })
    }));

    let payload = panic_result
        .expect_err("a panic in scoped work should propagate out of ThreadPool::in_place_scope");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional ThreadPool::in_place_scope scoped panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);

    let sibling_count = sibling_started.load(Ordering::SeqCst);
    assert!(sibling_count <= thread_count * 2);

    let completed_before_panic = completed_before_panic
        .into_inner()
        .expect("sibling record mutex should not be poisoned");

    assert_eq!(completed_before_panic.len(), sibling_count);
    assert!(
        completed_before_panic
            .iter()
            .all(|(value, worker_index)| *value < thread_count * 2 && *worker_index < thread_count)
    );

    let completed_values: BTreeSet<_> = completed_before_panic
        .iter()
        .map(|(value, _)| *value)
        .collect();
    assert_eq!(
        completed_values.len(),
        completed_before_panic.len(),
        "each non-panicking sibling task should report at most once"
    );

    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(pool_ref),
        None,
        "after unwinding, the caller should still not be a pool worker"
    );

    let mut recovery_seeds = rayon_core::ThreadPool::broadcast(pool_ref, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, (index + 5) * (num_threads + 29))
    });

    recovery_seeds.sort_by_key(|entry| entry.0);

    let expected_indices = expected_worker_indices(thread_count);
    assert_eq!(recovery_seeds.len(), thread_count);
    assert_eq!(
        recovery_seeds
            .iter()
            .map(|(index, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let expected_seed_sum: usize = recovery_seeds.iter().map(|(_, seed)| *seed).sum();
    let seed_by_origin: BTreeMap<usize, usize> = recovery_seeds.iter().copied().collect();

    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());
    let recovery_started = AtomicUsize::new(0);

    let recovery_return = rayon_core::ThreadPool::in_place_scope(pool_ref, |scope| {
        for (origin_index, seed) in recovery_seeds.iter().copied() {
            let recovery_records = &recovery_records;
            let recovery_started = &recovery_started;

            rayon_core::Scope::spawn(scope, move |nested_scope| {
                recovery_started.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::ThreadPool::current_thread_index(pool_ref)
                    .expect("recovery parent work should run in the custom pool");
                assert!(executing_index < thread_count);

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || executing_index + thread_count,
                );
                let parent_value = left + right;

                recovery_records
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        stage: 0,
                        origin_index,
                        seed,
                        executing_index,
                        value: parent_value,
                    });

                rayon_core::Scope::spawn(nested_scope, move |_| {
                    recovery_started.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index =
                        rayon_core::ThreadPool::current_thread_index(pool_ref)
                            .expect("recovery child work should run in the custom pool");
                    assert!(child_executing_index < thread_count);

                    let (from_parent, from_child_worker) =
                        rayon_core::join(move || parent_value, move || child_executing_index);

                    recovery_records
                        .lock()
                        .expect("recovery record mutex should not be poisoned")
                        .push(RecoveryRecord {
                            stage: 1,
                            origin_index,
                            seed,
                            executing_index: child_executing_index,
                            value: from_parent + from_child_worker,
                        });
                });
            });
        }

        expected_seed_sum + sibling_count
    });

    assert_eq!(recovery_return, expected_seed_sum + sibling_count);
    assert_eq!(recovery_started.load(Ordering::SeqCst), thread_count * 2);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| (record.stage, record.origin_index));

    assert_eq!(recovery_records.len(), thread_count * 2);

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

    assert_eq!(parent_records.len(), thread_count);
    assert_eq!(child_records.len(), thread_count);

    assert_eq!(
        parent_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );
    assert_eq!(
        child_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &parent_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );
        assert_eq!(
            record.value,
            record.seed + record.origin_index + record.executing_index + thread_count
        );
    }

    let parent_by_origin: BTreeMap<usize, RecoveryRecord> = parent_records
        .into_iter()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(parent_by_origin.len(), thread_count);

    for record in &child_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("recovery child record should have a parent record");

        assert_eq!(record.seed, parent.seed);
        assert_eq!(record.value, parent.value + record.executing_index);
    }
}