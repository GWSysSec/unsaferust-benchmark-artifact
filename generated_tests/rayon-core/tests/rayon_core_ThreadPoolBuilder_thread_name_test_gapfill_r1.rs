use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NameCall {
    index: usize,
    ordinal: usize,
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartNameEvent {
    index: usize,
    name: Option<String>,
    current_index: Option<usize>,
    current_threads: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkerNameObservation {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    name: String,
    derived: usize,
    pending_status_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedNameCheck {
    origin_index: usize,
    running_index: usize,
    running_name: String,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AsyncNameCheck {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    name: String,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExitNameEvent {
    index: usize,
    name: Option<String>,
    marker: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpawnHandlerEvent {
    index: usize,
    name: Option<String>,
    stack_size: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SeedNameRecord {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    name: String,
    seed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NamedPanicRecord {
    origin_index: usize,
    seed: usize,
    executing_index: usize,
    worker_name: String,
    checksum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HandledNamedPanic {
    Record(NamedPanicRecord),
    Message(String),
    Unexpected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveryNameRecord {
    origin_index: usize,
    executing_index: usize,
    running_name: String,
    value: usize,
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

fn drain_until_quiet<T>(receiver: &mpsc::Receiver<T>, quiet: Duration) -> Vec<T> {
    let mut drained = Vec::new();

    while let Ok(item) = receiver.recv_timeout(quiet) {
        drained.push(item);
    }

    drained
}

fn expected_worker_indices(thread_count: usize) -> BTreeSet<usize> {
    (0..thread_count).collect()
}

fn names_by_index(calls: &[NameCall]) -> BTreeMap<usize, BTreeSet<String>> {
    let mut names = BTreeMap::<usize, BTreeSet<String>>::new();

    for call in calls {
        names.entry(call.index).or_default().insert(call.name.clone());
    }

    names
}

fn observed_name_map(observations: &[WorkerNameObservation]) -> BTreeMap<usize, String> {
    let mut map = BTreeMap::new();

    for observation in observations {
        assert!(
            map.insert(observation.index, observation.name.clone())
                .is_none(),
            "worker {} should be observed exactly once",
            observation.index
        );
    }

    map
}

fn classify_named_payload(payload: &(dyn Any + Send)) -> HandledNamedPanic {
    if let Some(record) = payload.downcast_ref::<NamedPanicRecord>() {
        HandledNamedPanic::Record(record.clone())
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        HandledNamedPanic::Message((*message).to_owned())
    } else if let Some(message) = payload.downcast_ref::<String>() {
        HandledNamedPanic::Message(message.clone())
    } else {
        HandledNamedPanic::Unexpected
    }
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_thread_name_fnmut_names_are_stable_across_lifecycle_and_nested_work() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let lifecycle_marker = Arc::new(AtomicUsize::new(0));

    let (name_tx, name_rx) = mpsc::channel::<NameCall>();
    let (start_tx, start_rx) = mpsc::channel::<StartNameEvent>();
    let (exit_tx, exit_rx) = mpsc::channel::<ExitNameEvent>();

    let builder = rayon_core::ThreadPoolBuilder::new().num_threads(thread_count);

    let builder = rayon_core::ThreadPoolBuilder::thread_name(builder, {
        let mut ordinal = 0usize;

        move |index| {
            let call_ordinal = ordinal;
            ordinal += 1;

            let name =
                format!("builder-thread-name-fnmut-worker-{index}-call-{call_ordinal}");

            name_tx
                .send(NameCall {
                    index,
                    ordinal: call_ordinal,
                    name: name.clone(),
                })
                .expect("thread_name closure should report every generated name");

            name
        }
    });

    let builder = rayon_core::ThreadPoolBuilder::start_handler(builder, move |index| {
        start_tx
            .send(StartNameEvent {
                index,
                name: std::thread::current().name().map(str::to_owned),
                current_index: rayon_core::current_thread_index(),
                current_threads: rayon_core::current_num_threads(),
            })
            .expect("start handler should report the configured worker name");
    });

    let builder = rayon_core::ThreadPoolBuilder::exit_handler(builder, {
        let lifecycle_marker = Arc::clone(&lifecycle_marker);

        move |index| {
            exit_tx
                .send(ExitNameEvent {
                    index,
                    name: std::thread::current().name().map(str::to_owned),
                    marker: lifecycle_marker.load(Ordering::SeqCst),
                })
                .expect("exit handler should report the configured worker name");
        }
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("ThreadPoolBuilder::thread_name should build a named custom pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "the integration-test thread should not be a worker in the custom pool"
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut name_calls = recv_exact(
        &name_rx,
        thread_count,
        "ThreadPoolBuilder::thread_name closure",
    );
    name_calls.extend(drain_until_quiet(
        &name_rx,
        Duration::from_millis(100),
    ));
    name_calls.sort_by_key(|call| call.ordinal);

    assert!(
        name_calls.len() >= thread_count,
        "thread_name should be called at least once for every worker"
    );
    assert_eq!(
        name_calls
            .iter()
            .map(|call| call.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    assert_eq!(
        name_calls
            .iter()
            .map(|call| call.ordinal)
            .collect::<BTreeSet<_>>(),
        (0..name_calls.len()).collect::<BTreeSet<_>>(),
        "mutable state in the FnMut thread_name closure should advance once per call"
    );

    assert_eq!(
        name_calls
            .iter()
            .map(|call| call.name.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        name_calls.len(),
        "each generated worker name should be unique"
    );

    let names_by_worker = names_by_index(&name_calls);

    let mut start_events = recv_exact(
        &start_rx,
        thread_count,
        "ThreadPoolBuilder::start_handler after thread_name",
    );
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

        let name = event
            .name
            .as_ref()
            .expect("start_handler should run on a named worker thread");

        assert!(
            names_by_worker
                .get(&event.index)
                .expect("worker index should have generated names")
                .contains(name),
            "start_handler observed name {name:?} for worker {}, but it was not generated by ThreadPoolBuilder::thread_name",
            event.index
        );
    }

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "start_handler should run exactly once per worker"
    );

    let mut observations = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);
        let current_index = rayon_core::current_thread_index();

        assert_eq!(num_threads, thread_count);
        assert_eq!(current_index, Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let name = std::thread::current()
            .name()
            .map(str::to_owned)
            .expect("broadcast should run on a named Rayon worker");

        assert!(
            names_by_worker
                .get(&index)
                .expect("worker index should have generated names")
                .contains(&name),
            "broadcast observed name {name:?} for worker {index}, but it was not generated by ThreadPoolBuilder::thread_name"
        );

        WorkerNameObservation {
            index,
            num_threads,
            current_index,
            derived: name.len() + index + num_threads * 10,
            name,
            pending_status_available: rayon_core::current_thread_has_pending_tasks().is_some(),
        }
    });

    observations.sort_by_key(|observation| observation.index);

    assert_eq!(observations.len(), thread_count);
    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for observation in &observations {
        assert_eq!(observation.num_threads, thread_count);
        assert_eq!(observation.current_index, Some(observation.index));
        assert_eq!(
            observation.derived,
            observation.name.len() + observation.index + thread_count * 10
        );
        assert!(
            observation.pending_status_available,
            "broadcast worker should be able to query worker-local pending-task status"
        );
    }

    let observed_name_by_index = observed_name_map(&observations);

    for event in &start_events {
        assert_eq!(
            event.name.as_ref(),
            observed_name_by_index.get(&event.index),
            "start_handler and broadcast should observe the same configured name for worker {}",
            event.index
        );
    }

    let expected_observation_sum: usize =
        observations.iter().map(|record| record.derived).sum();

    let scoped_records = Mutex::new(Vec::<ScopedNameCheck>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for observation in observations.iter().cloned() {
            let scoped_records = &scoped_records;
            let observed_name_by_index = &observed_name_by_index;

            rayon_core::Scope::spawn(scope, move |_| {
                let running_index = rayon_core::current_thread_index()
                    .expect("scoped work should run on a Rayon worker");

                assert!(running_index < observation.num_threads);
                assert_eq!(rayon_core::current_num_threads(), observation.num_threads);

                let running_name = std::thread::current()
                    .name()
                    .map(str::to_owned)
                    .expect("scoped work should run on a named worker thread");

                assert_eq!(
                    observed_name_by_index.get(&running_index),
                    Some(&running_name),
                    "worker index should retain its configured thread_name during scoped work"
                );

                let name_len = running_name.len();
                let (left, right) =
                    rayon_core::join(move || observation.derived, move || name_len + running_index);

                scoped_records
                    .lock()
                    .expect("scoped record mutex should not be poisoned")
                    .push(ScopedNameCheck {
                        origin_index: observation.index,
                        running_index,
                        running_name,
                        value: left + right,
                    });
            });
        }

        expected_observation_sum
    });

    assert_eq!(scope_return, expected_observation_sum);

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
        assert!(record.running_index < thread_count);
        assert_eq!(
            observed_name_by_index.get(&record.running_index),
            Some(&record.running_name)
        );

        let origin = observations
            .iter()
            .find(|observation| observation.index == record.origin_index)
            .expect("scoped record should correspond to a broadcast observation");

        assert_eq!(
            record.value,
            origin.derived + record.running_name.len() + record.running_index
        );
    }

    let expected_for_async = Arc::new(observed_name_by_index.clone());
    let (async_tx, async_rx) = mpsc::channel::<AsyncNameCheck>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let expected_for_async = Arc::clone(&expected_for_async);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let num_threads = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(num_threads, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let name = std::thread::current()
                .name()
                .map(str::to_owned)
                .expect("spawn_broadcast should run on a named worker thread");

            assert_eq!(
                expected_for_async.get(&index),
                Some(&name),
                "spawn_broadcast should observe the same configured name for its worker index"
            );

            let name_len = name.len();
            let (left, right) =
                rayon_core::join(move || name_len + index, move || num_threads * 100);

            async_tx
                .send(AsyncNameCheck {
                    index,
                    num_threads,
                    current_index: rayon_core::current_thread_index(),
                    name,
                    value: left + right,
                })
                .expect("spawn_broadcast worker should report its name check");
        }
    });

    let mut async_records = recv_exact(
        &async_rx,
        thread_count,
        "ThreadPool::spawn_broadcast after thread_name",
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
        assert_eq!(
            observed_name_by_index.get(&record.index),
            Some(&record.name)
        );
        assert_eq!(
            record.value,
            record.name.len() + record.index + thread_count * 100
        );
    }

    assert!(
        async_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_broadcast should report exactly once per worker"
    );

    let final_marker = expected_observation_sum
        + scoped_records.iter().map(|record| record.value).sum::<usize>()
        + async_records.iter().map(|record| record.value).sum::<usize>();

    lifecycle_marker.store(final_marker, Ordering::SeqCst);

    drop(pool);

    let mut exit_events = recv_exact(
        &exit_rx,
        thread_count,
        "ThreadPoolBuilder::exit_handler after named work",
    );
    exit_events.sort_by_key(|event| event.index);

    assert_eq!(
        exit_events
            .iter()
            .map(|event| event.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &exit_events {
        assert_eq!(
            event.name.as_ref(),
            observed_name_by_index.get(&event.index),
            "exit_handler should run on the same named worker thread"
        );
        assert_eq!(
            event.marker, final_marker,
            "exit_handler should observe the marker published after named worker work completed"
        );
    }

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit_handler should run exactly once per worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn thread_pool_builder_thread_name_is_visible_to_custom_spawn_handler_and_recovered_after_panics()
{
    let thread_count = 3usize;
    let stack_size = 2 * 1024 * 1024usize;
    let expected_indices = expected_worker_indices(thread_count);

    let (spawn_tx, spawn_rx) = mpsc::channel::<SpawnHandlerEvent>();
    let (start_tx, start_rx) = mpsc::channel::<StartNameEvent>();
    let (panic_tx, panic_rx) = mpsc::channel::<HandledNamedPanic>();

    let builder = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .stack_size(stack_size);

    let builder = rayon_core::ThreadPoolBuilder::thread_name(builder, |index| {
        format!("custom-spawn-thread-name-worker-{index}")
    });

    let builder = rayon_core::ThreadPoolBuilder::start_handler(builder, move |index| {
        start_tx
            .send(StartNameEvent {
                index,
                name: std::thread::current().name().map(str::to_owned),
                current_index: rayon_core::current_thread_index(),
                current_threads: rayon_core::current_num_threads(),
            })
            .expect("start handler should report custom-spawn worker name");
    });

    let builder = rayon_core::ThreadPoolBuilder::panic_handler(builder, move |payload| {
        panic_tx
            .send(classify_named_payload(&*payload))
            .expect("panic handler should report named panic payload");
    });

    let builder = builder.spawn_handler(move |thread| {
        let index = rayon_core::ThreadBuilder::index(&thread);
        let name = rayon_core::ThreadBuilder::name(&thread).map(str::to_owned);
        let observed_stack_size = rayon_core::ThreadBuilder::stack_size(&thread);

        spawn_tx
            .send(SpawnHandlerEvent {
                index,
                name: name.clone(),
                stack_size: observed_stack_size,
            })
            .expect("spawn_handler should report ThreadBuilder metadata");

        let mut builder = std::thread::Builder::new();

        if let Some(name) = &name {
            builder = builder.name(name.clone());
        }

        if let Some(stack_size) = observed_stack_size {
            builder = builder.stack_size(stack_size);
        }

        builder
            .spawn(move || rayon_core::ThreadBuilder::run(thread))
            .map(|_| ())
    });

    let pool = rayon_core::ThreadPoolBuilder::build(builder)
        .expect("custom spawn_handler should build a pool with ThreadPoolBuilder::thread_name");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let mut spawn_events = recv_exact(
        &spawn_rx,
        thread_count,
        "custom spawn_handler receiving ThreadBuilder names",
    );
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
        let expected_name = format!("custom-spawn-thread-name-worker-{}", event.index);

        assert_eq!(event.name.as_deref(), Some(expected_name.as_str()));
        assert_eq!(event.stack_size, Some(stack_size));

        assert!(
            expected_names
                .insert(event.index, expected_name)
                .is_none(),
            "each worker index should be recorded once by spawn_handler"
        );
    }

    assert!(
        spawn_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_handler should receive exactly one ThreadBuilder per worker"
    );

    let mut start_events = recv_exact(
        &start_rx,
        thread_count,
        "start_handler on custom-spawn named threads",
    );
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
        "start_handler should run exactly once per custom-spawn worker"
    );

    let mut seeds = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        let name = std::thread::current()
            .name()
            .map(str::to_owned)
            .expect("broadcast should run on a named custom-spawn worker");

        assert_eq!(expected_names.get(&index), Some(&name));

        SeedNameRecord {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            name,
            seed: (index + 1) * (num_threads + 211),
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
        assert_eq!(expected_names.get(&record.index), Some(&record.name));
        assert_eq!(record.seed, (record.index + 1) * (thread_count + 211));
    }

    let seed_by_origin: BTreeMap<usize, usize> =
        seeds.iter().map(|record| (record.index, record.seed)).collect();

    let expected_names_for_panics = Arc::new(expected_names.clone());

    for seed_record in seeds.iter().cloned() {
        let expected_names_for_panics = Arc::clone(&expected_names_for_panics);

        rayon_core::ThreadPool::spawn(&pool, move || {
            let executing_index = rayon_core::current_thread_index()
                .expect("detached named panic should run on a Rayon worker");

            assert!(executing_index < thread_count);
            assert_eq!(rayon_core::current_num_threads(), thread_count);
            assert!(
                rayon_core::current_thread_has_pending_tasks().is_some(),
                "detached worker should be able to query pending-task status"
            );

            let worker_name = std::thread::current()
                .name()
                .map(str::to_owned)
                .expect("detached worker should keep its configured name");

            assert_eq!(
                expected_names_for_panics.get(&executing_index),
                Some(&worker_name)
            );

            let origin_index = seed_record.index;
            let seed = seed_record.seed;
            let (left, right) = rayon_core::join(
                move || seed + origin_index,
                move || thread_count * 100 + executing_index,
            );

            std::panic::panic_any(NamedPanicRecord {
                origin_index,
                seed,
                executing_index,
                worker_name,
                checksum: left + right,
            });
        });
    }

    let panic_events = recv_exact(
        &panic_rx,
        thread_count,
        "panic_handler for named detached work",
    );

    let mut panic_records = Vec::<NamedPanicRecord>::new();

    for event in panic_events {
        match event {
            HandledNamedPanic::Record(record) => panic_records.push(record),
            unexpected => panic!("unexpected named panic handler event: {unexpected:?}"),
        }
    }

    panic_records.sort_by_key(|record| record.origin_index);

    assert_eq!(
        panic_records
            .iter()
            .map(|record| record.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for record in &panic_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(
            seed_by_origin.get(&record.origin_index),
            Some(&record.seed)
        );
        assert_eq!(
            expected_names.get(&record.executing_index),
            Some(&record.worker_name)
        );
        assert_eq!(
            record.checksum,
            record.seed + record.origin_index + thread_count * 100 + record.executing_index
        );
    }

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "panic_handler should receive exactly the scheduled named panic payloads"
    );

    let panic_checksum_sum: usize = panic_records.iter().map(|record| record.checksum).sum();
    let recovery_records = Mutex::new(Vec::<RecoveryNameRecord>::new());

    let scope_return = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for record in panic_records.iter().cloned() {
            let recovery_records = &recovery_records;
            let expected_names = &expected_names;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let executing_index = rayon_core::current_thread_index()
                    .expect("recovery FIFO work should run on a Rayon worker");

                assert!(executing_index < thread_count);
                assert_eq!(rayon_core::current_num_threads(), thread_count);

                let running_name = std::thread::current()
                    .name()
                    .map(str::to_owned)
                    .expect("recovery worker should keep its configured name");

                assert_eq!(expected_names.get(&executing_index), Some(&running_name));

                let origin_index = record.origin_index;
                let seed = record.seed;
                let checksum = record.checksum;
                let (left, right) = rayon_core::join(
                    move || checksum + seed,
                    move || origin_index + executing_index,
                );

                recovery_records
                    .lock()
                    .expect("recovery record mutex should not be poisoned")
                    .push(RecoveryNameRecord {
                        origin_index,
                        executing_index,
                        running_name,
                        value: left + right,
                    });
            });
        }

        panic_checksum_sum
    });

    assert_eq!(scope_return, panic_checksum_sum);

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

    let panic_by_origin: BTreeMap<usize, NamedPanicRecord> = panic_records
        .iter()
        .cloned()
        .map(|record| (record.origin_index, record))
        .collect();

    for record in &recovery_records {
        assert!(record.origin_index < thread_count);
        assert!(record.executing_index < thread_count);
        assert_eq!(
            expected_names.get(&record.executing_index),
            Some(&record.running_name)
        );

        let panic_record = panic_by_origin
            .get(&record.origin_index)
            .expect("recovery record should correspond to a handled panic record");

        assert_eq!(
            record.value,
            panic_record.checksum
                + panic_record.seed
                + record.origin_index
                + record.executing_index
        );
    }

    let (observed_recovery_sum, recomputed_recovery_sum) =
        rayon_core::ThreadPool::join(
            &pool,
            || recovery_records.iter().map(|record| record.value).sum::<usize>(),
            || {
                recovery_records
                    .iter()
                    .map(|record| {
                        let panic_record = panic_by_origin
                            .get(&record.origin_index)
                            .expect("panic record should exist during recomputation");

                        panic_record.checksum
                            + panic_record.seed
                            + record.origin_index
                            + record.executing_index
                    })
                    .sum::<usize>()
            },
        );

    assert_eq!(observed_recovery_sum, recomputed_recovery_sum);

    assert!(
        panic_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "successful recovery work should not invoke the panic handler"
    );
}