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
    num_threads: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    seed: usize,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BroadcastChildRecord {
    origin_index: usize,
    executing_index: usize,
    value: usize,
}

#[derive(Clone, Debug)]
struct ModuleInPlaceOutcome {
    trace: Rc<RefCell<Vec<String>>>,
    scheduled_parent_jobs: usize,
    scheduled_broadcast_jobs: usize,
    expected_parent_sum: usize,
    expected_broadcast_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CurrentPoolSummary {
    body_index: usize,
    body_name: String,
    body_threads: usize,
    trace_snapshot: Vec<String>,
    scheduled_jobs: usize,
    scheduled_broadcast_jobs: usize,
    seed_sum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CurrentPoolRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    worker_name: String,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CurrentPoolBroadcastRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: String,
    seed: usize,
    value: usize,
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
fn module_path_in_place_scope_runs_body_on_external_caller_and_drives_nested_broadcast_pipeline() {
    let caller_thread = std::thread::current().id();
    let global_threads = rayon_core::current_num_threads();

    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "the integration-test thread should start outside Rayon"
    );
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
            seed: (index + 1) * (num_threads + 31),
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
        assert_eq!(record.seed, (expected_index + 1) * (global_threads + 31));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_parent_sum: usize = seeds
        .iter()
        .map(|record| record.seed + record.index + record.num_threads * 2)
        .sum();
    let expected_broadcast_sum: usize = seed_by_index
        .iter()
        .enumerate()
        .map(|(index, seed)| *seed * 3 + index + global_threads * 5)
        .sum();

    let parent_records = Mutex::new(Vec::<ParentRecord>::new());
    let child_records = Mutex::new(Vec::<ChildRecord>::new());
    let broadcast_records = Mutex::new(Vec::<BroadcastRecord>::new());
    let broadcast_child_records = Mutex::new(Vec::<BroadcastChildRecord>::new());

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
                    .expect("parent work spawned by in_place_scope should run on Rayon");
                assert!(executing_index < global_threads);
                assert_eq!(rayon_core::current_num_threads(), global_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (left, right) =
                    rayon_core::join(move || seed + origin_index, move || num_threads * 2);
                let parent_value = left + right;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(ParentRecord {
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

                    let (parent_component, worker_component) =
                        rayon_core::join(move || parent_value * 2, move || child_executing_index);

                    child_records_ref
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(ChildRecord {
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
                .push(BroadcastRecord {
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
                    .push(BroadcastChildRecord {
                        origin_index: index,
                        executing_index,
                        value: broadcast_value + num_threads + executing_index,
                    });
            });
        });

        local_trace.borrow_mut().push(format!(
            "body-queued-parent-sum-{expected_parent_sum}-broadcast-sum-{expected_broadcast_sum}"
        ));

        ModuleInPlaceOutcome {
            trace: Rc::clone(&local_trace),
            scheduled_parent_jobs: seeds.len(),
            scheduled_broadcast_jobs: global_threads,
            expected_parent_sum,
            expected_broadcast_sum,
        }
    });

    assert!(Rc::ptr_eq(&outcome.trace, &local_trace));
    assert_eq!(outcome.scheduled_parent_jobs, global_threads);
    assert_eq!(outcome.scheduled_broadcast_jobs, global_threads);
    assert_eq!(outcome.expected_parent_sum, expected_parent_sum);
    assert_eq!(outcome.expected_broadcast_sum, expected_broadcast_sum);
    assert_eq!(
        outcome.trace.borrow().clone(),
        vec![
            format!("body-started-with-{global_threads}-seeds"),
            format!("body-queued-parent-sum-{expected_parent_sum}-broadcast-sum-{expected_broadcast_sum}")
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
            record.seed + record.origin_index + global_threads * 2
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

    let parent_by_origin: BTreeMap<usize, ParentRecord> = parent_records
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
            "broadcast work spawned from in_place_scope should observe pending-task status"
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
fn module_path_in_place_scope_called_from_custom_pool_worker_uses_current_pool() {
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("module-path-in-place-current-pool-worker-{index}"))
        .build()
        .expect("custom Rayon pool should build");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: (index + 2) * (num_threads + 43),
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
        assert_eq!(record.seed, (record.index + 2) * (thread_count + 43));
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let current_pool_records = Mutex::new(Vec::<CurrentPoolRecord>::new());
    let current_pool_broadcast_records = Mutex::new(Vec::<CurrentPoolBroadcastRecord>::new());
    let current_pool_started = AtomicUsize::new(0);
    let current_pool_broadcast_started = AtomicUsize::new(0);

    let summary = rayon_core::ThreadPool::scope(&pool, |_| {
        let outer_index = rayon_core::current_thread_index()
            .expect("ThreadPool::scope body should run on a custom-pool worker");
        assert!(outer_index < thread_count);

        let outer_name = std::thread::current()
            .name()
            .map(str::to_owned)
            .expect("custom-pool worker should have a configured name");
        assert_eq!(
            outer_name,
            format!("module-path-in-place-current-pool-worker-{outer_index}")
        );

        let local_trace = Rc::new(RefCell::new(Vec::<String>::new()));

        rayon_core::in_place_scope(|scope| {
            assert_eq!(
                rayon_core::current_thread_index(),
                Some(outer_index),
                "in_place_scope called from a worker should keep the body on that worker"
            );
            assert_eq!(rayon_core::current_num_threads(), thread_count);
            assert!(
                rayon_core::current_thread_has_pending_tasks().is_some(),
                "a worker-local in-place body should be able to query pending-task status"
            );

            let body_name = std::thread::current()
                .name()
                .map(str::to_owned)
                .expect("in-place body should still run on the named custom-pool worker");
            assert_eq!(body_name, outer_name);

            local_trace
                .borrow_mut()
                .push(format!("body-on-worker-{outer_index}"));

            for seed_record in seeds.iter().cloned() {
                let current_pool_records_ref = &current_pool_records;
                let current_pool_started_ref = &current_pool_started;

                rayon_core::Scope::spawn(scope, move |_| {
                    current_pool_started_ref.fetch_add(1, Ordering::SeqCst);

                    let executing_index = rayon_core::current_thread_index()
                        .expect("spawned in-place work should run in the current custom pool");
                    assert!(executing_index < thread_count);
                    assert_eq!(rayon_core::current_num_threads(), thread_count);

                    let worker_name = std::thread::current()
                        .name()
                        .map(str::to_owned)
                        .expect("custom-pool worker should have a configured name");
                    assert_eq!(
                        worker_name,
                        format!("module-path-in-place-current-pool-worker-{executing_index}")
                    );

                    let origin_index = seed_record.index;
                    let seed = seed_record.seed;

                    let (left, right) = rayon_core::join(
                        move || seed + origin_index,
                        move || executing_index + thread_count,
                    );

                    current_pool_records_ref
                        .lock()
                        .expect("current-pool record mutex should not be poisoned")
                        .push(CurrentPoolRecord {
                            origin_index,
                            seed,
                            executing_index,
                            num_threads: rayon_core::current_num_threads(),
                            worker_name,
                            value: left + right,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });
                });
            }

            let seed_by_index_ref = &seed_by_index;
            let current_pool_broadcast_records_ref = &current_pool_broadcast_records;
            let current_pool_broadcast_started_ref = &current_pool_broadcast_started;

            rayon_core::Scope::spawn_broadcast(scope, move |_, context| {
                current_pool_broadcast_started_ref.fetch_add(1, Ordering::SeqCst);

                let index = rayon_core::BroadcastContext::index(&context);
                let num_threads = rayon_core::BroadcastContext::num_threads(&context);

                assert_eq!(num_threads, thread_count);
                assert_eq!(rayon_core::current_thread_index(), Some(index));
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let worker_name = std::thread::current()
                    .name()
                    .map(str::to_owned)
                    .expect("broadcast worker should have a configured name");
                assert_eq!(
                    worker_name,
                    format!("module-path-in-place-current-pool-worker-{index}")
                );

                let seed = seed_by_index_ref[index];
                let (left, right) =
                    rayon_core::join(move || seed * 2, move || index + num_threads * 10);

                current_pool_broadcast_records_ref
                    .lock()
                    .expect("current-pool broadcast mutex should not be poisoned")
                    .push(CurrentPoolBroadcastRecord {
                        index,
                        num_threads,
                        current_index: rayon_core::current_thread_index(),
                        worker_name,
                        seed,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });

            local_trace
                .borrow_mut()
                .push(format!("queued-{}-seeded-jobs", seeds.len()));

            let trace_snapshot = local_trace.borrow().clone();

            CurrentPoolSummary {
                body_index: outer_index,
                body_name,
                body_threads: rayon_core::current_num_threads(),
                trace_snapshot,
                scheduled_jobs: seeds.len(),
                scheduled_broadcast_jobs: thread_count,
                seed_sum: expected_seed_sum,
            }
        })
    });

    assert!(summary.body_index < thread_count);
    assert_eq!(
        summary.body_name,
        format!(
            "module-path-in-place-current-pool-worker-{}",
            summary.body_index
        )
    );
    assert_eq!(summary.body_threads, thread_count);
    assert_eq!(summary.scheduled_jobs, thread_count);
    assert_eq!(summary.scheduled_broadcast_jobs, thread_count);
    assert_eq!(summary.seed_sum, expected_seed_sum);
    assert_eq!(
        summary.trace_snapshot,
        vec![
            format!("body-on-worker-{}", summary.body_index),
            format!("queued-{thread_count}-seeded-jobs")
        ]
    );

    assert_eq!(current_pool_started.load(Ordering::SeqCst), thread_count);
    assert_eq!(
        current_pool_broadcast_started.load(Ordering::SeqCst),
        thread_count
    );

    let mut current_pool_records = current_pool_records
        .into_inner()
        .expect("current-pool record mutex should not be poisoned");
    current_pool_records.sort_by_key(|record| record.origin_index);

    assert_eq!(current_pool_records.len(), thread_count);
    assert_eq!(
        current_pool_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &current_pool_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.worker_name,
            format!(
                "module-path-in-place-current-pool-worker-{}",
                record.executing_index
            )
        );
        assert_eq!(
            record.value,
            record.seed + record.origin_index + record.executing_index + thread_count
        );
        assert!(
            record.pending_status_available,
            "current-pool in-place work should observe pending-task status"
        );
    }

    let mut current_pool_broadcast_records = current_pool_broadcast_records
        .into_inner()
        .expect("current-pool broadcast mutex should not be poisoned");
    current_pool_broadcast_records.sort_by_key(|record| record.index);

    assert_eq!(current_pool_broadcast_records.len(), thread_count);
    assert_eq!(
        current_pool_broadcast_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &current_pool_broadcast_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(
            record.worker_name,
            format!("module-path-in-place-current-pool-worker-{}", record.index)
        );
        assert_eq!(
            record.value,
            seed_by_index[record.index] * 2 + record.index + thread_count * 10
        );
        assert!(
            record.pending_status_available,
            "broadcast work spawned from worker-local in_place_scope should observe pending-task status"
        );
    }

    let expected_current_pool_sum: usize =
        current_pool_records.iter().map(|record| record.value).sum();
    let expected_broadcast_sum: usize = current_pool_broadcast_records
        .iter()
        .map(|record| record.value)
        .sum();

    let (observed_current_pool_sum, observed_broadcast_sum) = rayon_core::ThreadPool::join(
        &pool,
        || current_pool_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            current_pool_broadcast_records
                .iter()
                .map(|record| record.value)
                .sum::<usize>()
        },
    );

    assert_eq!(observed_current_pool_sum, expected_current_pool_sum);
    assert_eq!(observed_broadcast_sum, expected_broadcast_sum);
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn module_path_in_place_scope_body_panic_waits_for_prequeued_work_and_later_recovers() {
    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);

    let task_count = (global_threads + 5).min(16);
    let started = AtomicUsize::new(0);
    let completed = Mutex::new(Vec::<QueuedBeforePanicRecord>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        rayon_core::in_place_scope(|scope| {
            assert_eq!(
                rayon_core::current_thread_index(),
                None,
                "the body-panic test should run the in-place body on the external caller"
            );

            for input in 0usize..task_count {
                let started_ref = &started;
                let completed_ref = &completed;

                rayon_core::Scope::spawn(scope, move |_| {
                    started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::current_thread_index()
                        .expect("prequeued work should run on a Rayon worker");
                    assert!(worker_index < global_threads);
                    assert_eq!(rayon_core::current_num_threads(), global_threads);

                    let (square, cube) =
                        rayon_core::join(move || input * input, move || input * input * input);

                    completed_ref
                        .lock()
                        .expect("completed record mutex should not be poisoned")
                        .push(QueuedBeforePanicRecord {
                            input,
                            worker_index,
                            num_threads: rayon_core::current_num_threads(),
                            value: square + cube,
                            pending_status_available:
                                rayon_core::current_thread_has_pending_tasks().is_some(),
                        });
                });
            }

            panic!("intentional rayon_core::in_place_scope body panic after queued work");
        });
    }));

    let payload = panic_result
        .expect_err("a panic in rayon_core::in_place_scope body should propagate to the caller");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("rayon_core::in_place_scope body panic"),
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
            "prequeued work should observe worker-local pending-task status"
        );
    }

    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "after unwinding, the external caller should still not be a Rayon worker"
    );
    assert_eq!(rayon_core::current_thread_has_pending_tasks(), None);

    let completed_by_input: BTreeMap<usize, QueuedBeforePanicRecord> = completed
        .iter()
        .cloned()
        .map(|record| (record.input, record))
        .collect();
    assert_eq!(completed_by_input.len(), task_count);

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
                let value = record.value;
                let original_worker_index = record.worker_index;

                let (left, right) = rayon_core::join(
                    move || value + input,
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