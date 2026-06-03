#![allow(deprecated)]

use std::collections::{BTreeMap, BTreeSet};
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkerNameObservation {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedNameResult {
    origin_index: usize,
    running_index: usize,
    running_name: String,
    combined: usize,
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

fn names_by_index(calls: &[NameCall]) -> BTreeMap<usize, BTreeSet<String>> {
    let mut map = BTreeMap::<usize, BTreeSet<String>>::new();

    for call in calls {
        map.entry(call.index).or_default().insert(call.name.clone());
    }

    map
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

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn deprecated_configuration_thread_name_drives_lifecycle_broadcast_and_scoped_work() {
    let thread_count = 4usize;
    let expected_indices = expected_worker_indices(thread_count);

    let name_calls = Arc::new(Mutex::new(Vec::<NameCall>::new()));
    let (start_tx, start_rx) = mpsc::channel::<StartNameEvent>();
    let (exit_tx, exit_rx) = mpsc::channel::<usize>();

    let config = rayon_core::Configuration::new().num_threads(thread_count);

    let config = rayon_core::Configuration::thread_name(config, {
        let name_calls = Arc::clone(&name_calls);
        let mut ordinal = 0usize;

        move |index| {
            let call_ordinal = ordinal;
            ordinal += 1;

            let name =
                format!("deprecated-config-thread-name-worker-{index}-call-{call_ordinal}");

            name_calls
                .lock()
                .expect("thread-name call log mutex should not be poisoned")
                .push(NameCall {
                    index,
                    ordinal: call_ordinal,
                    name: name.clone(),
                });

            name
        }
    });

    let config = rayon_core::Configuration::start_handler(config, move |index| {
        start_tx
            .send(StartNameEvent {
                index,
                name: std::thread::current().name().map(str::to_owned),
            })
            .expect("start handler should be able to report thread name");
    });

    let config = rayon_core::Configuration::exit_handler(config, move |index| {
        exit_tx
            .send(index)
            .expect("exit handler should be able to report worker shutdown");
    });

    let pool =
        rayon_core::Configuration::build(config).expect("Configuration should build a pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "the test thread should not be a worker in this custom pool"
    );

    let start_events = recv_exact(&start_rx, thread_count, "Configuration::start_handler");

    let calls = name_calls
        .lock()
        .expect("thread-name call log mutex should not be poisoned")
        .clone();

    assert!(
        calls.len() >= thread_count,
        "thread_name closure should be called at least once per worker"
    );

    let call_indices: BTreeSet<_> = calls.iter().map(|call| call.index).collect();
    assert_eq!(call_indices, expected_indices);

    let ordinals: BTreeSet<_> = calls.iter().map(|call| call.ordinal).collect();
    assert_eq!(
        ordinals,
        (0..calls.len()).collect::<BTreeSet<_>>(),
        "FnMut state in the deprecated thread_name closure should advance once per call"
    );

    let generated_names: BTreeSet<_> = calls.iter().map(|call| call.name.clone()).collect();
    assert_eq!(
        generated_names.len(),
        calls.len(),
        "each generated worker name should be unique"
    );

    let calls_by_index = names_by_index(&calls);

    let start_indices: BTreeSet<_> = start_events.iter().map(|event| event.index).collect();
    assert_eq!(start_indices, expected_indices);

    for event in &start_events {
        let start_name = event
            .name
            .as_ref()
            .expect("start handler should run on a named worker thread");

        assert!(
            calls_by_index
                .get(&event.index)
                .expect("worker index should have a generated name")
                .contains(start_name),
            "start handler observed name {start_name:?} for worker {}, but it was not produced by Configuration::thread_name",
            event.index
        );
    }

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "start handler should run exactly once per worker"
    );

    let mut broadcast_events = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);
        let current_index = rayon_core::current_thread_index();

        assert_eq!(num_threads, thread_count);
        assert_eq!(current_index, Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        WorkerNameObservation {
            index,
            num_threads,
            current_index,
            name: std::thread::current()
                .name()
                .map(str::to_owned)
                .expect("broadcast should run on a named Rayon worker"),
        }
    });

    broadcast_events.sort_by_key(|event| event.index);

    assert_eq!(broadcast_events.len(), thread_count);

    for event in &broadcast_events {
        assert_eq!(event.num_threads, thread_count);
        assert_eq!(event.current_index, Some(event.index));
        assert!(
            calls_by_index
                .get(&event.index)
                .expect("worker index should have generated names")
                .contains(&event.name),
            "broadcast observed name {:?} for worker {}, but it was not produced by Configuration::thread_name",
            event.name,
            event.index
        );
    }

    let observed_name_by_index = observed_name_map(&broadcast_events);
    assert_eq!(
        observed_name_by_index.keys().copied().collect::<BTreeSet<_>>(),
        expected_indices
    );

    for event in &start_events {
        let start_name = event
            .name
            .as_ref()
            .expect("start handler should observe a named thread");

        assert_eq!(
            observed_name_by_index.get(&event.index),
            Some(start_name),
            "start handler and broadcast should observe the same configured name for worker {}",
            event.index
        );
    }

    let expected_checksum: usize = broadcast_events
        .iter()
        .map(|event| event.name.len() + event.index + event.num_threads)
        .sum();

    let scoped_results = Mutex::new(Vec::<ScopedNameResult>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for event in broadcast_events.iter().cloned() {
            let scoped_results = &scoped_results;
            let observed_name_by_index = &observed_name_by_index;

            rayon_core::Scope::spawn(scope, move |_| {
                let running_index = rayon_core::current_thread_index()
                    .expect("scoped work should run on a Rayon worker");
                assert!(running_index < event.num_threads);

                let running_name = std::thread::current()
                    .name()
                    .map(str::to_owned)
                    .expect("scoped work should run on a named worker thread");

                assert_eq!(
                    observed_name_by_index.get(&running_index),
                    Some(&running_name),
                    "worker index should continue using its configured name"
                );

                let name_len = event.name.len();
                let index_plus_threads = event.index + event.num_threads;
                let (left, right) =
                    rayon_core::join(move || name_len, move || index_plus_threads);

                scoped_results
                    .lock()
                    .expect("scoped result mutex should not be poisoned")
                    .push(ScopedNameResult {
                        origin_index: event.index,
                        running_index,
                        running_name,
                        combined: left + right,
                    });
            });
        }

        broadcast_events.len()
    });

    assert_eq!(scope_return, thread_count);

    let mut scoped_results = scoped_results
        .into_inner()
        .expect("scoped result mutex should not be poisoned");
    scoped_results.sort_by_key(|result| result.origin_index);

    assert_eq!(scoped_results.len(), thread_count);
    assert_eq!(
        scoped_results
            .iter()
            .map(|result| result.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    let scoped_checksum: usize = scoped_results.iter().map(|result| result.combined).sum();
    assert_eq!(scoped_checksum, expected_checksum);

    for result in &scoped_results {
        assert!(result.running_index < thread_count);
        assert_eq!(
            observed_name_by_index.get(&result.running_index),
            Some(&result.running_name)
        );

        let origin = broadcast_events
            .iter()
            .find(|event| event.index == result.origin_index)
            .expect("scoped result should correspond to a broadcast origin");

        assert_eq!(
            result.combined,
            origin.name.len() + origin.index + origin.num_threads
        );
    }

    drop(pool);

    let exited: BTreeSet<_> = recv_exact(&exit_rx, thread_count, "Configuration::exit_handler")
        .into_iter()
        .collect();
    assert_eq!(exited, expected_indices);

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit handler should run exactly once per worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn deprecated_configuration_thread_name_fnmut_state_is_visible_to_pool_new_fifo_and_async_broadcast()
{
    let thread_count = 3usize;
    let expected_indices = expected_worker_indices(thread_count);
    let (name_tx, name_rx) = mpsc::channel::<NameCall>();

    let config = rayon_core::Configuration::new().num_threads(thread_count);

    let config = rayon_core::Configuration::thread_name(config, {
        let mut token = 1000usize;

        move |index| {
            let ordinal = token;
            token += index + 7;

            let name = format!("deprecated-config-fnmut-name-worker-{index}-token-{ordinal}");

            name_tx
                .send(NameCall {
                    index,
                    ordinal,
                    name: name.clone(),
                })
                .expect("thread_name closure should be able to report generated names");

            name
        }
    });

    let pool = rayon_core::ThreadPool::new(config)
        .expect("ThreadPool::new should accept Configuration::thread_name");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);

    let name_calls = recv_exact(&name_rx, thread_count, "Configuration::thread_name");
    assert!(
        name_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "default pool construction should request one name per worker"
    );

    let token_set: BTreeSet<_> = name_calls.iter().map(|call| call.ordinal).collect();
    assert_eq!(
        token_set.len(),
        thread_count,
        "mutable state in the FnMut thread_name closure should produce distinct tokens"
    );
    assert!(
        token_set.iter().all(|token| *token >= 1000),
        "generated tokens should come from the closure-owned state"
    );

    let mut expected_by_index = BTreeMap::<usize, String>::new();
    for call in name_calls {
        assert!(
            expected_by_index.insert(call.index, call.name).is_none(),
            "each worker index should receive one configured name"
        );
    }

    assert_eq!(
        expected_by_index.keys().copied().collect::<BTreeSet<_>>(),
        expected_indices
    );

    let mut observations = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);
        let current_index = rayon_core::current_thread_index();
        let name = std::thread::current()
            .name()
            .map(str::to_owned)
            .expect("broadcast should run on a named worker thread");

        assert_eq!(num_threads, thread_count);
        assert_eq!(current_index, Some(index));
        assert_eq!(
            expected_by_index.get(&index).map(String::as_str),
            Some(name.as_str())
        );

        WorkerNameObservation {
            index,
            num_threads,
            current_index,
            name,
        }
    });

    observations.sort_by_key(|observation| observation.index);
    assert_eq!(observations.len(), thread_count);

    let observed_by_index = observed_name_map(&observations);
    assert_eq!(observed_by_index, expected_by_index);

    let expected_fifo_checksum: usize = observations
        .iter()
        .map(|observation| observation.index * 100 + observation.name.len())
        .sum();

    let fifo_results = Mutex::new(Vec::<ScopedNameResult>::new());

    let returned_fifo_checksum = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        for observation in observations.iter().cloned() {
            let fifo_results = &fifo_results;
            let expected_by_index = &expected_by_index;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let running_index = rayon_core::current_thread_index()
                    .expect("FIFO scoped work should run on a Rayon worker");
                assert!(running_index < observation.num_threads);

                let running_name = std::thread::current()
                    .name()
                    .map(str::to_owned)
                    .expect("FIFO scoped work should run on a named worker thread");

                assert_eq!(
                    expected_by_index.get(&running_index),
                    Some(&running_name),
                    "FIFO work should observe the configured name for its worker index"
                );

                let left_input = observation.index * 100;
                let right_input = observation.name.len();
                let (left, right) =
                    rayon_core::join(move || left_input, move || right_input);

                fifo_results
                    .lock()
                    .expect("FIFO result mutex should not be poisoned")
                    .push(ScopedNameResult {
                        origin_index: observation.index,
                        running_index,
                        running_name,
                        combined: left + right,
                    });
            });
        }

        expected_fifo_checksum
    });

    assert_eq!(returned_fifo_checksum, expected_fifo_checksum);

    let mut fifo_results = fifo_results
        .into_inner()
        .expect("FIFO result mutex should not be poisoned");
    fifo_results.sort_by_key(|result| result.origin_index);

    assert_eq!(fifo_results.len(), thread_count);
    assert_eq!(
        fifo_results
            .iter()
            .map(|result| result.origin_index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );
    assert_eq!(
        fifo_results
            .iter()
            .map(|result| result.combined)
            .sum::<usize>(),
        expected_fifo_checksum
    );

    for result in &fifo_results {
        assert!(result.running_index < thread_count);
        assert_eq!(
            expected_by_index.get(&result.running_index),
            Some(&result.running_name)
        );

        let origin = observations
            .iter()
            .find(|observation| observation.index == result.origin_index)
            .expect("FIFO result should correspond to a broadcast observation");

        assert_eq!(
            result.combined,
            origin.index * 100 + origin.name.len(),
            "FIFO work should use data produced by the earlier broadcast step"
        );
    }

    let (async_tx, async_rx) = mpsc::channel::<WorkerNameObservation>();
    let expected_for_async = expected_by_index.clone();

    rayon_core::ThreadPool::spawn_broadcast(&pool, move |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);
        let current_index = rayon_core::current_thread_index();
        let name = std::thread::current()
            .name()
            .map(str::to_owned)
            .expect("spawn_broadcast should run on a named worker thread");

        assert_eq!(
            expected_for_async.get(&index).map(String::as_str),
            Some(name.as_str())
        );

        async_tx
            .send(WorkerNameObservation {
                index,
                num_threads,
                current_index,
                name,
            })
            .expect("spawn_broadcast worker should be able to report its configured name");
    });

    let mut async_observations =
        recv_exact(&async_rx, thread_count, "ThreadPool::spawn_broadcast");
    async_observations.sort_by_key(|observation| observation.index);

    assert!(
        async_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "spawn_broadcast should report exactly once per worker"
    );

    assert_eq!(
        async_observations
            .iter()
            .map(|observation| observation.index)
            .collect::<BTreeSet<_>>(),
        expected_indices
    );

    for observation in &async_observations {
        assert_eq!(observation.num_threads, thread_count);
        assert_eq!(observation.current_index, Some(observation.index));
        assert_eq!(
            expected_by_index.get(&observation.index),
            Some(&observation.name),
            "asynchronous broadcast should observe the same name configured by Configuration::thread_name"
        );
    }
}