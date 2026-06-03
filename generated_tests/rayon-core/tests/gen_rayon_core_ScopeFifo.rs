use rayon_core::{scope_fifo, in_place_scope_fifo, ThreadPoolBuilder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[test]
fn scope_fifo_spawn_fifo_ordering_and_results() {
    let pool = ThreadPoolBuilder::new().num_threads(1).build().unwrap();

    let order: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    let counter = AtomicUsize::new(0);
    let sum = AtomicUsize::new(0);

    let pre_count = counter.load(Ordering::SeqCst);
    let pre_sum = sum.load(Ordering::SeqCst);
    assert_eq!(pre_count, 0);
    assert_eq!(pre_sum, 0);
    assert_eq!(order.lock().unwrap().len(), 0);

    pool.install(|| {
        scope_fifo(|s| {
            for i in 0..8usize {
                let order_ref = &order;
                let counter_ref = &counter;
                let sum_ref = &sum;
                s.spawn_fifo(move |_inner| {
                    counter_ref.fetch_add(1, Ordering::SeqCst);
                    sum_ref.fetch_add(i, Ordering::SeqCst);
                    order_ref.lock().unwrap().push(i);
                });
            }
        });
    });

    let post_count = counter.load(Ordering::SeqCst);
    let post_sum = sum.load(Ordering::SeqCst);
    assert_eq!(post_count, 8);
    assert_eq!(post_sum, 0 + 1 + 2 + 3 + 4 + 5 + 6 + 7);

    let recorded = order.into_inner().unwrap();
    assert_eq!(recorded.len(), 8);

    assert_eq!(recorded, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    assert_ne!(recorded, vec![7, 6, 5, 4, 3, 2, 1, 0]);
}

#[test]
fn scope_fifo_nested_spawn_fifo_completes_all() {
    let pool = ThreadPoolBuilder::new().num_threads(2).build().unwrap();

    let outer_done = AtomicUsize::new(0);
    let inner_done = AtomicUsize::new(0);

    assert_eq!(outer_done.load(Ordering::SeqCst), 0);
    assert_eq!(inner_done.load(Ordering::SeqCst), 0);

    pool.install(|| {
        scope_fifo(|s| {
            for _ in 0..4usize {
                let outer_ref = &outer_done;
                let inner_ref = &inner_done;
                s.spawn_fifo(move |s2| {
                    outer_ref.fetch_add(1, Ordering::SeqCst);
                    for _ in 0..3usize {
                        let inner_ref2 = inner_ref;
                        s2.spawn_fifo(move |_| {
                            inner_ref2.fetch_add(1, Ordering::SeqCst);
                        });
                    }
                });
            }
        });
    });

    let final_outer = outer_done.load(Ordering::SeqCst);
    let final_inner = inner_done.load(Ordering::SeqCst);
    assert_eq!(final_outer, 4);
    assert_eq!(final_inner, 12);
    assert_ne!(final_outer, 0);
    assert_ne!(final_inner, 0);
    assert!(final_inner > final_outer);
}

#[test]
fn in_place_scope_fifo_spawn_fifo_accumulates() {
    let values: Vec<usize> = (1..=10).collect();
    let total = AtomicUsize::new(0);
    let count = AtomicUsize::new(0);

    assert_eq!(total.load(Ordering::SeqCst), 0);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(values.len(), 10);

    in_place_scope_fifo(|s| {
        for v in &values {
            let total_ref = &total;
            let count_ref = &count;
            s.spawn_fifo(move |_| {
                total_ref.fetch_add(*v, Ordering::SeqCst);
                count_ref.fetch_add(1, Ordering::SeqCst);
            });
        }
    });

    let final_total = total.load(Ordering::SeqCst);
    let final_count = count.load(Ordering::SeqCst);
    assert_eq!(final_total, 55);
    assert_eq!(final_count, 10);
    assert_ne!(final_total, 0);
    assert_ne!(final_count, 0);
}

#[test]
fn scope_fifo_returns_value_after_spawns_complete() {
    let pool = ThreadPoolBuilder::new().num_threads(2).build().unwrap();
    let n = AtomicUsize::new(0);

    assert_eq!(n.load(Ordering::SeqCst), 0);

    let result: usize = pool.install(|| {
        scope_fifo(|s| {
            for _ in 0..5usize {
                let nr = &n;
                s.spawn_fifo(move |_| {
                    nr.fetch_add(2, Ordering::SeqCst);
                });
            }
            42usize
        })
    });

    let final_n = n.load(Ordering::SeqCst);
    assert_eq!(result, 42);
    assert_eq!(final_n, 10);
    assert_ne!(final_n, 0);
    assert_ne!(result, 0);
    assert!(result > final_n);
}