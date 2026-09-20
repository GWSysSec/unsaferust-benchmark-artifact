use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[test]
fn thread_pool_join_and_scopes_complete_nested_workflows() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(4)
        .thread_name(|index| format!("integration-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    assert_eq!(rayon_core::ThreadPool::current_num_threads(&pool), 4);
    assert_eq!(
        rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool),
        None,
        "outside any rayon worker, pending-task status is unavailable"
    );

    let (left, right) = rayon_core::ThreadPool::join(
        &pool,
        || {
            let pending = rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool);
            let index = rayon_core::ThreadPool::current_thread_index(&pool);
            let (a, b) = rayon_core::join(|| 21usize, || 2usize);
            (pending, index, a * b)
        },
        || {
            let pending = rayon_core::ThreadPool::current_thread_has_pending_tasks(&pool);
            let index = rayon_core::ThreadPool::current_thread_index(&pool);
            let sum = (1usize..=5).sum::<usize>();
            (pending, index, sum)
        },
    );

    assert!(left.0.is_some(), "join branch should run in the custom pool");
    assert!(right.0.is_some(), "join branch should run in the custom pool");
    assert!(left.1.unwrap() < 4);
    assert!(right.1.unwrap() < 4);
    assert_eq!(left.2, 42);
    assert_eq!(right.2, 15);

    let scoped_values = Mutex::new(Vec::<usize>::new());
    let scoped_result = rayon_core::ThreadPool::scope(&pool, |scope| {
        scoped_values.lock().unwrap().push(10);

        rayon_core::Scope::spawn(scope, |nested_scope| {
            scoped_values.lock().unwrap().push(20);

            rayon_core::Scope::spawn(nested_scope, |_| {
                let (a, b) = rayon_core::join(|| 3usize, || 4usize);
                scoped_values.lock().unwrap().push(a * b);
            });
        });

        rayon_core::Scope::spawn(scope, |_| {
            scoped_values.lock().unwrap().push(30);
        });

        99usize
    });

    assert_eq!(scoped_result, 99);
    let mut scoped_values = scoped_values.into_inner().unwrap();
    scoped_values.sort_unstable();
    assert_eq!(scoped_values, vec![10, 12, 20, 30]);

    let fifo_values = Mutex::new(Vec::<usize>::new());
    let fifo_result = rayon_core::ThreadPool::scope_fifo(&pool, |scope| {
        fifo_values.lock().unwrap().push(100);

        rayon_core::ScopeFifo::spawn_fifo(scope, |nested_scope| {
            fifo_values.lock().unwrap().push(200);

            rayon_core::ScopeFifo::spawn_fifo(nested_scope, |_| {
                fifo_values.lock().unwrap().push(300);
            });
        });

        rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
            fifo_values.lock().unwrap().push(400);
        });

        "fifo scope finished"
    });

    assert_eq!(fifo_result, "fifo scope finished");
    let mut fifo_values = fifo_values.into_inner().unwrap();
    fifo_values.sort_unstable();
    assert_eq!(fifo_values, vec![100, 200, 300, 400]);
}

#[test]
fn in_place_scopes_and_async_fifo_broadcast_tasks_run_on_pool() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(3)
        .thread_name(|index| format!("integration-async-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    let in_place_values = Mutex::new(Vec::<usize>::new());
    let in_place_return = rayon_core::ThreadPool::in_place_scope(&pool, |scope| {
        in_place_values.lock().unwrap().push(1);

        rayon_core::Scope::spawn(scope, |_| {
            in_place_values.lock().unwrap().push(2);
        });

        rayon_core::Scope::spawn(scope, |nested_scope| {
            in_place_values.lock().unwrap().push(3);

            rayon_core::Scope::spawn(nested_scope, |_| {
                in_place_values.lock().unwrap().push(4);
            });
        });

        123usize
    });

    assert_eq!(in_place_return, 123);
    let mut in_place_values = in_place_values.into_inner().unwrap();
    in_place_values.sort_unstable();
    assert_eq!(in_place_values, vec![1, 2, 3, 4]);

    let in_place_fifo_values = Mutex::new(Vec::<usize>::new());
    let in_place_fifo_return = rayon_core::ThreadPool::in_place_scope_fifo(&pool, |scope| {
        in_place_fifo_values.lock().unwrap().push(11);

        rayon_core::ScopeFifo::spawn_fifo(scope, |nested_scope| {
            in_place_fifo_values.lock().unwrap().push(22);

            rayon_core::ScopeFifo::spawn_fifo(nested_scope, |_| {
                in_place_fifo_values.lock().unwrap().push(33);
            });
        });

        rayon_core::ScopeFifo::spawn_fifo(scope, |_| {
            in_place_fifo_values.lock().unwrap().push(44);
        });

        456usize
    });

    assert_eq!(in_place_fifo_return, 456);
    let mut in_place_fifo_values = in_place_fifo_values.into_inner().unwrap();
    in_place_fifo_values.sort_unstable();
    assert_eq!(in_place_fifo_values, vec![11, 22, 33, 44]);

    let (fifo_tx, fifo_rx) = mpsc::channel::<(Option<usize>, Option<bool>, usize)>();
    rayon_core::ThreadPool::spawn_fifo(&pool, move || {
        let index = rayon_core::current_thread_index();
        let pending = rayon_core::current_thread_has_pending_tasks();
        fifo_tx
            .send((index, pending, 7usize * 8usize))
            .expect("receiver should still be alive");
    });

    let (fifo_index, fifo_pending, fifo_value) = fifo_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("spawn_fifo task should complete");
    assert!(fifo_index.unwrap() < 3);
    assert!(fifo_pending.is_some());
    assert_eq!(fifo_value, 56);

    let broadcast_count = Arc::new(AtomicUsize::new(0));
    let broadcast_indices = Arc::new(Mutex::new(BTreeSet::<usize>::new()));
    let (broadcast_tx, broadcast_rx) = mpsc::channel::<()>();

    rayon_core::ThreadPool::spawn_broadcast(&pool, {
        let broadcast_count = Arc::clone(&broadcast_count);
        let broadcast_indices = Arc::clone(&broadcast_indices);
        let broadcast_tx = broadcast_tx.clone();

        move |context| {
            assert_eq!(context.num_threads(), 3);
            assert!(context.index() < context.num_threads());

            broadcast_indices.lock().unwrap().insert(context.index());

            if broadcast_count.fetch_add(1, Ordering::SeqCst) + 1 == context.num_threads() {
                broadcast_tx
                    .send(())
                    .expect("broadcast completion receiver should still be alive");
            }
        }
    });

    broadcast_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("spawn_broadcast should run once on every pool thread");

    assert_eq!(broadcast_count.load(Ordering::SeqCst), 3);
    let observed_indices = broadcast_indices.lock().unwrap().clone();
    assert_eq!(observed_indices, BTreeSet::from([0usize, 1usize, 2usize]));
}