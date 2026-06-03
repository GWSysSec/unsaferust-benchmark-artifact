use rayon_core::{
    broadcast, current_thread_index, in_place_scope, in_place_scope_fifo, join, join_context,
    scope, scope_fifo, spawn_broadcast, spawn_fifo, BroadcastContext, FnContext,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[test]
fn join_recursive_parallel_sum_is_exact() {
    fn sum_range(slice: &[u64]) -> u64 {
        if slice.len() <= 1 {
            return slice.iter().copied().sum();
        }
        let mid = slice.len() / 2;
        let (l, r) = slice.split_at(mid);
        let (a, b) = join(|| sum_range(l), || sum_range(r));
        a + b
    }

    let data: Vec<u64> = (1..=1000).collect();
    assert_eq!(data.len(), 1000);
    assert_eq!(data[0], 1);
    assert_eq!(*data.last().unwrap(), 1000);

    let result = sum_range(&data);
    let expected: u64 = (1..=1000u64).sum();
    assert_eq!(result, 500_500);
    assert_eq!(result, expected);
    assert_ne!(result, 0);

    let empty: Vec<u64> = vec![];
    assert_eq!(sum_range(&empty), 0);
    assert_eq!(sum_range(&[42]), 42);
    assert_eq!(sum_range(&[1, 2, 3, 4, 5]), 15);


    let (s, n) = join(|| String::from("rayon"), || 7u32);
    assert_eq!(s, "rayon");
    assert_eq!(s.len(), 5);
    assert_eq!(n, 7u32);
}

#[test]
fn join_context_runs_both_closures_and_exposes_migrated() {
    let a_ran = Arc::new(AtomicBool::new(false));
    let b_ran = Arc::new(AtomicBool::new(false));
    let a_mig = Arc::new(AtomicUsize::new(2));
    let b_mig = Arc::new(AtomicUsize::new(2));

    assert!(!a_ran.load(Ordering::SeqCst));
    assert!(!b_ran.load(Ordering::SeqCst));
    assert_eq!(a_mig.load(Ordering::SeqCst), 2);
    assert_eq!(b_mig.load(Ordering::SeqCst), 2);

    let ar = Arc::clone(&a_ran);
    let br = Arc::clone(&b_ran);
    let am = Arc::clone(&a_mig);
    let bm = Arc::clone(&b_mig);

    let (ra, rb): (i64, i64) = join_context(
        move |ctx: FnContext| {
            ar.store(true, Ordering::SeqCst);
            am.store(if ctx.migrated() { 1 } else { 0 }, Ordering::SeqCst);
            100
        },
        move |ctx: FnContext| {
            br.store(true, Ordering::SeqCst);
            bm.store(if ctx.migrated() { 1 } else { 0 }, Ordering::SeqCst);
            200
        },
    );

    assert_eq!(ra, 100);
    assert_eq!(rb, 200);
    assert_eq!(ra + rb, 300);
    assert!(a_ran.load(Ordering::SeqCst));
    assert!(b_ran.load(Ordering::SeqCst));


    let amv = a_mig.load(Ordering::SeqCst);
    let bmv = b_mig.load(Ordering::SeqCst);
    assert!(amv == 0 || amv == 1, "a migrated value out of range: {}", amv);
    assert!(bmv == 0 || bmv == 1, "b migrated value out of range: {}", bmv);
}

#[test]
fn scope_completes_all_spawned_tasks_before_return() {
    let counter = Arc::new(AtomicUsize::new(0));
    const N: usize = 50;

    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let c = Arc::clone(&counter);
    let returned: usize = scope(|s| {
        for i in 0..N {
            let c = Arc::clone(&c);
            s.spawn(move |_inner| {
                c.fetch_add(1, Ordering::SeqCst);
                let _ = i;
            });
        }
        N * 2
    });

    let final_val = counter.load(Ordering::SeqCst);
    assert_eq!(final_val, N);
    assert_eq!(returned, 100);
    assert_ne!(final_val, 0);
    assert!(final_val >= N);
    assert!(final_val <= N);
    assert_eq!(returned / 2, final_val);
    assert_eq!(returned - final_val, N);
}

#[test]
fn scope_nested_inner_spawns_all_complete() {
    let total = Arc::new(AtomicUsize::new(0));
    assert_eq!(total.load(Ordering::SeqCst), 0);

    let t = Arc::clone(&total);
    scope(|s| {
        for outer in 0..5usize {
            let t = Arc::clone(&t);
            s.spawn(move |inner_scope| {
                t.fetch_add(1, Ordering::SeqCst);
                for _ in 0..3 {
                    let t = Arc::clone(&t);
                    inner_scope.spawn(move |_| {
                        t.fetch_add(10, Ordering::SeqCst);
                    });
                }
                let _ = outer;
            });
        }
    });


    let final_val = total.load(Ordering::SeqCst);
    assert_eq!(final_val, 155);
    assert_ne!(final_val, 0);
    assert!(final_val > 100);
    assert!(final_val < 200);
    assert_eq!(final_val % 5, 0);
    assert_eq!(final_val - 5, 150);
    assert_eq!(150 / 10, 15);
}

#[test]
fn scope_returns_value_of_op() {
    let s_out: String = scope(|_| String::from("scoped"));
    assert_eq!(s_out, "scoped");
    assert_eq!(s_out.len(), 6);
    assert_ne!(s_out, "");

    let v: Vec<i32> = scope(|_| vec![1, 2, 3]);
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], 1);
    assert_eq!(v[2], 3);
    assert_eq!(v.iter().sum::<i32>(), 6);

    let n: i64 = scope(|_| -42i64);
    assert_eq!(n, -42);
    assert!(n < 0);
    assert_ne!(n, 0);
}

#[test]
fn scope_fifo_runs_every_task_exactly_once() {
    let counter = Arc::new(AtomicUsize::new(0));
    let visited = Arc::new(Mutex::new(Vec::<usize>::new()));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(visited.lock().unwrap().len(), 0);

    let c = Arc::clone(&counter);
    let v = Arc::clone(&visited);

    scope_fifo(|s| {
        for i in 0..20usize {
            let c = Arc::clone(&c);
            let v = Arc::clone(&v);
            s.spawn_fifo(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                let start = Instant::now();
                loop {
                    if let Ok(mut g) = v.try_lock() {
                        g.push(i);
                        break;
                    }
                    if start.elapsed() > Duration::from_secs(5) {
                        panic!("scope_fifo lock timeout");
                    }
                    std::thread::yield_now();
                }
            });
        }
    });

    assert_eq!(counter.load(Ordering::SeqCst), 20);
    let final_visited = visited.lock().expect("lock available after scope");
    assert_eq!(final_visited.len(), 20);

    let set: HashSet<usize> = final_visited.iter().copied().collect();
    assert_eq!(set.len(), 20, "every index must appear exactly once");
    assert!(set.contains(&0));
    assert!(set.contains(&19));
    assert!(!set.contains(&20));
    assert_eq!(*set.iter().min().unwrap(), 0);
    assert_eq!(*set.iter().max().unwrap(), 19);
}

#[test]
fn in_place_scope_returns_value_and_finishes_work() {
    let caller_idx_before = current_thread_index();
    let counter = Arc::new(AtomicUsize::new(0));
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let c = Arc::clone(&counter);
    let result: usize = in_place_scope(|s| {
        for _ in 0..15 {
            let c = Arc::clone(&c);
            s.spawn(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        999
    });

    let caller_idx_after = current_thread_index();
    assert_eq!(result, 999);
    assert_eq!(counter.load(Ordering::SeqCst), 15);
    assert_ne!(counter.load(Ordering::SeqCst), 0);
    assert!(result > 0);
    assert_eq!(caller_idx_before, caller_idx_after);
    assert_eq!(result - counter.load(Ordering::SeqCst), 984);
    assert!(result >= 15);
}

#[test]
fn in_place_scope_fifo_completes_all_tasks_with_correct_sum() {
    let counter = Arc::new(AtomicUsize::new(0));
    let sum = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(sum.load(Ordering::SeqCst), 0);

    let c = Arc::clone(&counter);
    let s = Arc::clone(&sum);

    let returned: &'static str = in_place_scope_fifo(|sc| {
        for i in 1..=10usize {
            let c = Arc::clone(&c);
            let s = Arc::clone(&s);
            sc.spawn_fifo(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                s.fetch_add(i, Ordering::SeqCst);
            });
        }
        "done"
    });

    assert_eq!(returned, "done");
    assert_eq!(returned.len(), 4);
    assert_eq!(counter.load(Ordering::SeqCst), 10);
    assert_eq!(sum.load(Ordering::SeqCst), 55);
    assert_ne!(sum.load(Ordering::SeqCst), 0);
    assert!(counter.load(Ordering::SeqCst) >= 10);
    assert!(sum.load(Ordering::SeqCst) > counter.load(Ordering::SeqCst));
    assert_ne!(returned, "");
}

#[test]
fn broadcast_runs_on_every_worker_with_unique_indices() {
    let results = broadcast(|ctx: BroadcastContext<'_>| ctx.index());

    assert!(!results.is_empty());
    let n = results.len();
    assert!(n >= 1);

    let set: HashSet<usize> = results.iter().copied().collect();
    assert_eq!(set.len(), n, "indices must be unique across workers");

    let mut sorted = results.clone();
    sorted.sort();
    let expected: Vec<usize> = (0..n).collect();
    assert_eq!(sorted, expected, "broadcast indices cover 0..n");

    assert_eq!(*results.iter().min().unwrap(), 0);
    assert_eq!(*results.iter().max().unwrap(), n - 1);


    let strings: Vec<String> = broadcast(|ctx| format!("w-{}", ctx.index()));
    assert_eq!(strings.len(), n);
    let unique_strings: HashSet<String> = strings.iter().cloned().collect();
    assert_eq!(unique_strings.len(), n);
    for s in &strings {
        assert!(s.starts_with("w-"), "unexpected prefix: {}", s);
        assert!(s.len() >= 3);
    }
}

#[test]
fn spawn_broadcast_runs_on_every_worker() {

    let probe = broadcast(|_| 1usize);
    let n = probe.len();
    assert!(n >= 1);
    assert_eq!(probe.iter().sum::<usize>(), n);
    assert!(probe.iter().all(|&x| x == 1));

    let counter = Arc::new(AtomicUsize::new(0));
    let indices = Arc::new(Mutex::new(Vec::<usize>::new()));
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(indices.lock().unwrap().len(), 0);

    let c = Arc::clone(&counter);
    let ix = Arc::clone(&indices);
    spawn_broadcast(move |ctx: BroadcastContext<'_>| {
        c.fetch_add(1, Ordering::SeqCst);
        let start = Instant::now();
        loop {
            if let Ok(mut g) = ix.try_lock() {
                g.push(ctx.index());
                break;
            }
            if start.elapsed() > Duration::from_secs(5) {
                break;
            }
            std::thread::yield_now();
        }
    });

    let start = Instant::now();
    while counter.load(Ordering::SeqCst) < n {
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
        std::thread::yield_now();
    }

    let final_count = counter.load(Ordering::SeqCst);
    assert_eq!(final_count, n);
    assert_ne!(final_count, 0);

    let observed = indices.lock().expect("indices mutex not poisoned");
    assert_eq!(observed.len(), n);
    let set: HashSet<usize> = observed.iter().copied().collect();
    assert_eq!(set.len(), n);
    assert_eq!(*set.iter().min().unwrap(), 0);
    assert_eq!(*set.iter().max().unwrap(), n - 1);
}

#[test]
fn spawn_fifo_executes_all_submitted_tasks() {
    let counter = Arc::new(AtomicUsize::new(0));
    let total_sum = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(total_sum.load(Ordering::SeqCst), 0);

    const N: usize = 20;
    for i in 1..=N {
        let c = Arc::clone(&counter);
        let s = Arc::clone(&total_sum);
        spawn_fifo(move || {
            c.fetch_add(1, Ordering::SeqCst);
            s.fetch_add(i, Ordering::SeqCst);
        });
    }

    let start = Instant::now();
    while counter.load(Ordering::SeqCst) < N {
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
        std::thread::yield_now();
    }

    let c = counter.load(Ordering::SeqCst);
    let s = total_sum.load(Ordering::SeqCst);

    assert_eq!(c, N);
    assert_eq!(s, 210);
    assert_eq!(s, (1..=N).sum::<usize>());
    assert!(c > 0);
    assert!(s > 0);
    assert_ne!(c, 0);
    assert_ne!(s, 0);
    assert!(s > c);
}