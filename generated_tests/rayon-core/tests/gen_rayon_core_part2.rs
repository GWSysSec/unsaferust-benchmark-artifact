use rayon_core::{
    broadcast, current_num_threads, current_thread_index, in_place_scope,
    in_place_scope_fifo, join, join_context, scope, scope_fifo, spawn_broadcast,
    spawn_fifo, BroadcastContext, FnContext,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn poll_until<F: Fn() -> bool>(cond: F, timeout_ms: u64) -> bool {
    let start = Instant::now();
    while !cond() {
        if start.elapsed() > Duration::from_millis(timeout_ms) {
            return false;
        }
        std::thread::yield_now();
    }
    true
}

#[test]
fn test_join_parallel_sum_split() {
    let data: Vec<u64> = (1..=1000u64).collect();
    let expected_total: u64 = 500_500;
    let expected_left: u64 = (1..=500u64).sum();
    let expected_right: u64 = (501..=1000u64).sum();

    assert_eq!(data.len(), 1000);
    assert_eq!(data[0], 1);
    assert_eq!(data[999], 1000);
    assert_eq!(expected_left + expected_right, expected_total);
    assert_ne!(expected_left, expected_right);

    let (left, right) = join(
        || data[..500].iter().sum::<u64>(),
        || data[500..].iter().sum::<u64>(),
    );

    assert_eq!(left, expected_left);
    assert_eq!(right, expected_right);
    assert_eq!(left + right, expected_total);
    assert_ne!(left, right);
    assert!(left < right);


    let ((q1, q2), (q3, q4)) = join(
        || join(
            || data[..250].iter().sum::<u64>(),
            || data[250..500].iter().sum::<u64>(),
        ),
        || join(
            || data[500..750].iter().sum::<u64>(),
            || data[750..].iter().sum::<u64>(),
        ),
    );
    assert_eq!(q1 + q2, expected_left);
    assert_eq!(q3 + q4, expected_right);
    assert_eq!(q1 + q2 + q3 + q4, expected_total);
    assert!(q1 < q2);
    assert!(q3 < q4);
}

#[test]
fn test_join_context_migration_consistency() {
    let runs = 32usize;
    let a_log: Arc<Mutex<Vec<(i32, bool)>>> = Arc::new(Mutex::new(Vec::new()));
    let b_log: Arc<Mutex<Vec<(i32, bool)>>> = Arc::new(Mutex::new(Vec::new()));

    assert_eq!(a_log.lock().unwrap().len(), 0);
    assert_eq!(b_log.lock().unwrap().len(), 0);

    for i in 0..runs {
        let al = Arc::clone(&a_log);
        let bl = Arc::clone(&b_log);
        let v = i as i32;
        let (ra, rb) = join_context(
            move |ctx: FnContext| {
                let m1 = ctx.migrated();
                let m2 = ctx.migrated();
                assert_eq!(m1, m2);
                al.lock().unwrap().push((v, m1));
                v * 2
            },
            move |ctx: FnContext| {
                let m1 = ctx.migrated();
                let m2 = ctx.migrated();
                assert_eq!(m1, m2);
                bl.lock().unwrap().push((v + 100, m1));
                v * 3
            },
        );
        assert_eq!(ra, v * 2);
        assert_eq!(rb, v * 3);
    }

    let a_entries = a_log.lock().unwrap().clone();
    let b_entries = b_log.lock().unwrap().clone();
    assert_eq!(a_entries.len(), runs);
    assert_eq!(b_entries.len(), runs);

    for (i, &(val, _)) in a_entries.iter().enumerate() {
        assert_eq!(val, i as i32);
    }
    for (i, &(val, _)) in b_entries.iter().enumerate() {
        assert_eq!(val, i as i32 + 100);
    }

    let a_sum: i32 = a_entries.iter().map(|(v, _)| v).sum();
    let expected_a_sum: i32 = (0..runs as i32).sum();
    assert_eq!(a_sum, expected_a_sum);
    assert_ne!(a_sum, 0);
}

#[test]
fn test_scope_nested_and_returns_value() {
    let outer = AtomicUsize::new(0);
    let inner = AtomicUsize::new(0);

    assert_eq!(outer.load(Ordering::SeqCst), 0);
    assert_eq!(inner.load(Ordering::SeqCst), 0);

    let scope_result: i64 = scope(|s| {
        for _ in 0..4 {
            s.spawn(|s2| {
                outer.fetch_add(1, Ordering::SeqCst);
                for _ in 0..3 {
                    s2.spawn(|_| {
                        inner.fetch_add(1, Ordering::SeqCst);
                    });
                }
            });
        }
        -7i64
    });

    assert_eq!(scope_result, -7);
    assert_eq!(outer.load(Ordering::SeqCst), 4);
    assert_eq!(inner.load(Ordering::SeqCst), 12);
    assert_ne!(outer.load(Ordering::SeqCst), inner.load(Ordering::SeqCst));
    assert!(inner.load(Ordering::SeqCst) > outer.load(Ordering::SeqCst));
    let total = outer.load(Ordering::SeqCst) + inner.load(Ordering::SeqCst);
    assert_eq!(total, 16);
    assert!(total >= 16 && total < 1000);
}

#[test]
fn test_scope_fifo_completes_all_with_return() {
    let counter = AtomicUsize::new(0);
    let counter_ref = &counter;
    let n_tasks = 20usize;

    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let returned: usize = scope_fifo(|s| {
        for i in 0..n_tasks {
            s.spawn_fifo(move |_| {
                counter_ref.fetch_add(i + 1, Ordering::SeqCst);
            });
        }
        9999usize
    });

    let expected: usize = (1..=n_tasks).sum();
    assert_eq!(returned, 9999);
    assert_eq!(expected, 210);
    assert_eq!(counter.load(Ordering::SeqCst), expected);
    assert_ne!(counter.load(Ordering::SeqCst), 0);
    assert!(counter.load(Ordering::SeqCst) > 100);

    let nested = AtomicUsize::new(0);
    let nested_ref = &nested;
    scope_fifo(|s| {
        s.spawn_fifo(move |s2| {
            nested_ref.fetch_add(1, Ordering::SeqCst);
            s2.spawn_fifo(move |_| {
                nested_ref.fetch_add(10, Ordering::SeqCst);
            });
        });
    });
    assert_eq!(nested.load(Ordering::SeqCst), 11);
    assert_ne!(nested.load(Ordering::SeqCst), 0);
    assert_ne!(nested.load(Ordering::SeqCst), counter.load(Ordering::SeqCst));
}

#[test]
fn test_in_place_scope_runs_op_on_caller() {
    let caller_thread = std::thread::current().id();
    let workers = AtomicUsize::new(0);
    let workers_ref = &workers;
    let op_ran_here = AtomicBool::new(false);

    assert_eq!(workers.load(Ordering::SeqCst), 0);
    assert!(!op_ran_here.load(Ordering::SeqCst));

    let result: i32 = in_place_scope(|s| {
        let here = std::thread::current().id();
        if here == caller_thread {
            op_ran_here.store(true, Ordering::SeqCst);
        }
        for i in 0..5u32 {
            s.spawn(move |_| {
                workers_ref.fetch_add((i + 1) as usize, Ordering::SeqCst);
            });
        }
        42
    });

    assert_eq!(result, 42);
    assert!(op_ran_here.load(Ordering::SeqCst), "OP must run on caller thread");
    let expected_sum: usize = 1 + 2 + 3 + 4 + 5;
    assert_eq!(expected_sum, 15);
    assert_eq!(workers.load(Ordering::SeqCst), expected_sum);
    assert_ne!(workers.load(Ordering::SeqCst), 0);
    assert!(workers.load(Ordering::SeqCst) > 10);
    assert_ne!(result, 0);
}

#[test]
fn test_in_place_scope_fifo_returns_string() {
    let counter = AtomicUsize::new(0);
    let counter_ref = &counter;
    let started = AtomicBool::new(false);

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert!(!started.load(Ordering::SeqCst));

    let result: String = in_place_scope_fifo(|s| {
        started.store(true, Ordering::SeqCst);
        for i in 1..=8u32 {
            s.spawn_fifo(move |_| {
                counter_ref.fetch_add(i as usize, Ordering::SeqCst);
            });
        }
        String::from("done")
    });

    let expected: usize = (1..=8usize).sum();
    assert_eq!(result, "done");
    assert_eq!(result.len(), 4);
    assert!(started.load(Ordering::SeqCst));
    assert_eq!(expected, 36);
    assert_eq!(counter.load(Ordering::SeqCst), expected);
    assert_ne!(counter.load(Ordering::SeqCst), 0);
    assert!(counter.load(Ordering::SeqCst) >= 36);
    assert_ne!(result.as_str(), "");
}

#[test]
fn test_broadcast_indices_and_thread_consistency() {
    let n = current_num_threads();
    assert!(n >= 1, "expected at least one worker, got {}", n);

    let results: Vec<usize> = broadcast(|ctx: BroadcastContext<'_>| ctx.index());

    assert_eq!(results.len(), n);
    let unique: HashSet<usize> = results.iter().copied().collect();
    assert_eq!(unique.len(), n);
    for &idx in &results {
        assert!(idx < n, "index {} out of range for {} threads", idx, n);
    }

    let sum: usize = results.iter().sum();
    let expected_sum = n * (n - 1) / 2;
    assert_eq!(sum, expected_sum);

    let mismatches = Arc::new(AtomicUsize::new(0));
    let m = Arc::clone(&mismatches);
    let confirms: Vec<bool> = broadcast(move |ctx: BroadcastContext<'_>| {
        let bi = ctx.index();
        let ti = current_thread_index();
        let ok = ti == Some(bi);
        if !ok {
            m.fetch_add(1, Ordering::SeqCst);
        }
        ok
    });
    assert_eq!(confirms.len(), n);
    assert_eq!(mismatches.load(Ordering::SeqCst), 0);
    for &c in &confirms {
        assert!(c);
    }
    assert_eq!(confirms.iter().filter(|&&c| c).count(), n);
}

#[test]
fn test_spawn_broadcast_reaches_every_worker() {
    let n = current_num_threads();
    assert!(n >= 1);
    assert!(n <= 1024);

    let count = Arc::new(AtomicUsize::new(0));
    let seen: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(seen.lock().unwrap().len(), 0);

    let c2 = Arc::clone(&count);
    let s2 = Arc::clone(&seen);
    spawn_broadcast(move |ctx: BroadcastContext<'_>| {
        s2.lock().unwrap().push(ctx.index());
        c2.fetch_add(1, Ordering::SeqCst);
    });

    let ok = poll_until(|| count.load(Ordering::SeqCst) >= n, 5000);
    assert!(ok, "spawn_broadcast did not reach all workers in time");
    assert_eq!(count.load(Ordering::SeqCst), n);

    let mut indices = seen.lock().unwrap().clone();
    indices.sort();
    let expected: Vec<usize> = (0..n).collect();
    assert_eq!(indices, expected);
    assert_eq!(indices.len(), n);
    assert_ne!(count.load(Ordering::SeqCst), 0);

    let unique: HashSet<usize> = indices.iter().copied().collect();
    assert_eq!(unique.len(), n);
}

#[test]
fn test_spawn_fifo_many_tasks_complete() {
    let total = Arc::new(AtomicUsize::new(0));
    let done_count = Arc::new(AtomicUsize::new(0));
    let n_tasks = 50usize;

    assert_eq!(total.load(Ordering::SeqCst), 0);
    assert_eq!(done_count.load(Ordering::SeqCst), 0);

    for i in 1..=n_tasks {
        let t = Arc::clone(&total);
        let d = Arc::clone(&done_count);
        spawn_fifo(move || {
            t.fetch_add(i, Ordering::SeqCst);
            d.fetch_add(1, Ordering::SeqCst);
        });
    }

    let ok = poll_until(|| done_count.load(Ordering::SeqCst) >= n_tasks, 10_000);
    assert!(ok, "spawn_fifo tasks did not all finish in time");
    assert_eq!(done_count.load(Ordering::SeqCst), n_tasks);

    let expected_sum: usize = (1..=n_tasks).sum();
    assert_eq!(expected_sum, 1275);
    assert_eq!(total.load(Ordering::SeqCst), expected_sum);
    assert_ne!(total.load(Ordering::SeqCst), 0);
    assert!(total.load(Ordering::SeqCst) > 1000);
    assert!(done_count.load(Ordering::SeqCst) >= 50);
}

#[test]
fn test_join_with_scope_combination() {
    let left_count = AtomicUsize::new(0);
    let right_count = AtomicUsize::new(0);
    let left_ref = &left_count;
    let right_ref = &right_count;

    assert_eq!(left_count.load(Ordering::SeqCst), 0);
    assert_eq!(right_count.load(Ordering::SeqCst), 0);

    let (la, rb) = join(
        || {
            scope(|s| {
                for _ in 0..5 {
                    s.spawn(move |_| {
                        left_ref.fetch_add(2, Ordering::SeqCst);
                    });
                }
            });
            left_ref.load(Ordering::SeqCst)
        },
        || {
            scope_fifo(|s| {
                for _ in 0..3 {
                    s.spawn_fifo(move |_| {
                        right_ref.fetch_add(7, Ordering::SeqCst);
                    });
                }
            });
            right_ref.load(Ordering::SeqCst)
        },
    );

    assert_eq!(la, 10);
    assert_eq!(rb, 21);
    assert_eq!(left_count.load(Ordering::SeqCst), 10);
    assert_eq!(right_count.load(Ordering::SeqCst), 21);
    assert_ne!(la, rb);
    assert_eq!(la + rb, 31);
    assert!(la < rb);
    assert_ne!(la, 0);
    assert_ne!(rb, 0);
}