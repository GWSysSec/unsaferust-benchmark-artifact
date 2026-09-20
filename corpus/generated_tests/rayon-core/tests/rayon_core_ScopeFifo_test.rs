use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[test]
fn scope_fifo_spawn_fifo_nested_jobs_complete_before_scope_returns() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(3)
        .thread_name(|index| format!("scope-fifo-spawn-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    let events = Arc::new(Mutex::new(Vec::<usize>::new()));
    let nested_counter = Arc::new(AtomicUsize::new(0));

    let returned = pool.scope_fifo({
        let events = Arc::clone(&events);
        let nested_counter = Arc::clone(&nested_counter);

        move |scope| {
            events.lock().unwrap().push(10);

            rayon_core::ScopeFifo::spawn_fifo(scope, {
                let events = Arc::clone(&events);
                let nested_counter = Arc::clone(&nested_counter);

                move |nested_scope| {
                    events.lock().unwrap().push(20);
                    nested_counter.fetch_add(1, Ordering::SeqCst);

                    rayon_core::ScopeFifo::spawn_fifo(nested_scope, {
                        let events = Arc::clone(&events);
                        let nested_counter = Arc::clone(&nested_counter);

                        move |_| {
                            events.lock().unwrap().push(30);
                            nested_counter.fetch_add(1, Ordering::SeqCst);

                            let (left, right) = rayon_core::join(|| 7usize, || 11usize);
                            events.lock().unwrap().push(left + right);
                        }
                    });
                }
            });

            rayon_core::ScopeFifo::spawn_fifo(scope, {
                let events = Arc::clone(&events);

                move |_| {
                    let thread_index = rayon_core::current_thread_index();
                    assert!(
                        thread_index.is_some(),
                        "spawned FIFO jobs should execute in a Rayon worker"
                    );
                    assert_eq!(rayon_core::current_num_threads(), 3);
                    events.lock().unwrap().push(40);
                }
            });

            99usize
        }
    });

    assert_eq!(returned, 99);
    assert_eq!(nested_counter.load(Ordering::SeqCst), 2);

    let mut observed = events.lock().unwrap().clone();
    observed.sort_unstable();

    assert_eq!(observed.len(), 5);
    assert_eq!(observed, vec![10, 18, 20, 30, 40]);
}

#[test]
fn scope_fifo_spawn_broadcast_runs_once_per_worker_and_can_enqueue_fifo_work() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(4)
        .thread_name(|index| format!("scope-fifo-broadcast-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    let broadcast_records = Arc::new(Mutex::new(Vec::<(usize, usize, Option<usize>)>::new()));
    let nested_records = Arc::new(Mutex::new(Vec::<(usize, Option<usize>)>::new()));
    let nested_jobs = Arc::new(AtomicUsize::new(0));

    let returned = pool.scope_fifo({
        let broadcast_records = Arc::clone(&broadcast_records);
        let nested_records = Arc::clone(&nested_records);
        let nested_jobs = Arc::clone(&nested_jobs);

        move |scope| {
            rayon_core::ScopeFifo::spawn_broadcast(scope, {
                let broadcast_records = Arc::clone(&broadcast_records);
                let nested_records = Arc::clone(&nested_records);
                let nested_jobs = Arc::clone(&nested_jobs);

                move |scope, context| {
                    let index = context.index();
                    let num_threads = context.num_threads();
                    let current_index = rayon_core::current_thread_index();

                    assert!(index < num_threads);
                    assert_eq!(num_threads, 4);
                    assert_eq!(current_index, Some(index));
                    assert_eq!(rayon_core::current_num_threads(), 4);

                    broadcast_records
                        .lock()
                        .unwrap()
                        .push((index, num_threads, current_index));

                    rayon_core::ScopeFifo::spawn_fifo(scope, {
                        let nested_records = Arc::clone(&nested_records);
                        let nested_jobs = Arc::clone(&nested_jobs);

                        move |_| {
                            nested_jobs.fetch_add(1, Ordering::SeqCst);
                            nested_records
                                .lock()
                                .unwrap()
                                .push((index, rayon_core::current_thread_index()));
                        }
                    });
                }
            });

            rayon_core::ScopeFifo::spawn_fifo(scope, {
                let nested_records = Arc::clone(&nested_records);
                let nested_jobs = Arc::clone(&nested_jobs);

                move |_| {
                    nested_jobs.fetch_add(1, Ordering::SeqCst);
                    nested_records
                        .lock()
                        .unwrap()
                        .push((usize::MAX, rayon_core::current_thread_index()));
                }
            });

            "scope-fifo-broadcast-complete"
        }
    });

    assert_eq!(returned, "scope-fifo-broadcast-complete");

    let broadcast_records = broadcast_records.lock().unwrap().clone();
    assert_eq!(broadcast_records.len(), 4);

    let broadcast_indexes: BTreeSet<usize> = broadcast_records
        .iter()
        .map(|(index, _, _)| *index)
        .collect();
    assert_eq!(broadcast_indexes, BTreeSet::from([0, 1, 2, 3]));

    for (index, num_threads, current_index) in &broadcast_records {
        assert_eq!(*num_threads, 4);
        assert_eq!(*current_index, Some(*index));
    }

    assert_eq!(nested_jobs.load(Ordering::SeqCst), 5);

    let nested_records = nested_records.lock().unwrap().clone();
    assert_eq!(nested_records.len(), 5);

    let nested_sources: BTreeSet<usize> = nested_records.iter().map(|(source, _)| *source).collect();
    assert_eq!(nested_sources, BTreeSet::from([0, 1, 2, 3, usize::MAX]));

    assert!(
        nested_records
            .iter()
            .all(|(_, current_index)| current_index.is_some()),
        "all nested FIFO jobs should execute inside the custom Rayon pool"
    );
}