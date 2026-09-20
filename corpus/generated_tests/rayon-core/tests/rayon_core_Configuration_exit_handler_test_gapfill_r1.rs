#![allow(deprecated)]

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

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
fn deprecated_configuration_exit_handler_reports_all_workers_after_scoped_work() {
    let thread_count = 3usize;

    let (start_tx, start_rx) = mpsc::channel::<usize>();
    let (exit_tx, exit_rx) = mpsc::channel::<usize>();
    let exit_calls = Arc::new(AtomicUsize::new(0));

    let config = rayon_core::Configuration::new().num_threads(thread_count);

    let config = rayon_core::Configuration::start_handler(config, move |index| {
        start_tx
            .send(index)
            .expect("start handler should be able to report worker index");
    });

    let config = rayon_core::Configuration::exit_handler(config, {
        let exit_calls = Arc::clone(&exit_calls);

        move |index| {
            exit_calls.fetch_add(1, Ordering::SeqCst);
            exit_tx
                .send(index)
                .expect("exit handler should be able to report worker index");
        }
    });

    let pool =
        rayon_core::Configuration::build(config).expect("configuration should build a thread pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert_eq!(
        rayon_core::ThreadPool::current_thread_index(&pool),
        None,
        "the test thread should not be a worker in the custom pool"
    );

    let started: BTreeSet<_> = recv_exact(&start_rx, thread_count, "start handler")
        .into_iter()
        .collect();
    assert_eq!(started, expected_worker_indices(thread_count));

    let task_count = rayon_core::ThreadPool::current_num_threads(&pool) * 2;
    let (work_tx, work_rx) = mpsc::channel::<(usize, usize, Option<usize>)>();

    let scope_result = rayon_core::ThreadPool::scope(&pool, {
        let work_tx = work_tx.clone();

        move |scope| {
            for input in 0..task_count {
                let work_tx = work_tx.clone();

                rayon_core::Scope::spawn(scope, move |_| {
                    let (square, cube) =
                        rayon_core::join(|| input * input, || input * input * input);

                    work_tx
                        .send((input, square + cube, rayon_core::current_thread_index()))
                        .expect("scoped worker should be able to report its result");
                });
            }

            task_count
        }
    });

    assert_eq!(scope_result, task_count);
    drop(work_tx);

    let mut work = recv_exact(&work_rx, task_count, "scoped work");
    work.sort_by_key(|entry| entry.0);

    for (input, combined, worker_index) in work {
        assert_eq!(combined, input * input + input * input * input);

        let worker_index = worker_index.expect("scoped work should run on a Rayon worker");
        assert!(
            worker_index < thread_count,
            "worker index {worker_index} should be within the configured pool"
        );
    }

    assert!(
        work_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "each scoped task should report exactly once"
    );

    drop(pool);

    let exited: BTreeSet<_> = recv_exact(&exit_rx, thread_count, "exit handler")
        .into_iter()
        .collect();
    assert_eq!(exited, expected_worker_indices(thread_count));
    assert_eq!(exit_calls.load(Ordering::SeqCst), thread_count);

    assert!(
        exit_rx.recv_timeout(Duration::from_millis(100)).is_err(),
        "Configuration::exit_handler should run once per worker"
    );
}

#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn deprecated_configuration_exit_handler_runs_after_handled_detached_panic_and_broadcast() {
    let thread_count = 2usize;

    let (panic_tx, panic_rx) = mpsc::channel::<String>();
    let (exit_tx, exit_rx) = mpsc::channel::<usize>();
    let panic_count = Arc::new(AtomicUsize::new(0));

    let config = rayon_core::Configuration::new().num_threads(thread_count);

    let config = rayon_core::Configuration::panic_handler(config, {
        let panic_count = Arc::clone(&panic_count);

        move |payload| {
            panic_count.fetch_add(1, Ordering::SeqCst);

            let any = &*payload;
            let message = if let Some(message) = any.downcast_ref::<&'static str>() {
                (*message).to_owned()
            } else if let Some(message) = any.downcast_ref::<String>() {
                message.clone()
            } else {
                "<non-string panic payload>".to_owned()
            };

            panic_tx
                .send(message)
                .expect("panic handler should be able to report panic payload");
        }
    });

    let config = rayon_core::Configuration::exit_handler(config, move |index| {
        exit_tx
            .send(index)
            .expect("exit handler should be able to report shutdown");
    });

    let pool =
        rayon_core::Configuration::build(config).expect("configuration should build a thread pool");

    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );

    rayon_core::ThreadPool::spawn(&pool, || {
        panic!("detached panic handled before pool shutdown");
    });

    let panic_message = panic_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("configured panic handler should receive detached task panic");
    assert_eq!(panic_message, "detached panic handled before pool shutdown");
    assert_eq!(panic_count.load(Ordering::SeqCst), 1);

    let (broadcast_tx, broadcast_rx) = mpsc::channel::<(usize, usize, Option<usize>)>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, move |context| {
        broadcast_tx
            .send((
                rayon_core::BroadcastContext::index(&context),
                rayon_core::BroadcastContext::num_threads(&context),
                rayon_core::current_thread_index(),
            ))
            .expect("broadcast worker should be able to report its context");
    });

    let broadcast_records = recv_exact(&broadcast_rx, thread_count, "spawn_broadcast");
    let mut broadcast_indices = BTreeSet::new();

    for (context_index, context_threads, current_index) in broadcast_records {
        assert_eq!(context_threads, thread_count);
        assert_eq!(current_index, Some(context_index));
        assert!(
            broadcast_indices.insert(context_index),
            "worker {context_index} should run broadcast exactly once"
        );
    }

    assert_eq!(broadcast_indices, expected_worker_indices(thread_count));

    let (left, right) = rayon_core::ThreadPool::join(
        &pool,
        || rayon_core::current_thread_index(),
        || rayon_core::current_thread_index(),
    );

    for observed in [left, right] {
        let index = observed.expect("join branch should run inside the custom Rayon pool");
        assert!(index < thread_count);
    }

    drop(pool);

    let exited: BTreeSet<_> = recv_exact(&exit_rx, thread_count, "exit handler")
        .into_iter()
        .collect();
    assert_eq!(exited, expected_worker_indices(thread_count));
}