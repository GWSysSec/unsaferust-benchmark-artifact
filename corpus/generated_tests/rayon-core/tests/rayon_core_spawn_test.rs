use std::collections::BTreeSet;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn free_spawn_fifo_runs_static_work_items_on_global_pool() {
    let (tx, rx) = mpsc::channel::<(usize, usize, Option<usize>, usize)>();

    for input in 0usize..6 {
        let tx = tx.clone();
        rayon_core::spawn_fifo(move || {
            let (square, cube) = rayon_core::join(|| input * input, || input * input * input);
            let thread_index = rayon_core::current_thread_index();
            let thread_count = rayon_core::current_num_threads();
            tx.send((input, square + cube, thread_index, thread_count))
                .expect("receiver should still be alive");
        });
    }
    drop(tx);

    let mut observed = Vec::new();
    for _ in 0..6 {
        observed.push(
            rx.recv_timeout(Duration::from_secs(10))
                .expect("spawn_fifo work item should complete"),
        );
    }

    observed.sort_by_key(|entry| entry.0);

    assert_eq!(observed.len(), 6);
    for (input, combined, thread_index, thread_count) in observed {
        assert_eq!(combined, input * input + input * input * input);
        assert!(thread_count >= 1);
        if let Some(index) = thread_index {
            assert!(index < thread_count);
        }
    }

    assert!(
        rx.recv_timeout(Duration::from_millis(50)).is_err(),
        "all scheduled work items should have been received exactly once"
    );
}

#[test]
fn thread_pool_spawn_fifo_scope_and_broadcast_complete_consistently() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(4)
        .thread_name(|index| format!("fifo-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    assert_eq!(pool.current_num_threads(), 4);
    assert_eq!(pool.current_thread_index(), None);
    assert_eq!(pool.current_thread_has_pending_tasks(), None);

    let (tx, rx) = mpsc::channel::<usize>();

    for input in 1usize..=8 {
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            let (left, right) = rayon_core::join(
                || input.checked_mul(10).expect("small multiplication"),
                || input.checked_mul(100).expect("small multiplication"),
            );
            tx.send(left + right)
                .expect("receiver should still be alive");
        });
    }
    drop(tx);

    let mut values = Vec::new();
    for _ in 0..8 {
        values.push(
            rx.recv_timeout(Duration::from_secs(10))
                .expect("pool spawn_fifo work item should complete"),
        );
    }
    values.sort_unstable();

    assert_eq!(values, vec![110, 220, 330, 440, 550, 660, 770, 880]);

    let scoped_sum = pool.scope_fifo(|scope| {
        let (tx, rx) = mpsc::channel::<usize>();

        for base in [2usize, 3, 5, 7] {
            let tx = tx.clone();
            scope.spawn_fifo(move |nested_scope| {
                let tx_nested = tx.clone();
                nested_scope.spawn_fifo(move |_| {
                    tx_nested
                        .send(base * base)
                        .expect("scope receiver should still be alive");
                });

                tx.send(base).expect("scope receiver should still be alive");
            });
        }
        drop(tx);

        let mut scoped_values = Vec::new();
        for _ in 0..8 {
            scoped_values.push(
                rx.recv_timeout(Duration::from_secs(10))
                    .expect("all scoped fifo work should complete before scope exits"),
            );
        }

        scoped_values.into_iter().sum::<usize>()
    });

    assert_eq!(scoped_sum, 2 + 4 + 3 + 9 + 5 + 25 + 7 + 49);

    let mut broadcast_results = pool.broadcast(|context| {
        let index = context.index();
        let total = context.num_threads();

        assert!(index < total);
        assert_eq!(total, 4);

        (index, total)
    });

    broadcast_results.sort_unstable();

    let expected: Vec<(usize, usize)> = (0..4).map(|index| (index, 4)).collect();
    assert_eq!(broadcast_results, expected);

    let seen_indices: BTreeSet<usize> = broadcast_results
        .iter()
        .map(|&(index, _)| index)
        .collect();
    assert_eq!(seen_indices, BTreeSet::from([0, 1, 2, 3]));
}