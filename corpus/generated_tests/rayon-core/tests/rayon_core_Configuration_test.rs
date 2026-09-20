#![allow(deprecated)]

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn deprecated_configuration_handlers_drive_custom_pool_lifecycle_and_panics() {
    let start_indices = Arc::new(Mutex::new(Vec::<usize>::new()));
    let exit_indices = Arc::new(Mutex::new(Vec::<usize>::new()));
    let panic_count = Arc::new(AtomicUsize::new(0));
    let name_call_count = Arc::new(AtomicUsize::new(0));

    let config = rayon_core::Configuration::new().num_threads(2);

    let config = rayon_core::Configuration::thread_name(config, {
        let name_call_count = Arc::clone(&name_call_count);
        move |index| {
            name_call_count.fetch_add(1, Ordering::SeqCst);
            format!("configuration-test-worker-{index}")
        }
    });

    let config = rayon_core::Configuration::start_handler(config, {
        let start_indices = Arc::clone(&start_indices);
        move |index| {
            start_indices
                .lock()
                .expect("start handler mutex should not be poisoned")
                .push(index);
        }
    });

    let config = rayon_core::Configuration::exit_handler(config, {
        let exit_indices = Arc::clone(&exit_indices);
        move |index| {
            exit_indices
                .lock()
                .expect("exit handler mutex should not be poisoned")
                .push(index);
        }
    });

    let config = rayon_core::Configuration::panic_handler(config, {
        let panic_count = Arc::clone(&panic_count);
        move |payload| {
            if payload.is::<&'static str>() || payload.is::<String>() {
                panic_count.fetch_add(1, Ordering::SeqCst);
            }
        }
    });

    let pool = rayon_core::Configuration::build(config).expect("configuration should build a pool");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 2);

    let mut worker_observations = rayon_core::ThreadPool::broadcast(&pool, |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);
        let current_index = rayon_core::current_thread_index();
        let current_name = thread::current()
            .name()
            .map(str::to_owned)
            .expect("configured worker thread should have a name");

        (index, num_threads, current_index, current_name)
    });

    worker_observations.sort_by_key(|entry| entry.0);

    assert_eq!(worker_observations.len(), 2);
    for (index, num_threads, current_index, current_name) in &worker_observations {
        assert_eq!(*num_threads, 2);
        assert_eq!(*current_index, Some(*index));
        assert_eq!(
            current_name,
            &format!("configuration-test-worker-{index}")
        );
    }

    let observed_indices: BTreeSet<usize> = worker_observations
        .iter()
        .map(|(index, _, _, _)| *index)
        .collect();
    assert_eq!(observed_indices, BTreeSet::from([0, 1]));

    let started = {
        let mut started = start_indices
            .lock()
            .expect("start handler mutex should not be poisoned")
            .clone();
        started.sort_unstable();
        started
    };
    assert_eq!(started, vec![0, 1]);

    assert!(
        name_call_count.load(Ordering::SeqCst) >= 2,
        "thread_name closure should be called for the worker threads"
    );

    rayon_core::ThreadPool::spawn(&pool, || {
        panic!("intentional panic handled by deprecated Configuration::panic_handler");
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    while panic_count.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        rayon_core::ThreadPool::yield_now(&pool);
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(panic_count.load(Ordering::SeqCst), 1);

    drop(pool);

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let exited_len = exit_indices
            .lock()
            .expect("exit handler mutex should not be poisoned")
            .len();

        if exited_len == 2 || Instant::now() >= deadline {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }

    let mut exited = exit_indices
        .lock()
        .expect("exit handler mutex should not be poisoned")
        .clone();
    exited.sort_unstable();
    assert_eq!(exited, vec![0, 1]);
}