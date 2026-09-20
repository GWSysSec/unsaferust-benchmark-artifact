use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[test]
fn module_scoped_apis_run_nested_workflows_to_completion() {
    let values = Mutex::new(Vec::<usize>::new());

    let returned = rayon_core::scope(|scope| {
        values.lock().unwrap().push(0);

        scope.spawn(|nested_scope| {
            values.lock().unwrap().push(1);

            nested_scope.spawn(|_| {
                values.lock().unwrap().push(2);
            });
        });

        scope.spawn(|_| {
            let (left, right) = rayon_core::join(|| 10usize + 1, || 20usize + 2);
            values.lock().unwrap().push(left + right);
        });

        42usize
    });

    assert_eq!(returned, 42);

    let mut observed = values.lock().unwrap().clone();
    observed.sort_unstable();
    assert_eq!(observed, vec![0, 1, 2, 33]);

    let fifo_values = Mutex::new(Vec::<usize>::new());

    let fifo_returned = rayon_core::scope_fifo(|scope| {
        fifo_values.lock().unwrap().push(100);

        scope.spawn_fifo(|nested_scope| {
            fifo_values.lock().unwrap().push(101);

            nested_scope.spawn_fifo(|_| {
                fifo_values.lock().unwrap().push(102);
            });
        });

        scope.spawn_fifo(|_| {
            fifo_values.lock().unwrap().push(103);
        });

        "scope_fifo complete"
    });

    assert_eq!(fifo_returned, "scope_fifo complete");

    let mut fifo_observed = fifo_values.lock().unwrap().clone();
    fifo_observed.sort_unstable();
    assert_eq!(fifo_observed, vec![100, 101, 102, 103]);
}

#[test]
fn module_in_place_scopes_can_borrow_stack_data_and_join_inner_work() {
    let mut words = vec!["rayon".to_string(), "core".to_string()];

    let total_len = rayon_core::in_place_scope(|scope| {
        let suffixes = Arc::new(Mutex::new(Vec::<&'static str>::new()));

        {
            let suffixes = Arc::clone(&suffixes);
            scope.spawn(move |_| {
                suffixes.lock().unwrap().push("-scope");
            });
        }

        {
            let suffixes = Arc::clone(&suffixes);
            scope.spawn(move |_| {
                suffixes.lock().unwrap().push("-api");
            });
        }

        words.push("integration".to_string());

        let (left, right) = rayon_core::join(
            || words.iter().map(String::len).sum::<usize>(),
            || words.len(),
        );

        left + right
    });

    assert_eq!(words, vec!["rayon", "core", "integration"]);
    assert_eq!(total_len, "rayoncoreintegration".len() + 3);

    let mut numbers = vec![3usize, 1, 4];

    let fifo_sum = rayon_core::in_place_scope_fifo(|scope| {
        let additions = Arc::new(Mutex::new(Vec::<usize>::new()));

        {
            let additions = Arc::clone(&additions);
            scope.spawn_fifo(move |_| {
                additions.lock().unwrap().push(1);
            });
        }

        {
            let additions = Arc::clone(&additions);
            scope.spawn_fifo(move |nested_scope| {
                additions.lock().unwrap().push(5);

                let additions = Arc::clone(&additions);
                nested_scope.spawn_fifo(move |_| {
                    additions.lock().unwrap().push(9);
                });
            });
        }

        numbers.push(1);
        numbers.iter().sum::<usize>()
    });

    assert_eq!(numbers, vec![3, 1, 4, 1]);
    assert_eq!(fifo_sum, 9);
}

#[test]
fn module_spawn_fifo_runs_static_task_and_reports_pool_context() {
    let (sender, receiver) = mpsc::channel::<(Option<usize>, usize, bool)>();

    rayon_core::spawn_fifo(move || {
        let index = rayon_core::current_thread_index();
        let num_threads = rayon_core::current_num_threads();
        let has_pending_result = rayon_core::current_thread_has_pending_tasks().is_some();

        sender
            .send((index, num_threads, has_pending_result))
            .expect("receiver should still be alive");
    });

    let (index, num_threads, has_pending_result) = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("spawn_fifo task should run promptly");

    assert!(num_threads > 0);
    assert!(num_threads <= rayon_core::max_num_threads());

    if let Some(index) = index {
        assert!(index < num_threads);
        assert!(has_pending_result);
    }
}

#[test]
fn module_spawn_broadcast_runs_once_for_each_worker_context() {
    let expected_threads = rayon_core::current_num_threads();
    assert!(expected_threads > 0);

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
                .expect("receiver should still be alive");
        }
    });

    let mut observed = BTreeSet::new();

    for _ in 0..expected_threads {
        let (context_index, context_threads, current_index) = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("spawn_broadcast should run on every worker");

        assert_eq!(context_threads, expected_threads);
        assert!(context_index < context_threads);
        assert_eq!(current_index, Some(context_index));
        assert!(
            observed.insert(context_index),
            "worker index {context_index} should be observed only once"
        );
    }

    assert_eq!(observed.len(), expected_threads);
    assert_eq!(observed.first().copied(), Some(0));
    assert_eq!(observed.last().copied(), Some(expected_threads - 1));
    assert_eq!(run_count.load(Ordering::SeqCst), expected_threads);
    assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
}