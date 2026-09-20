use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpawnHandlerEvent {
    index: usize,
    name: Option<String>,
    stack_size: Option<usize>,
    handler_thread_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartEvent {
    index: usize,
    name: Option<String>,
    current_index: Option<usize>,
    current_threads: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: String,
    seed: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    executing_name: String,
    value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AsyncRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    worker_name: String,
    seed: usize,
    scoped_value: usize,
    async_value: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExitEvent {
    index: usize,
    name: Option<String>,
    observed_marker: usize,
}

fn recv_exact<T>(receiver: &mpsc::Receiver<T>, count: usize, label: &str) -> Vec<T> {
    (0..count)
        .map(|index| {
            receiver
                .recv_timeout(Duration::from_secs(5))
                .unwrap_or_else(|error| {
                    panic!(
                        "{label} did not produce item {} of {} in time: {error}",
                        index + 1,
                        count
                    )
                })
        })
        .collect()
}

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_spawn_handler_threadbuilder_metadata_feeds_lifecycle_broadcast_scope_and_async_work(
) {
    let thread_count = 3usize;
    let stack_size = 2 * 1024 * 1024usize;
    let expected_indices = expected_worker_indices(thread_count);

    let lifecycle_marker = Arc::new(AtomicUsize::new(0));

    let (spawn_tx, spawn_rx) = mpsc::channel::<SpawnHandlerEvent>();
    let (start_tx, start_rx) = mpsc::channel::<StartEvent>();
    let (exit_tx, exit_rx) = mpsc::channel::<ExitEvent>();

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("spawn-handler-lifecycle-worker-{index}"))
        .stack_size(stack_size)
        .start_handler(move |index| {
            start_tx
                .send(StartEvent {
                    index,
                    name: std::thread::current().name().map(str::to_owned),
                    current_index: rayon_core::current_thread_index(),
                    current_threads: rayon_core::current_num_threads(),
                })
                .expect("start_handler should report worker startup");
        })
        .exit_handler({
            let lifecycle_marker = Arc::clone(&lifecycle_marker);

            move |index| {
                exit_tx
                    .send(ExitEvent {
                        index,
                        name: std::thread::current().name().map(str::to_owned),
                        observed_marker: lifecycle_marker.load(Ordering::SeqCst),
                    })
                    .expect("exit_handler should report worker shutdown");
            }
        });

    let builder = rayon_core::ThreadPoolBuilder::spawn_handler(builder, move |thread| {
        let index = rayon_core::ThreadBuilder::index(&thread);
        let name = rayon_core::ThreadBuilder::name(&thread).map(str::to_owned);
        let stack_size = rayon_core::ThreadBuilder::stack_size(&thread);

        spawn_tx
            .send(SpawnHandlerEvent {
                index,
                name: name.clone(),
                stack_size,
                handler_thread_index: rayon_core::current_thread_index(),
            })
            .expect("spawn_handler should report ThreadBuilder metadata");

        let mut std_builder = std::thread::Builder::new();

        if let Some(name) = name {
            std_builder = std_builder.name(name);
        }

        if let Some(stack_size) = stack_size {
            std_builder = std_builder.stack_size(stack_size);
        }

        std_builder
            .spawn(move || rayon_core::ThreadBuilder::run(thread))
            .map(|_| ())
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("custom spawn_handler should successfully build the pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut spawn_events = recv_exact(&spawn_rx, thread_count, "spawn_handler");
    spawn_events.sort_by_key(|event| event.index);

    assert_eq!(
        spawn_events
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let mut expected_names = BTreeMap::<usize, String>::new();

    for event in &spawn_events {
        let expected_name = format!("spawn-handler-lifecycle-worker-{}", event.index);

        assert_eq!(event.name.as_deref(), Some(expected_name.as_str()));
        assert_eq!(event.stack_size, Some(stack_size));
        assert_eq!(
            event.handler_thread_index, None,
            "spawn_handler itself should run on the external builder thread"
        );

        assert!(
            expected_names
                .insert(event.index, expected_name)
                .is_none(),
            "each worker index should be handed to spawn_handler exactly once"
        );
    }

    assert!(
        spawn_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_handler should be called exactly once per worker"
    );

    let mut start_events = recv_exact(&start_rx, thread_count, "start_handler");
    start_events.sort_by_key(|event| event.index);

    assert_eq!(
        start_events
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &start_events {
        assert_eq!(event.current_index, Some(event.index));
        assert_eq!(event.current_threads, thread_count);
        assert_eq!(event.name.as_ref(), expected_names.get(&event.index));
    }

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "start_handler should run exactly once per custom-spawned worker"
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let worker_name = std::thread::current()
            .name()
            .map(str::to_owned)
            .expect("custom spawn_handler should preserve configured worker names");

        assert_eq!(
            expected_names.get(&index).map(String::as_str),
            Some(worker_name.as_str())
        );

        SeedRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            worker_name,
            seed: (index + 1) * (num_threads + 101),
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
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
        assert_eq!(
            record.worker_name,
            expected_names[&expected_index],
            "broadcast should run on the std thread created by spawn_handler"
        );
        assert_eq!(record.seed, (expected_index + 1) * (thread_count + 101));
        assert!(
            record.pending_status_available,
            "broadcast work should observe worker-local pending-task status"
        );
    }

    let expected_seed_sum: usize = seeds.iter().map(|record| record.seed).sum();
    let scoped_records = std::sync::Mutex::new(Vec::<ScopedRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for seed_record in seeds.iter().cloned() {
            let scoped_records_ref = &scoped_records;
            let expected_names_ref = &expected_names;

            rayon_core::Scope::spawn(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("scoped work should run on a custom-spawned Rayon worker");

                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let executing_name = std::thread::current()
                    .name()
                    .map(str::to_owned)
                    .expect("scoped work should run on a named custom-spawned thread");

                assert_eq!(
                    expected_names_ref.get(&executing_index),
                    Some(&executing_name)
                );

                let origin_index = seed_record.index;
                let seed = seed_record.seed;

                let (left, right) = rayon_core::join(
                    move || seed + origin_index,
                    move || executing_index + thread_count,
                );

                scoped_records_ref
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(ScopedRecord {
                        origin_index,
                        seed,
                        executing_index,
                        executing_name,
                        value: left + right,
                        pending_status_available:
                            rayon_core::current_thread_has_pending_tasks().is_some(),
                    });
            });
        }

        expected_seed_sum
    });

    assert_eq!(scope_return, expected_seed_sum);

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
        assert_eq!(record.seed, seeds[record.origin_index].seed);
        assert_eq!(
            expected_names.get(&record.executing_index),
            Some(&record.executing_name)
        );
        assert_eq!(
            record.value,
            record.seed + record.origin_index + record.executing_index + thread_count
        );
        assert!(
            record.pending_status_available,
            "scoped work should observe pending-task status on custom-spawned workers"
        );
    }

    let expected_scoped_sum: usize = scoped_records.iter().map(|record| record.value).sum();
    let scoped_by_origin: Arc<BTreeMap<usize, ScopedRecord>> = Arc::new(
        scoped_records
            .iter()
            .cloned()
            .map(|record| (record.origin_index, record))
            .collect(),
    );
    let seed_by_index = Arc::new(seeds.iter().map(|record| record.seed).collect::<Vec<_>>());
    let expected_names_for_async = Arc::new(expected_names.clone());

    let (async_tx, async_rx) = mpsc::channel::<AsyncRecord>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let scoped_by_origin = Arc::clone(&scoped_by_origin);
        let seed_by_index = Arc::clone(&seed_by_index);
        let expected_names_for_async = Arc::clone(&expected_names_for_async);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let worker_name = std::thread::current()
                .name()
                .map(str::to_owned)
                .expect("spawn_broadcast should run on a named custom-spawned thread");

            assert_eq!(
                expected_names_for_async.get(&index),
                Some(&worker_name)
            );

            let seed = seed_by_index[index];
            let scoped_value = scoped_by_origin
                .get(&index)
                .expect("async broadcast should consume prior scoped output")
                .value;

            let (left, right) =
                rayon_core::join(move || scoped_value + seed, move || index + num_threads * 100);

            async_tx
                .send(AsyncRecord {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    worker_name,
                    seed,
                    scoped_value,
                    async_value: left + right,
                    pending_status_available:
                        rayon_core::current_thread_has_pending_tasks().is_some(),
                })
                .expect("spawn_broadcast worker should report its derived output");
        }
    });

    let mut async_records = recv_exact(
        &async_rx,
        thread_count,
        "spawn_broadcast after custom spawn_handler",
    );
    async_records.sort_by_key(|record| record.index);

    assert_eq!(
        async_records
            .iter()
            .map(|record| record.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &async_records {
        assert_eq!(record.num_threads, thread_count);
        assert_eq!(record.current_index, Some(record.index));
        assert_eq!(record.worker_name, expected_names[&record.index]);
        assert_eq!(record.seed, seed_by_index[record.index]);
        assert_eq!(record.scoped_value, scoped_by_origin[&record.index].value);
        assert_eq!(
            record.async_value,
            record.scoped_value + record.seed + record.index + thread_count * 100
        );
        assert!(
            record.pending_status_available,
            "spawn_broadcast should observe pending-task status on custom-spawned workers"
        );
    }

    assert!(
        async_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_broadcast should report exactly once per worker"
    );

    let async_sum: usize = async_records.iter().map(|record| record.async_value).sum();
    let final_marker = expected_seed_sum + expected_scoped_sum + async_sum;
    lifecycle_marker.store(final_marker, Ordering::SeqCst);

    drop(pool);

    let mut exit_events = recv_exact(&exit_rx, thread_count, "exit_handler");
    exit_events.sort_by_key(|event| event.index);

    assert_eq!(
        exit_events
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &exit_events {
        assert_eq!(event.name.as_ref(), expected_names.get(&event.index));
        assert_eq!(
            event.observed_marker, final_marker,
            "exit_handler should observe data published after all custom-spawned work completed"
        );
    }

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit_handler should run exactly once per worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_spawn_handler_error_reports_metadata_and_later_custom_spawn_pool_still_works(
) {
    let stack_size = 1024 * 1024usize;
    let error_message = "intentional ThreadPoolBuilder::spawn_handler construction failure";

    let (attempt_tx, attempt_rx) = mpsc::channel::<SpawnHandlerEvent>();
    let (start_tx, start_rx) = mpsc::channel::<usize>();

    let failing_builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("failing-spawn-handler-worker-{index}"))
        .stack_size(stack_size)
        .start_handler(move |index| {
            start_tx
                .send(index)
                .expect("start_handler channel should be available if a worker starts");
        });

    let failing_builder =
        rayon_core::ThreadPoolBuilder::spawn_handler(failing_builder, move |thread| {
            attempt_tx
                .send(SpawnHandlerEvent {
                    index: rayon_core::ThreadBuilder::index(&thread),
                    name: rayon_core::ThreadBuilder::name(&thread).map(str::to_owned),
                    stack_size: rayon_core::ThreadBuilder::stack_size(&thread),
                    handler_thread_index: rayon_core::current_thread_index(),
                })
                .expect("failing spawn_handler should report ThreadBuilder metadata");

            Err(io::Error::new(io::ErrorKind::Other, error_message))
        });

    let build_error = match rayon_core::ThreadPoolBuilder::build(failing_builder) {
        Ok(pool) => {
            drop(pool);
            panic!("spawn_handler returning an I/O error should make build fail");
        }
        Err(error) => error,
    };

    let attempts = recv_exact(&attempt_rx, 1, "failing spawn_handler");
    assert!(
        attempt_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "single-thread failing pool should call spawn_handler exactly once"
    );

    let attempt = &attempts[0];
    assert_eq!(attempt.index, 0);
    assert_eq!(
        attempt.name.as_deref(),
        Some("failing-spawn-handler-worker-0")
    );
    assert_eq!(attempt.stack_size, Some(stack_size));
    assert_eq!(attempt.handler_thread_index, None);

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "ThreadBuilder::run was never called, so start_handler should not run"
    );

    let error_text = build_error.to_string();
    let mut saw_custom_error = error_text.contains(error_message);
    let mut source = build_error.source();

    while let Some(error) = source {
        let source_text = error.to_string();
        saw_custom_error |= source_text.contains(error_message);
        source = error.source();
    }

    assert!(
        saw_custom_error,
        "build error should preserve the spawn_handler I/O error; got {error_text:?}"
    );

    let recovery_thread_count = 2usize;
    let expected_indices = expected_worker_indices(recovery_thread_count);
    let (recovery_spawn_tx, recovery_spawn_rx) = mpsc::channel::<SpawnHandlerEvent>();

    let recovery_builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(recovery_thread_count)
        .thread_name(|index| format!("spawn-handler-recovery-worker-{index}"))
        .stack_size(
            attempt
                .stack_size
                .expect("failed attempt should still expose configured stack size"),
        );

    let recovery_builder =
        rayon_core::ThreadPoolBuilder::spawn_handler(recovery_builder, move |thread| {
            let index = rayon_core::ThreadBuilder::index(&thread);
            let name = rayon_core::ThreadBuilder::name(&thread).map(str::to_owned);
            let stack_size = rayon_core::ThreadBuilder::stack_size(&thread);

            recovery_spawn_tx
                .send(SpawnHandlerEvent {
                    index,
                    name: name.clone(),
                    stack_size,
                    handler_thread_index: rayon_core::current_thread_index(),
                })
                .expect("recovery spawn_handler should report metadata");

            let mut std_builder = std::thread::Builder::new();

            if let Some(name) = name {
                std_builder = std_builder.name(name);
            }

            if let Some(stack_size) = stack_size {
                std_builder = std_builder.stack_size(stack_size);
            }

            std_builder
                .spawn(move || rayon_core::ThreadBuilder::run(thread))
                .map(|_| ())
        });

    let recovery_pool = rayon_core::ThreadPoolBuilder::build(recovery_builder)
        .expect("a later pool using spawn_handler should build after the earlier error");

    let mut recovery_spawns = recv_exact(
        &recovery_spawn_rx,
        recovery_thread_count,
        "recovery spawn_handler",
    );
    recovery_spawns.sort_by_key(|event| event.index);

    assert_eq!(
        recovery_spawns
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &recovery_spawns {
        let expected_name = format!("spawn-handler-recovery-worker-{}", event.index);
        assert_eq!(event.name.as_deref(), Some(expected_name.as_str()));
        assert_eq!(event.stack_size, Some(stack_size));
        assert_eq!(event.handler_thread_index, None);
    }

    assert!(
        recovery_spawn_rx
            .recv_timeout(Duration::from_millis(100))
            .is_err(),
        "recovery spawn_handler should run exactly once per worker"
    );

    let mut observations = rayon_core::ThreadPool::broadcast(&recovery_pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);
        let name = std::thread::current()
            .name()
            .map(str::to_owned)
            .expect("recovery worker should keep its spawn_handler-provided name");

        assert_eq!(num_threads, recovery_thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (
            index,
            num_threads,
            rayon_core::current_thread_index(),
            name,
            rayon_core::current_thread_has_pending_tasks().is_some(),
        )
    });

    observations.sort_by_key(|record| record.0);

    assert_eq!(observations.len(), recovery_thread_count);
    assert_eq!(
        observations
            .iter()
            .map(|record| record.0)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for (index, num_threads, current_index, name, pending_status_available) in &observations {
        assert_eq!(*num_threads, recovery_thread_count);
        assert_eq!(*current_index, Some(*index));
        assert_eq!(
            name,
            &format!("spawn-handler-recovery-worker-{index}")
        );
        assert!(
            *pending_status_available,
            "recovery broadcast should run on real Rayon workers"
        );
    }

    let (name_len_sum, index_sum) = rayon_core::ThreadPool::join(
        &recovery_pool,
        || observations.iter().map(|record| record.3.len()).sum::<usize>(),
        || observations.iter().map(|record| record.0).sum::<usize>(),
    );

    assert_eq!(
        name_len_sum,
        observations
            .iter()
            .map(|record| format!("spawn-handler-recovery-worker-{}", record.0).len())
            .sum::<usize>()
    );
    assert_eq!(
        index_sum,
        recovery_thread_count * (recovery_thread_count - 1) / 2
    );
}