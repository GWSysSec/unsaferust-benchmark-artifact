use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

#[test]
fn broadcast_reports_every_worker_context_once() {
    let observed = rayon_core::broadcast(|context| {
        let index = context.index();
        let num_threads = context.num_threads();

        assert!(index < num_threads);
        assert_eq!(num_threads, rayon_core::current_num_threads());
        assert_eq!(rayon_core::current_thread_index(), Some(index));

        (index, num_threads)
    });

    let global_threads = rayon_core::current_num_threads();
    assert_eq!(observed.len(), global_threads);
    assert!(global_threads > 0);
    assert!(global_threads <= rayon_core::max_num_threads());

    let mut indices = BTreeSet::new();
    for (index, num_threads) in observed {
        assert_eq!(num_threads, global_threads);
        assert!(indices.insert(index), "worker index {index} was reported more than once");
    }

    assert_eq!(indices.len(), global_threads);
    assert_eq!(indices.first().copied(), Some(0));
    assert_eq!(indices.last().copied(), Some(global_threads - 1));
}

#[test]
fn spawn_broadcast_eventually_runs_once_on_each_worker() {
    let expected_threads = rayon_core::current_num_threads();
    let (sender, receiver) = mpsc::channel::<(usize, usize, Option<usize>)>();
    let run_count = Arc::new(AtomicUsize::new(0));

    rayon_core::spawn_broadcast({
        let run_count = Arc::clone(&run_count);
        move |context| {
            run_count.fetch_add(1, Ordering::SeqCst);
            sender
                .send((
                    context.index(),
                    context.num_threads(),
                    rayon_core::current_thread_index(),
                ))
                .expect("receiver should remain alive while broadcast tasks run");
        }
    });

    let mut indices = BTreeSet::new();
    for _ in 0..expected_threads {
        let (index, num_threads, current_index) = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("spawn_broadcast task did not complete in time");

        assert_eq!(num_threads, expected_threads);
        assert_eq!(current_index, Some(index));
        assert!(index < expected_threads);
        assert!(indices.insert(index), "worker index {index} ran more than once");
    }

    assert_eq!(indices.len(), expected_threads);
    assert_eq!(run_count.load(Ordering::SeqCst), expected_threads);
    assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
}

#[test]
fn local_pool_broadcast_and_spawn_broadcast_are_isolated_to_pool_threads() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(4)
        .thread_name(|index| format!("integration-broadcast-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    assert_eq!(pool.current_num_threads(), 4);
    assert_eq!(pool.current_thread_index(), None);

    let broadcast_results = pool.broadcast(|context| {
        assert_eq!(context.num_threads(), 4);
        assert_eq!(rayon_core::current_num_threads(), 4);
        assert_eq!(rayon_core::current_thread_index(), Some(context.index()));

        context.index()
    });

    let broadcast_indices: BTreeSet<_> = broadcast_results.into_iter().collect();
    assert_eq!(broadcast_indices.len(), 4);
    assert_eq!(broadcast_indices, BTreeSet::from([0, 1, 2, 3]));

    let (sender, receiver) = mpsc::channel::<usize>();
    pool.spawn_broadcast(move |context| {
        assert_eq!(context.num_threads(), 4);
        sender
            .send(context.index())
            .expect("receiver should remain alive for local pool broadcast");
    });

    let mut spawned_indices = BTreeSet::new();
    for _ in 0..4 {
        let index = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("local spawn_broadcast task did not complete in time");
        assert!(spawned_indices.insert(index));
    }

    assert_eq!(spawned_indices, BTreeSet::from([0, 1, 2, 3]));
    assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
}