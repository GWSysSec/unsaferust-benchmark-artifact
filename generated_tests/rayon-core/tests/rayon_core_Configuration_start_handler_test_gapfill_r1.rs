#![allow(deprecated)]

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartEvent {
    index: usize,
    ordinal: usize,
    name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BroadcastEvent {
    index: usize,
    num_threads: usize,
    current_index: Option<usize>,
    value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedEvent {
    origin_index: usize,
    worker_index: usize,
    total: usize,
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
fn deprecated_configuration_start_handler_reports_named_workers_and_drives_followup_scoped_work() {
    let thread_count = 4usize;
    let start_count = Arc::new(AtomicUsize::new(0));

    let (start_tx, start_rx) = mpsc::channel::<StartEvent>();
    let (exit_tx, exit_rx) = mpsc::channel::<usize>();

    let config = rayon_core::Configuration::new().num_threads(thread_count);

    let config = rayon_core::Configuration::thread_name(config, |index| {
        format!("deprecated-start-handler-worker-{index}")
    });

    let config = rayon_core::Configuration::start_handler(config, {
        let start_count = Arc::clone(&start_count);

        move |index| {
            let ordinal = start_count.fetch_add(1, Ordering::SeqCst);
            let name = std::thread::current().name().map(str::to_owned);

            start_tx
                .send(StartEvent {
                    index,
                    ordinal,
                    name,
                })
                .expect("start handler should be able to report worker startup");
        }
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
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None
    );

    let start_events = recv_exact(&start_rx, thread_count, "Configuration::start_handler");
    assert_eq!(start_count.load(Ordering::SeqCst), thread_count);
    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "start_handler should run exactly once for each worker"
    );

    let expected_indices = expected_worker_indices(thread_count);

    let observed_start_indices: BTreeSet<_> =
        start_events.iter().map(|event| event.index).collect();
    assert_eq!(observed_start_indices, expected_indices);

    let observed_ordinals: BTreeSet<_> =
        start_events.iter().map(|event| event.ordinal).collect();
    assert_eq!(observed_ordinals, expected_worker_indices(thread_count));

    for event in &start_events {
        let expected_name = format!("deprecated-start-handler-worker-{}", event.index);
        assert_eq!(
            event.name.as_deref(),
            Some(expected_name.as_str()),
            "worker {} should already have its configured name when start_handler runs",
            event.index
        );
    }

    let start_checksum: usize = start_events
        .iter()
        .map(|event| (event.index + 1) * (event.ordinal + 3))
        .sum();

    let expected_for_broadcast = expected_indices.clone();
    let mut broadcast_events = rayon_core::ThreadPool::broadcast(&pool, move |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);

        assert_eq!(num_threads, thread_count);
        assert!(expected_for_broadcast.contains(&index));
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_num_threads(), thread_count);

        BroadcastEvent {
            index,
            num_threads,
            current_index: rayon_core::current_thread_index(),
            value: start_checksum + (index + 1) * num_threads,
        }
    });

    broadcast_events.sort_by_key(|event| event.index);

    assert_eq!(broadcast_events.len(), thread_count);
    let observed_broadcast_indices: BTreeSet<_> =
        broadcast_events.iter().map(|event| event.index).collect();
    assert_eq!(observed_broadcast_indices, expected_indices);

    for event in &broadcast_events {
        assert_eq!(event.num_threads, thread_count);
        assert_eq!(event.current_index, Some(event.index));
        assert_eq!(
            event.value,
            start_checksum + (event.index + 1) * thread_count
        );
    }

    let scoped_events = Mutex::new(Vec::<ScopedEvent>::new());

    let scope_return = rayon_core::ThreadPool::scope(&pool, |scope| {
        for event in broadcast_events.iter().cloned() {
            let scoped_events = &scoped_events;

            rayon_core::Scope::spawn(scope, move |_| {
                let origin_index = event.index;
                let value = event.value;
                let right_input = event.index
                    + event.num_threads
                    + event
                        .current_index
                        .expect("broadcast event should include a worker index");

                let (left, right) =
                    rayon_core::join(move || value, move || right_input);

                let worker_index = rayon_core::current_thread_index()
                    .expect("scoped follow-up work should run on a Rayon worker");

                scoped_events
                    .lock()
                    .expect("scoped event mutex should not be poisoned")
                    .push(ScopedEvent {
                        origin_index,
                        worker_index,
                        total: left + right,
                    });
            });
        }

        broadcast_events.len()
    });

    assert_eq!(scope_return, thread_count);

    let mut scoped_events = scoped_events
        .into_inner()
        .expect("scoped event mutex should not be poisoned");
    scoped_events.sort_by_key(|event| event.origin_index);

    assert_eq!(scoped_events.len(), thread_count);

    for (scoped, broadcast) in scoped_events.iter().zip(broadcast_events.iter()) {
        assert_eq!(scoped.origin_index, broadcast.index);
        assert!(scoped.worker_index < thread_count);
        assert_eq!(
            scoped.total,
            broadcast.value
                + broadcast.index
                + broadcast.num_threads
                + broadcast.current_index.expect("broadcast current index")
        );
    }

    drop(pool);

    let exited: BTreeSet<_> = recv_exact(&exit_rx, thread_count, "exit handler")
        .into_iter()
        .collect();
    assert_eq!(exited, expected_indices);
    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "exit handler should also run exactly once per worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn deprecated_configuration_start_handler_seeded_state_is_used_by_async_and_fifo_work() {
    let thread_count = 3usize;
    let seeds = Arc::new(Mutex::new(vec![0usize; thread_count]));

    let (start_tx, start_rx) = mpsc::channel::<(usize, usize)>();
    let (exit_tx, exit_rx) = mpsc::channel::<usize>();

    let config = rayon_core::Configuration::new().num_threads(thread_count);

    let config = rayon_core::Configuration::start_handler(config, {
        let seeds = Arc::clone(&seeds);

        move |index| {
            let seed = (index + 1) * 11;

            {
                let mut seeds = seeds
                    .lock()
                    .expect("seed mutex should not be poisoned in start handler");
                seeds[index] = seed;
            }

            start_tx
                .send((index, seed))
                .expect("start handler should report seeded worker state");
        }
    });

    let config = rayon_core::Configuration::exit_handler(config, move |index| {
        exit_tx
            .send(index)
            .expect("exit handler should report worker shutdown");
    });

    let pool = rayon_core::ThreadPool::new(config)
        .expect("ThreadPool::new should accept Configuration with start_handler");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(rayon_core::ThreadPool::current_thread_index(&pool), None);

    let mut starts = recv_exact(&start_rx, thread_count, "seed start handler");
    starts.sort_by_key(|entry| entry.0);

    let expected_indices = expected_worker_indices(thread_count);
    let observed_start_indices: BTreeSet<_> = starts.iter().map(|(index, _)| *index).collect();
    assert_eq!(observed_start_indices, expected_indices);

    for (index, seed) in &starts {
        assert_eq!(*seed, (*index + 1) * 11);
    }

    let expected_seed_vector: Vec<_> = (0..thread_count).map(|index| (index + 1) * 11).collect();
    assert_eq!(
        seeds
            .lock()
            .expect("seed mutex should not be poisoned")
            .clone(),
        expected_seed_vector
    );

    assert!(
        start_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "seed start_handler should run exactly once per worker"
    );

    let mut broadcast_values = rayon_core::ThreadPool::broadcast(&pool, {
        let seeds = Arc::clone(&seeds);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let total = rayon_core::BroadcastContext::num_threads(&context);

            assert_eq!(total, thread_count);
            assert_eq!(rayon_core::current_thread_index(), Some(index));

            let seed = {
                let seeds = seeds
                    .lock()
                    .expect("seed mutex should not be poisoned during broadcast");
                seeds[index]
            };

            assert_eq!(seed, (index + 1) * 11);

            (index, total, seed, seed + total + index)
        }
    });

    broadcast_values.sort_by_key(|entry| entry.0);
    assert_eq!(broadcast_values.len(), thread_count);

    for (index, total, seed, derived) in &broadcast_values {
        assert_eq!(*index + 1, *seed / 11);
        assert_eq!(*total, thread_count);
        assert_eq!(*derived, *seed + *total + *index);
    }

    let fifo_results = Mutex::new(Vec::<(usize, usize, Option<usize>)>::new());
    let broadcast_sum: usize = broadcast_values.iter().map(|entry| entry.3).sum();

    let in_place_return = rayon_core::ThreadPool::in_place_scope_fifo(&pool, |scope| {
        for &(index, total, seed, derived) in &broadcast_values {
            let fifo_results = &fifo_results;

            rayon_core::ScopeFifo::spawn_fifo(scope, move |_| {
                let (left, right) =
                    rayon_core::join(move || derived, move || seed + total + index);

                fifo_results
                    .lock()
                    .expect("fifo result mutex should not be poisoned")
                    .push((index, left + right, rayon_core::current_thread_index()));
            });
        }

        broadcast_sum
    });

    assert_eq!(in_place_return, broadcast_sum);

    let mut fifo_results = fifo_results
        .into_inner()
        .expect("fifo result mutex should not be poisoned");
    fifo_results.sort_by_key(|entry| entry.0);

    assert_eq!(fifo_results.len(), thread_count);

    for ((index, total, seed, derived), (result_index, combined, worker_index)) in
        broadcast_values.iter().zip(fifo_results.iter())
    {
        assert_eq!(result_index, index);
        assert_eq!(*combined, *derived + *seed + *total + *index);

        if let Some(worker_index) = *worker_index {
            assert!(worker_index < thread_count);
        }
    }

    let (async_tx, async_rx) = mpsc::channel::<(usize, usize, usize, Option<usize>)>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let seeds = Arc::clone(&seeds);

        move |context| {
            let index = rayon_core::BroadcastContext::index(&context);
            let total = rayon_core::BroadcastContext::num_threads(&context);
            let seed = {
                let seeds = seeds
                    .lock()
                    .expect("seed mutex should not be poisoned in spawn_broadcast");
                seeds[index]
            };

            async_tx
                .send((index, total, seed, rayon_core::current_thread_index()))
                .expect("spawn_broadcast worker should report seeded state");
        }
    });

    let async_records = recv_exact(
        &async_rx,
        thread_count,
        "spawn_broadcast using start_handler state",
    );

    let mut async_indices = BTreeSet::new();
    for (index, total, seed, current_index) in async_records {
        assert_eq!(total, thread_count);
        assert_eq!(seed, (index + 1) * 11);
        assert_eq!(current_index, Some(index));
        assert!(
            async_indices.insert(index),
            "worker {index} should run spawn_broadcast exactly once"
        );
    }

    assert_eq!(async_indices, expected_indices);

    drop(pool);

    let exited: BTreeSet<_> = recv_exact(&exit_rx, thread_count, "seed pool exit handler")
        .into_iter()
        .collect();
    assert_eq!(exited, expected_indices);
}