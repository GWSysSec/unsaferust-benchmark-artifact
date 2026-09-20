use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

fn quick_sort<T: PartialOrd + Send>(values: &mut [T]) {
    if values.len() > 1 {
        let mid = partition(values);
        let (lo, hi) = values.split_at_mut(mid);
        rayon_core::join(|| quick_sort(lo), || quick_sort(hi));
    }
}

fn partition<T: PartialOrd + Send>(values: &mut [T]) -> usize {
    let pivot = values.len() - 1;
    let mut i = 0;
    for j in 0..pivot {
        if values[j] <= values[pivot] {
            values.swap(i, j);
            i += 1;
        }
    }
    values.swap(i, pivot);
    i
}

#[test]
fn global_join_broadcast_and_context_workflow() {
    let init_result = rayon_core::initialize(rayon_core::Configuration::new().num_threads(2));
    assert!(
        init_result.is_ok() || init_result.is_err(),
        "initialize should return a Result without panicking"
    );

    let mut values = vec![9, 1, 5, 3, 7, 3, 0, 8, 2, 6, 4];
    quick_sort(&mut values);
    assert_eq!(values, vec![0, 1, 2, 3, 3, 4, 5, 6, 7, 8, 9]);

    let (sum, product) = rayon_core::join(
        || values.iter().copied().sum::<i32>(),
        || values.iter().copied().product::<i32>(),
    );
    assert_eq!(sum, 48);
    assert_eq!(product, 0);

    let ((left_index, left_migrated), (right_index, right_migrated)) =
        rayon_core::join_context(
            |context| (rayon_core::current_thread_index(), context.migrated()),
            |context| (rayon_core::current_thread_index(), context.migrated()),
        );
    assert_eq!(left_migrated, left_index.is_some());
    assert_eq!(right_migrated, right_index.is_some());

    let observed = rayon_core::broadcast(|context| {
        let index = context.index();
        let num_threads = context.num_threads();
        assert!(index < num_threads);
        assert_eq!(rayon_core::current_thread_index(), Some(index));
        assert_eq!(rayon_core::current_thread_has_pending_tasks(), Some(false));
        (index, num_threads)
    });

    let thread_count = rayon_core::current_num_threads();
    assert_eq!(observed.len(), thread_count);
    assert!(thread_count > 0);

    let mut indices = BTreeSet::new();
    for (index, num_threads) in observed {
        assert_eq!(num_threads, thread_count);
        assert!(indices.insert(index));
    }
    assert_eq!(indices.len(), thread_count);
}

#[test]
fn scopes_can_spawn_nested_stack_borrowing_work() {
    let results = Mutex::new(Vec::new());

    let returned = rayon_core::scope(|scope| {
        for value in 0..6 {
            let results = &results;
            scope.spawn(move |nested_scope| {
                results.lock().unwrap().push(value);
                if value % 2 == 0 {
                    nested_scope.spawn(move |_| {
                        results.lock().unwrap().push(value * 10);
                    });
                }
            });
        }
        1234
    });

    assert_eq!(returned, 1234);

    let mut collected = results.lock().unwrap().clone();
    collected.sort_unstable();
    assert_eq!(collected, vec![0, 0, 1, 2, 3, 4, 5, 20, 40]);

    let fifo_results = Mutex::new(Vec::new());
    let fifo_returned = rayon_core::scope_fifo(|scope| {
        for value in 1..=4 {
            let fifo_results = &fifo_results;
            scope.spawn_fifo(move |nested_scope| {
                fifo_results.lock().unwrap().push(value);
                if value == 4 {
                    nested_scope.spawn_fifo(move |_| {
                        fifo_results.lock().unwrap().push(40);
                    });
                }
            });
        }
        "fifo-complete"
    });

    assert_eq!(fifo_returned, "fifo-complete");

    let mut fifo_collected = fifo_results.lock().unwrap().clone();
    fifo_collected.sort_unstable();
    assert_eq!(fifo_collected, vec![1, 2, 3, 4, 40]);
}

#[test]
fn in_place_scopes_run_caller_closure_and_wait_for_spawned_work() {
    let caller_thread = std::thread::current().id();
    let results = Mutex::new(Vec::new());

    let in_place_value = rayon_core::in_place_scope(|scope| {
        assert_eq!(std::thread::current().id(), caller_thread);
        for value in [10, 20, 30] {
            let results = &results;
            scope.spawn(move |_| {
                results.lock().unwrap().push(value);
            });
        }
        77
    });

    assert_eq!(in_place_value, 77);

    let mut collected = results.lock().unwrap().clone();
    collected.sort_unstable();
    assert_eq!(collected, vec![10, 20, 30]);

    let fifo_results = Mutex::new(Vec::new());
    let fifo_value = rayon_core::in_place_scope_fifo(|scope| {
        assert_eq!(std::thread::current().id(), caller_thread);
        for value in [3, 1, 2] {
            let fifo_results = &fifo_results;
            scope.spawn_fifo(move |_| {
                fifo_results.lock().unwrap().push(value);
            });
        }
        88
    });

    assert_eq!(fifo_value, 88);

    let mut fifo_collected = fifo_results.lock().unwrap().clone();
    fifo_collected.sort_unstable();
    assert_eq!(fifo_collected, vec![1, 2, 3]);
}

#[test]
fn global_spawn_fifo_and_spawn_broadcast_eventually_run() {
    let (fifo_sender, fifo_receiver) = mpsc::channel();
    for value in 0..5 {
        let fifo_sender = fifo_sender.clone();
        rayon_core::spawn_fifo(move || {
            fifo_sender.send(value * value).unwrap();
        });
    }
    drop(fifo_sender);

    let mut squares = Vec::new();
    for _ in 0..5 {
        squares.push(
            fifo_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("spawn_fifo task should finish"),
        );
    }
    squares.sort_unstable();
    assert_eq!(squares, vec![0, 1, 4, 9, 16]);

    let expected_threads = rayon_core::current_num_threads();
    let (broadcast_sender, broadcast_receiver) = mpsc::channel();

    rayon_core::spawn_broadcast(move |context| {
        broadcast_sender
            .send((context.index(), context.num_threads(), rayon_core::current_thread_index()))
            .unwrap();
    });

    let mut seen = BTreeSet::new();
    for _ in 0..expected_threads {
        let (index, num_threads, current_index) = broadcast_receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("spawn_broadcast task should finish on each worker");
        assert_eq!(num_threads, expected_threads);
        assert_eq!(current_index, Some(index));
        assert!(seen.insert(index));
    }

    assert_eq!(seen.len(), expected_threads);
}

#[test]
fn builder_handlers_spawn_handler_and_pool_workflow() {
    let starts = Arc::new(AtomicUsize::new(0));
    let exits = Arc::new(AtomicUsize::new(0));
    let panics = Arc::new(AtomicUsize::new(0));

    let starts_for_handler = Arc::clone(&starts);
    let exits_for_handler = Arc::clone(&exits);
    let panics_for_handler = Arc::clone(&panics);

    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(2)
        .thread_name(|index| format!("integration-worker-{index}"))
        .start_handler(move |_| {
            starts_for_handler.fetch_add(1, Ordering::SeqCst);
        })
        .exit_handler(move |_| {
            exits_for_handler.fetch_add(1, Ordering::SeqCst);
        })
        .panic_handler(move |_| {
            panics_for_handler.fetch_add(1, Ordering::SeqCst);
        })
        .spawn_handler(|thread| {
            let name = thread.name().map(str::to_owned);
            let stack_size = thread.stack_size();
            let mut builder = std::thread::Builder::new();
            if let Some(name) = name {
                builder = builder.name(name);
            }
            if let Some(stack_size) = stack_size {
                builder = builder.stack_size(stack_size);
            }
            builder.spawn(move || thread.run()).map(|_| ())
        })
        .build()
        .expect("custom thread pool should build");

    assert_eq!(pool.current_num_threads(), 2);

    let names = pool.broadcast(|context| {
        assert_eq!(context.num_threads(), 2);
        assert_eq!(pool.current_thread_index(), Some(context.index()));
        std::thread::current().name().map(str::to_owned)
    });

    assert_eq!(names.len(), 2);
    for name in names {
        let name = name.expect("custom thread should have configured name");
        assert!(name.starts_with("integration-worker-"));
    }

    let (panic_sender, panic_receiver) = mpsc::channel();
    let panics_for_wait = Arc::clone(&panics);
    pool.spawn(move || {
        let _ = panic_sender.send(());
        panic!("intentional panic for panic_handler coverage");
    });

    panic_receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("panicking task should start");

    for _ in 0..50 {
        if panics_for_wait.load(Ordering::SeqCst) >= 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(panics.load(Ordering::SeqCst), 1);
    assert!(starts.load(Ordering::SeqCst) >= 2);

    drop(pool);

    for _ in 0..50 {
        if exits.load(Ordering::SeqCst) >= 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(exits.load(Ordering::SeqCst), 2);
}

#[test]
fn build_scoped_allows_using_pool_during_scoped_lifetime() {
    let result = rayon_core::ThreadPoolBuilder::new()
        .num_threads(2)
        .thread_name(|index| format!("scoped-worker-{index}"))
        .build_scoped(
            |thread| thread.run(),
            |pool| {
                let thread_count = pool.current_num_threads();
                let broadcast_indices = pool.broadcast(|context| {
                    assert_eq!(context.num_threads(), thread_count);
                    assert_eq!(pool.current_thread_index(), Some(context.index()));
                    context.index()
                });

                let mut sorted = broadcast_indices;
                sorted.sort_unstable();

                let (a, b) = pool.join(
                    || sorted.iter().copied().sum::<usize>(),
                    || sorted.iter().copied().max(),
                );

                (thread_count, sorted, a, b)
            },
        )
        .expect("scoped thread pool should build and run");

    let (thread_count, indices, sum, max) = result;
    assert_eq!(thread_count, 2);
    assert_eq!(indices, vec![0, 1]);
    assert_eq!(sum, 1);
    assert_eq!(max, Some(1));
}