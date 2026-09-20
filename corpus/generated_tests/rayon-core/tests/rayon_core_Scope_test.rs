use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[test]
fn scope_spawn_broadcast_runs_once_per_worker_and_nested_scope_jobs_complete() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(4)
        .thread_name(|index| format!("scope-broadcast-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    let records = Arc::new(Mutex::new(Vec::<(usize, usize, usize, Option<usize>)>::new()));
    let nested_jobs_started = Arc::new(AtomicUsize::new(0));

    let returned = pool.scope({
        let records = Arc::clone(&records);
        let nested_jobs_started = Arc::clone(&nested_jobs_started);

        move |scope| {
            rayon_core::Scope::spawn_broadcast(scope, {
                let records = Arc::clone(&records);
                let nested_jobs_started = Arc::clone(&nested_jobs_started);

                move |scope, context| {
                    let index = context.index();
                    let num_threads = context.num_threads();

                    assert!(index < num_threads);
                    assert_eq!(num_threads, 4);
                    assert_eq!(rayon_core::current_thread_index(), Some(index));
                    assert_eq!(rayon_core::current_num_threads(), num_threads);

                    records.lock().unwrap().push((
                        0,
                        index,
                        num_threads,
                        rayon_core::current_thread_index(),
                    ));

                    let records_for_nested = Arc::clone(&records);
                    let nested_jobs_started = Arc::clone(&nested_jobs_started);

                    scope.spawn(move |_| {
                        nested_jobs_started.fetch_add(1, Ordering::SeqCst);

                        let (left, right) = rayon_core::join(|| index + 1, || num_threads * 10);
                        assert_eq!(left, index + 1);
                        assert_eq!(right, 40);

                        let current_index = rayon_core::current_thread_index();
                        assert!(current_index.is_some());
                        assert!(current_index.unwrap() < num_threads);

                        records_for_nested.lock().unwrap().push((
                            1,
                            index,
                            left + right,
                            current_index,
                        ));
                    });
                }
            });

            12345usize
        }
    });

    assert_eq!(returned, 12345);
    assert_eq!(nested_jobs_started.load(Ordering::SeqCst), 4);

    let records = records.lock().unwrap().clone();
    assert_eq!(records.len(), 8);

    let broadcast_records: Vec<_> = records.iter().copied().filter(|record| record.0 == 0).collect();
    let nested_records: Vec<_> = records.iter().copied().filter(|record| record.0 == 1).collect();

    assert_eq!(broadcast_records.len(), 4);
    assert_eq!(nested_records.len(), 4);

    let mut broadcast_indices = BTreeSet::new();
    for (_, index, num_threads, current_index) in broadcast_records {
        assert_eq!(num_threads, 4);
        assert_eq!(current_index, Some(index));
        assert!(broadcast_indices.insert(index), "worker {index} broadcast more than once");
    }

    assert_eq!(broadcast_indices.len(), 4);
    assert_eq!(broadcast_indices.first().copied(), Some(0));
    assert_eq!(broadcast_indices.last().copied(), Some(3));

    let mut nested_origin_indices = BTreeSet::new();
    for (_, origin_index, computed_value, current_index) in nested_records {
        assert!(origin_index < 4);
        assert_eq!(computed_value, origin_index + 1 + 40);
        assert!(current_index.is_some());
        assert!(current_index.unwrap() < 4);
        assert!(
            nested_origin_indices.insert(origin_index),
            "nested job for broadcast origin {origin_index} ran more than once"
        );
    }

    assert_eq!(nested_origin_indices, broadcast_indices);
}

#[test]
fn scope_spawn_broadcast_handles_single_thread_pool_and_borrowed_scope_data() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("single-thread custom pool should build");

    let events = Mutex::new(Vec::<String>::new());
    let nested_count = AtomicUsize::new(0);

    let scope_value = pool.scope(|scope| {
        events.lock().unwrap().push("scope-entered".to_string());

        rayon_core::Scope::spawn_broadcast(scope, |scope, context| {
            assert_eq!(context.index(), 0);
            assert_eq!(context.num_threads(), 1);
            assert_eq!(rayon_core::current_thread_index(), Some(0));
            assert_eq!(rayon_core::current_num_threads(), 1);

            events.lock().unwrap().push(format!(
                "broadcast:{}-of-{}",
                context.index(),
                context.num_threads()
            ));

            scope.spawn(|_| {
                nested_count.fetch_add(1, Ordering::SeqCst);
                events.lock().unwrap().push("nested-finished".to_string());
            });
        });

        events.lock().unwrap().push("scope-body-finished".to_string());
        "scope-result"
    });

    assert_eq!(scope_value, "scope-result");
    assert_eq!(nested_count.load(Ordering::SeqCst), 1);

    let events = events.lock().unwrap().clone();
    assert_eq!(events.len(), 4);
    assert!(events.contains(&"scope-entered".to_string()));
    assert!(events.contains(&"broadcast:0-of-1".to_string()));
    assert!(events.contains(&"nested-finished".to_string()));
    assert!(events.contains(&"scope-body-finished".to_string()));
}