use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[test]
fn thread_pool_builder_handlers_observe_lifecycle_names_and_spawn_panics() {
    let thread_count = 3usize;

    let name_calls = Arc::new(AtomicUsize::new(0));
    let start_indices = Arc::new(Mutex::new(Vec::<usize>::new()));
    let exit_indices = Arc::new(Mutex::new(Vec::<usize>::new()));

    let (panic_tx, panic_rx) = mpsc::channel::<String>();
    let (exit_tx, exit_rx) = mpsc::channel::<usize>();

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name({
            let name_calls = Arc::clone(&name_calls);
            move |index| {
                name_calls.fetch_add(1, Ordering::SeqCst);
                format!("builder-handler-worker-{index}")
            }
        })
        .panic_handler({
            let panic_tx = panic_tx.clone();
            move |payload| {
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
        })
        .start_handler({
            let start_indices = Arc::clone(&start_indices);
            move |index| {
                start_indices
                    .lock()
                    .expect("start handler mutex should not be poisoned")
                    .push(index);
            }
        })
        .exit_handler({
            let exit_indices = Arc::clone(&exit_indices);
            let exit_tx = exit_tx.clone();
            move |index| {
                exit_indices
                    .lock()
                    .expect("exit handler mutex should not be poisoned")
                    .push(index);
                exit_tx
                    .send(index)
                    .expect("exit handler should be able to report index");
            }
        })
        .build()
        .expect("custom pool with all builder handlers should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), thread_count);

    let (name_tx, name_rx) = mpsc::channel::<(usize, String, Option<usize>, usize)>();
    rayon_core::ThreadPool::spawn_broadcast(&pool, move |context| {
        let index = rayon_core::BroadcastContext::index(&context);
        let num_threads = rayon_core::BroadcastContext::num_threads(&context);
        let current_index = rayon_core::current_thread_index();
        let current_name = std::thread::current()
            .name()
            .expect("worker thread should have a configured name")
            .to_owned();

        name_tx
            .send((index, current_name, current_index, num_threads))
            .expect("broadcast worker should be able to report thread metadata");
    });

    let mut observed_names = Vec::new();
    for _ in 0..thread_count {
        observed_names.push(
            name_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("each worker should run the broadcast job"),
        );
    }

    observed_names.sort_by_key(|record| record.0);
    for (index, name, current_index, num_threads) in &observed_names {
        assert_eq!(*num_threads, thread_count);
        assert_eq!(*current_index, Some(*index));
        assert_eq!(name, &format!("builder-handler-worker-{index}"));
    }

    let observed_name_indices: BTreeSet<usize> =
        observed_names.iter().map(|(index, _, _, _)| *index).collect();
    assert_eq!(observed_name_indices, BTreeSet::from([0usize, 1, 2]));
    assert!(
        name_calls.load(Ordering::SeqCst) >= thread_count,
        "thread_name closure should be called for worker threads"
    );

    let (sum, product) = rayon_core::ThreadPool::join(
        &pool,
        || (1usize..=5).sum::<usize>(),
        || (1usize..=5).product::<usize>(),
    );
    assert_eq!(sum, 15);
    assert_eq!(product, 120);

    let start_set: BTreeSet<usize> = start_indices
        .lock()
        .expect("start handler mutex should not be poisoned")
        .iter()
        .copied()
        .collect();
    assert_eq!(start_set, BTreeSet::from([0usize, 1, 2]));

    rayon_core::ThreadPool::spawn(&pool, || {
        panic!("spawn panic handled by ThreadPoolBuilder::panic_handler");
    });

    let panic_message = panic_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("spawn panic should be delivered to the configured panic handler");
    assert_eq!(
        panic_message,
        "spawn panic handled by ThreadPoolBuilder::panic_handler"
    );

    drop(pool);

    let mut exited_from_channel = Vec::new();
    for _ in 0..thread_count {
        exited_from_channel.push(
            exit_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("each worker should invoke the exit handler when the pool is dropped"),
        );
    }

    let exit_set_from_channel: BTreeSet<usize> = exited_from_channel.into_iter().collect();
    assert_eq!(exit_set_from_channel, BTreeSet::from([0usize, 1, 2]));

    let exit_set_from_mutex: BTreeSet<usize> = exit_indices
        .lock()
        .expect("exit handler mutex should not be poisoned")
        .iter()
        .copied()
        .collect();
    assert_eq!(exit_set_from_mutex, BTreeSet::from([0usize, 1, 2]));
}

#[test]
fn builder_handlers_still_allow_pool_work_after_a_handled_spawn_panic() {
    let thread_count = 2usize;

    let panic_count = Arc::new(AtomicUsize::new(0));
    let starts = Arc::new(AtomicUsize::new(0));
    let exits = Arc::new(AtomicUsize::new(0));
    let (panic_tx, panic_rx) = mpsc::channel::<()>();
    let (exit_tx, exit_rx) = mpsc::channel::<usize>();

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|index| format!("recovering-worker-{index}"))
        .panic_handler({
            let panic_count = Arc::clone(&panic_count);
            move |_| {
                panic_count.fetch_add(1, Ordering::SeqCst);
                panic_tx
                    .send(())
                    .expect("panic handler should be able to signal completion");
            }
        })
        .start_handler({
            let starts = Arc::clone(&starts);
            move |_| {
                starts.fetch_add(1, Ordering::SeqCst);
            }
        })
        .exit_handler({
            let exits = Arc::clone(&exits);
            move |index| {
                exits.fetch_add(1, Ordering::SeqCst);
                exit_tx
                    .send(index)
                    .expect("exit handler should be able to signal worker shutdown");
            }
        })
        .build()
        .expect("pool should build with all handlers configured");

    rayon_core::ThreadPool::spawn(&pool, || {
        panic!("first detached job panics");
    });

    panic_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("detached panic should be handled");

    assert_eq!(panic_count.load(Ordering::SeqCst), 1);

    let collected = rayon_core::ThreadPool::scope(&pool, |scope| {
        let values = Arc::new(Mutex::new(Vec::<usize>::new()));

        for value in 0..8usize {
            let values = Arc::clone(&values);
            rayon_core::Scope::spawn(scope, move |_| {
                values
                    .lock()
                    .expect("values mutex should not be poisoned")
                    .push(value * value);
            });
        }

        values
    });

    let mut squares = Arc::try_unwrap(collected)
        .expect("all scope jobs should have released their Arc clones")
        .into_inner()
        .expect("values mutex should not be poisoned");
    squares.sort_unstable();

    assert_eq!(squares, vec![0, 1, 4, 9, 16, 25, 36, 49]);
    assert_eq!(
        rayon_core::ThreadPool::current_num_threads(&pool),
        thread_count
    );
    assert!(
        starts.load(Ordering::SeqCst) >= thread_count,
        "start handler should have run for worker threads"
    );

    drop(pool);

    let mut exited_workers = Vec::new();
    for _ in 0..thread_count {
        exited_workers.push(
            exit_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("exit handler should run once per worker when the pool is dropped"),
        );
    }

    let exited_worker_set: BTreeSet<usize> = exited_workers.into_iter().collect();
    assert_eq!(exited_worker_set, BTreeSet::from([0usize, 1]));

    assert_eq!(
        exits.load(Ordering::SeqCst),
        thread_count,
        "exit handler should run once per worker when the pool is dropped"
    );
}