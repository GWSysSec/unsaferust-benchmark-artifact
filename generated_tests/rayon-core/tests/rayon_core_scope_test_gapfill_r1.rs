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
struct ParentRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    num_threads: usize,
    joined_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChildRecord {
    origin_index: usize,
    parent_executing_index: usize,
    executing_index: usize,
    inherited_value: usize,
    child_value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfirmationRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryRecord {
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

fn scope_based_join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    let mut result_a = None;
    let mut result_b = None;

    rayon_core::scope(|scope| {
        rayon_core::Scope::spawn(scope, |_| {
            result_a = Some(oper_a());
        });

        rayon_core::Scope::spawn(scope, |_| {
            result_b = Some(oper_b());
        });
    });

    (
        result_a.expect("left scoped join branch should complete"),
        result_b.expect("right scoped join branch should complete"),
    )
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_scope_emulates_join_and_feeds_broadcast_seeded_nested_pipeline() {
    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "the integration-test thread should start outside any Rayon worker"
    );

    let (joined_words, (product, scoped_thread_count)) = scope_based_join(
        || {
            let words = ["rayon", "core", "scope"];
            words.join(":")
        },
        || {
            let product = (1usize..=5).product::<usize>();
            (product, rayon_core::current_num_threads())
        },
    );

    assert_eq!(joined_words, "rayon:core:scope");
    assert_eq!(product, 120);

    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());
    assert_eq!(scoped_thread_count, global_threads);

    let base = joined_words.len() + product;

    let mut seeds = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert!(index < num_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), global_threads);

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            seed: base + (index + 1) * (num_threads + 13),
        }
    });

    seeds.sort_by_key(|record| record.index);

    assert_eq!(seeds.len(), global_threads);
    assert_eq!(
        seeds
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for (expected_index, record) in seeds.iter().enumerate() {
        assert_eq!(record.index, expected_index);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(expected_index));
        assert_eq!(
            record.seed,
            base + (expected_index + 1) * (global_threads + 13)
        );
    }

    let seed_by_index: Vec<_> = seeds.iter().map(|record| record.seed).collect();
    let expected_seed_sum: usize = seed_by_index.iter().sum();

    let parent_records = Mutex::new(Vec::<ParentRecord>::new());
    let child_records = Mutex::new(Vec::<ChildRecord>::new());
    let parent_started = AtomicUsize::new(0);
    let child_started = AtomicUsize::new(0);

    let scope_return = rayon_core::scope(|scope| {
        for seed_record in seeds.iter().cloned() {
            let parent_records_ref = &parent_records;
            let child_records_ref = &child_records;
            let parent_started_ref = &parent_started;
            let child_started_ref = &child_started;

            rayon_core::Scope::spawn(scope, move |nested_scope| {
                parent_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("parent scoped work should run on a Rayon worker");
                assert!(executing_index < seed_record.num_threads);
                assert_eq!(rayon_core::current_num_threads(), seed_record.num_threads);

                let origin_index = seed_record.index;
                let seed = seed_record.seed;
                let num_threads = seed_record.num_threads;

                let (seed_component, worker_component) = rayon_core::join(
                    move || seed + origin_index,
                    move || num_threads + executing_index,
                );
                let joined_value = seed_component + worker_component;

                parent_records_ref
                    .lock()
                    .expect("parent record mutex should not be poisoned")
                    .push(ParentRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        joined_value,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });

                rayon_core::Scope::spawn(nested_scope, move |_| {
                    child_started_ref.fetch_add(1, Ordering::SeqCst);

                    let child_executing_index = rayon_core::current_thread_index()
                        .expect("nested child scoped work should run on a Rayon worker");
                    assert!(child_executing_index < num_threads);

                    let (doubled_parent, worker_component) =
                        rayon_core::join(move || joined_value * 2, move || child_executing_index);

                    child_records_ref
                        .lock()
                        .expect("child record mutex should not be poisoned")
                        .push(ChildRecord {
                            origin_index,
                            parent_executing_index: executing_index,
                            executing_index: child_executing_index,
                            inherited_value: joined_value,
                            child_value: doubled_parent + worker_component,
                        });
                });
            });
        }

        expected_seed_sum
    });

    assert_eq!(scope_return, expected_seed_sum);
    assert_eq!(parent_started.load(Ordering::SeqCst), global_threads);
    assert_eq!(child_started.load(Ordering::SeqCst), global_threads);

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
        expected_worker_indices(global_threads)
    );

    for record in &parent_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.seed, seed_by_index[record.origin_index]);
        assert_eq!(
            record.joined_value,
            record.seed + record.origin_index + global_threads + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "scoped parent work should observe worker-local pending-task status"
        );
    }

    let parent_by_origin: BTreeMap<usize, ParentRecord> = parent_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(parent_by_origin.len(), global_threads);

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
        expected_worker_indices(global_threads)
    );

    for record in &child_records {
        assert!(record.origin_index < global_threads);
        assert!(record.parent_executing_index < global_threads);
        assert!(record.executing_index < global_threads);

        let parent = parent_by_origin
            .get(&record.origin_index)
            .expect("child record should correspond to a parent record");

        assert_eq!(record.parent_executing_index, parent.executing_index);
        assert_eq!(record.inherited_value, parent.joined_value);
        assert_eq!(
            record.child_value,
            parent.joined_value * 2 + record.executing_index
        );
    }

    let child_by_origin: BTreeMap<usize, ChildRecord> = child_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();
    assert_eq!(child_by_origin.len(), global_threads);

    let mut confirmations = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        let parent = parent_by_origin
            .get(&index)
            .expect("confirmation broadcast should find parent output");
        let child = child_by_origin
            .get(&index)
            .expect("confirmation broadcast should find child output");

        let parent_value = parent.joined_value;
        let child_value = child.child_value;
        let (left, right) = rayon_core::join(move || parent_value, move || child_value);

        ConfirmationRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            total: left + right,
        }
    });

    confirmations.sort_by_key(|record| record.index);

    assert_eq!(confirmations.len(), global_threads);
    assert_eq!(
        confirmations
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for record in &confirmations {
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(
            record.total,
            parent_by_origin[&record.index].joined_value
                + child_by_origin[&record.index].child_value
        );
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn free_scope_propagates_spawned_panic_and_later_scopes_reuse_global_pool() {
    let global_threads = rayon_core::current_num_threads();
    assert!(global_threads > 0);

    let task_count = (global_threads + 4).min(16);
    let panic_started = AtomicUsize::new(0);
    let sibling_started = AtomicUsize::new(0);
    let completed_before_panic = Mutex::new(Vec::<(usize, usize)>::new());

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let _: usize = rayon_core::scope(|scope| {
            rayon_core::Scope::spawn(scope, |_| {
                panic_started.fetch_add(1, Ordering::SeqCst);

                let worker_index = rayon_core::current_thread_index()
                    .expect("panicking scoped work should run on a Rayon worker");
                assert!(worker_index < global_threads);
                assert!(
                    rayon_core::current_thread_has_pending_tasks().is_some(),
                    "panicking scoped work should observe worker-local pending-task status"
                );

                panic!("intentional rayon_core::scope spawned panic for integration coverage");
            });

            for input in 0usize..task_count {
                let sibling_started_ref = &sibling_started;
                let completed_ref = &completed_before_panic;

                rayon_core::Scope::spawn(scope, move |_| {
                    sibling_started_ref.fetch_add(1, Ordering::SeqCst);

                    let worker_index = rayon_core::current_thread_index()
                        .expect("sibling scoped work should run on a Rayon worker");
                    assert!(worker_index < global_threads);

                    completed_ref
                        .lock()
                        .expect("completed sibling mutex should not be poisoned")
                        .push((input, worker_index));
                });
            }

            777usize
        });
    }));

    let payload = panic_result.expect_err("panic in scoped work should propagate out of scope");
    let panic_message = panic_payload_to_string(&*payload);

    assert!(
        panic_message.contains("intentional rayon_core::scope spawned panic"),
        "unexpected propagated panic payload: {panic_message:?}"
    );
    assert_eq!(panic_started.load(Ordering::SeqCst), 1);

    let sibling_count_before_recovery = sibling_started.load(Ordering::SeqCst);
    assert!(sibling_count_before_recovery <= task_count);

    let completed_before_panic = completed_before_panic
        .into_inner()
        .expect("completed sibling mutex should not be poisoned");

    assert_eq!(completed_before_panic.len(), sibling_count_before_recovery);
    assert!(
        completed_before_panic
            .iter()
            .all(|(input, worker_index)| *input < task_count && *worker_index < global_threads)
    );
    assert_eq!(
        completed_before_panic
            .iter()
            .map(|(input, _)| *input)
            .collect::<BTreeSet<_>>()
            .len(),
        completed_before_panic.len(),
        "each completed sibling should report at most once"
    );

    assert_eq!(
        rayon_core::current_thread_index(),
        None,
        "after unwinding, the external caller should still not be a Rayon worker"
    );

    let mut recovery_seeds = rayon_core::broadcast(|context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, global_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, num_threads, (index + 3) * (num_threads + 29))
    });

    recovery_seeds.sort_by_key(|entry| entry.0);

    assert_eq!(recovery_seeds.len(), global_threads);
    assert_eq!(
        recovery_seeds
            .iter()
            .map(|(index, _, _)| *index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for (index, num_threads, seed) in &recovery_seeds {
        assert_eq!(*num_threads, global_threads);
        assert_eq!(*seed, (*index + 3) * (global_threads + 29));
    }

    let seed_by_origin: BTreeMap<usize, usize> = recovery_seeds
        .iter()
        .map(|(index, _, seed)| (*index, *seed))
        .collect();
    assert_eq!(seed_by_origin.len(), global_threads);

    let expected_seed_sum: usize = recovery_seeds.iter().map(|(_, _, seed)| *seed).sum();
    let recovery_started = AtomicUsize::new(0);
    let recovery_records = Mutex::new(Vec::<RecoveryRecord>::new());

    let recovery_return = rayon_core::scope(|scope| {
        for (origin_index, num_threads, seed) in recovery_seeds.iter().copied() {
            let recovery_started_ref = &recovery_started;
            let recovery_records_ref = &recovery_records;

            rayon_core::Scope::spawn(scope, move |_| {
                recovery_started_ref.fetch_add(1, Ordering::SeqCst);

                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery scoped work should run on a Rayon worker");
                assert!(executing_index < num_threads);
                assert_eq!(rayon_core::current_num_threads(), num_threads);

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || num_threads + executing_index,
                );

                recovery_records_ref
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryRecord {
                        origin_index,
                        seed,
                        executing_index,
                        num_threads,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        expected_seed_sum + completed_before_panic.len()
    });

    assert_eq!(
        recovery_return,
        expected_seed_sum + completed_before_panic.len()
    );
    assert_eq!(recovery_started.load(Ordering::SeqCst), global_threads);

    let mut recovery_records = recovery_records
        .into_inner()
        .expect("recovery record mutex should not be poisoned");
    recovery_records.sort_by_key(|record| record.origin_index);

    assert_eq!(recovery_records.len(), global_threads);
    assert_eq!(
        recovery_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_worker_indices(global_threads)
    );

    for record in &recovery_records {
        assert!(record.origin_index < global_threads);
        assert!(record.executing_index < global_threads);
        assert_eq!(record.num_threads, global_threads);
        assert_eq!(seed_by_origin.get(&record.origin_index), Some(&record.seed));
        assert_eq!(
            record.value,
            record.seed + record.origin_index + global_threads + record.executing_index
        );
        assert!(
            record.pending_status_available,
            "recovery scoped work should observe worker-local pending-task status"
        );
    }

    let (observed_sum, recomputed_sum) = rayon_core::join(
        || recovery_records.iter().map(|record| record.value).sum::<usize>(),
        || {
            recovery_records
                .iter()
                .map(|record| {
                    seed_by_origin[&record.origin_index]
                        + record.origin_index
                        + global_threads
                        + record.executing_index
                })
                .sum::<usize>()
        },
    );

    assert_eq!(observed_sum, recomputed_sum);
}