use rayon_core::{
    broadcast, current_num_threads, current_thread_index, in_place_scope, in_place_scope_fifo,
    join, join_context, scope, scope_fifo, spawn_broadcast, spawn_fifo, BroadcastContext,
    FnContext,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[test]
fn join_recursive_parallel_sum() {
    fn psum(slice: &[u64]) -> u64 {
        if slice.len() <= 4 {
            return slice.iter().sum();
        }
        let mid = slice.len() / 2;
        let (left, right) = slice.split_at(mid);
        let (l, r) = join(|| psum(left), || psum(right));
        l + r
    }

    let data: Vec<u64> = (1..=100).collect();
    let expected: u64 = (1..=100).sum();
    assert_eq!(expected, 5050);

    let result = psum(&data);
    assert_eq!(result, 5050);
    assert_eq!(result, expected);
    assert_ne!(result, 0);

    let empty: [u64; 0] = [];
    assert_eq!(psum(&empty), 0);

    let small = [10u64, 20, 30];
    assert_eq!(psum(&small), 60);

    let (a, b) = join(|| 7i64 * 6, || 9i64 * 8);
    assert_eq!(a, 42);
    assert_eq!(b, 72);
    assert_eq!(a + b, 114);
    assert_ne!(a, b);


    let (s, n) = join(|| String::from("hello"), || 5usize);
    assert_eq!(s, "hello");
    assert_eq!(s.len(), n);
    assert_eq!(n, 5);
}

#[test]
fn join_context_returns_values_and_inspects_ctx() {
    let probed_a = Arc::new(AtomicBool::new(false));
    let probed_b = Arc::new(AtomicBool::new(false));
    let pa = probed_a.clone();
    let pb = probed_b.clone();

    let (ra, rb) = join_context(
        move |ctx: FnContext| {

            let _m: bool = ctx.migrated();
            pa.store(true, Ordering::SeqCst);
            42i32
        },
        move |ctx: FnContext| {
            let _m: bool = ctx.migrated();
            pb.store(true, Ordering::SeqCst);
            100i32
        },
    );

    assert_eq!(ra, 42);
    assert_eq!(rb, 100);
    assert_eq!(ra + rb, 142);
    assert_ne!(ra, rb);
    assert!(probed_a.load(Ordering::SeqCst));
    assert!(probed_b.load(Ordering::SeqCst));


    let count = AtomicUsize::new(0);
    let ((la, lb), (rc, rd)) = join_context(
        |_| join_context(|_| { count.fetch_add(1, Ordering::SeqCst); 1u32 },
                          |_| { count.fetch_add(1, Ordering::SeqCst); 2u32 }),
        |_| join_context(|_| { count.fetch_add(1, Ordering::SeqCst); 3u32 },
                          |_| { count.fetch_add(1, Ordering::SeqCst); 4u32 }),
    );
    assert_eq!(la, 1);
    assert_eq!(lb, 2);
    assert_eq!(rc, 3);
    assert_eq!(rd, 4);
    assert_eq!(la + lb + rc + rd, 10);
    assert_eq!(count.load(Ordering::SeqCst), 4);
}

#[test]
fn scope_nested_spawning_accumulates() {
    let counter = Arc::new(AtomicUsize::new(0));
    let total = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(total.load(Ordering::SeqCst), 0);

    scope(|s| {
        for i in 1..=10usize {
            let counter = counter.clone();
            let total = total.clone();
            s.spawn(move |s2| {
                counter.fetch_add(1, Ordering::SeqCst);
                let total2 = total.clone();
                s2.spawn(move |_| {
                    total2.fetch_add(i, Ordering::SeqCst);
                });
            });
        }
    });

    assert_eq!(counter.load(Ordering::SeqCst), 10);
    assert_eq!(total.load(Ordering::SeqCst), 55);
    assert_ne!(total.load(Ordering::SeqCst), counter.load(Ordering::SeqCst));


    let r: &'static str = scope(|s| {
        let c = counter.clone();
        s.spawn(move |_| {
            c.fetch_add(100, Ordering::SeqCst);
        });
        "returned"
    });
    assert_eq!(r, "returned");
    assert_eq!(r.len(), 8);
    assert_eq!(counter.load(Ordering::SeqCst), 110);


    let n: i64 = scope(|_| -1i64);
    assert_eq!(n, -1);
}

#[test]
fn scope_fifo_completes_all_tasks() {
    let executed: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    {
        let g = executed.lock().expect("uncontended pre-lock");
        assert_eq!(g.len(), 0);
        assert!(g.is_empty());
    }

    let returned: usize = scope_fifo(|s| {
        for i in 0..8usize {
            let executed = executed.clone();
            s.spawn_fifo(move |_| {
                let mut g = executed.lock().unwrap();
                g.push(i);
            });
        }
        8usize
    });

    assert_eq!(returned, 8);

    let g = executed.lock().expect("uncontended post-lock");
    assert_eq!(g.len(), 8);

    let mut sorted = g.clone();
    sorted.sort();
    let expected: Vec<usize> = (0..8).collect();
    assert_eq!(sorted, expected);

    let sum: usize = g.iter().sum();
    assert_eq!(sum, 28);

    let set: HashSet<usize> = g.iter().copied().collect();
    assert_eq!(set.len(), 8);
    assert!(set.contains(&0));
    assert!(set.contains(&7));
    assert!(!set.contains(&8));
}

#[test]
fn in_place_scope_runs_op_on_caller_thread() {
    let outer_id = std::thread::current().id();
    let on_caller = Arc::new(AtomicBool::new(false));
    let on_caller_c = on_caller.clone();
    let counter = AtomicUsize::new(0);

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert!(!on_caller.load(Ordering::SeqCst));

    let result: u64 = in_place_scope(|s| {
        if std::thread::current().id() == outer_id {
            on_caller_c.store(true, Ordering::SeqCst);
        }
        for _ in 0..6 {
            s.spawn(|_| {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
        12345u64
    });

    assert_eq!(result, 12345);
    assert!(
        on_caller.load(Ordering::SeqCst),
        "in_place_scope op closure must run on the calling thread"
    );
    assert_eq!(counter.load(Ordering::SeqCst), 6);
    assert_ne!(counter.load(Ordering::SeqCst), 0);


    let total = AtomicUsize::new(0);
    let r2: i32 = in_place_scope(|s| {
        for v in 1..=4i32 {
            let t = &total;
            s.spawn(move |_| {
                t.fetch_add(v as usize, Ordering::SeqCst);
            });
        }
        -7
    });
    assert_eq!(r2, -7);
    assert_eq!(total.load(Ordering::SeqCst), 10);


    let zero: i64 = in_place_scope(|_| 0i64);
    assert_eq!(zero, 0);

    let phrase: String = in_place_scope(|_| String::from("ok"));
    assert_eq!(phrase, "ok");
    assert_eq!(phrase.len(), 2);
}

#[test]
fn in_place_scope_fifo_runs_op_on_caller_thread() {
    let outer_id = std::thread::current().id();
    let same = Arc::new(AtomicBool::new(false));
    let same_c = same.clone();
    let acc = AtomicUsize::new(0);

    assert!(!same.load(Ordering::SeqCst));
    assert_eq!(acc.load(Ordering::SeqCst), 0);

    let r: String = in_place_scope_fifo(|s| {
        if std::thread::current().id() == outer_id {
            same_c.store(true, Ordering::SeqCst);
        }
        for i in 1..=5usize {
            let a = &acc;
            s.spawn_fifo(move |_| {
                a.fetch_add(i, Ordering::SeqCst);
            });
        }
        String::from("done")
    });

    assert_eq!(r, "done");
    assert_eq!(r.len(), 4);
    assert!(same.load(Ordering::SeqCst));
    assert_eq!(acc.load(Ordering::SeqCst), 15);
    assert_ne!(acc.load(Ordering::SeqCst), 0);


    let count = AtomicUsize::new(0);
    in_place_scope_fifo(|s| {
        for _ in 0..3 {
            s.spawn_fifo(|_| {
                count.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(count.load(Ordering::SeqCst), 3);

    let n: i32 = in_place_scope_fifo(|_| 99);
    assert_eq!(n, 99);
    assert_ne!(n, 0);
}

#[test]
fn broadcast_yields_one_value_per_worker() {
    let n = current_num_threads();
    assert!(n >= 1, "expected at least one worker thread");

    let results: Vec<usize> = broadcast(|ctx: BroadcastContext<'_>| ctx.index());
    assert_eq!(results.len(), n);

    let mut sorted = results.clone();
    sorted.sort();
    let expected: Vec<usize> = (0..n).collect();
    assert_eq!(sorted, expected);

    let set: HashSet<usize> = results.iter().copied().collect();
    assert_eq!(set.len(), n);
    assert!(set.contains(&0));


    let matches = Arc::new(AtomicUsize::new(0));
    let mc = matches.clone();
    let confirmations: Vec<bool> = broadcast(move |ctx: BroadcastContext<'_>| {
        let idx = ctx.index();
        let cti = current_thread_index();
        if cti == Some(idx) {
            mc.fetch_add(1, Ordering::SeqCst);
            true
        } else {
            false
        }
    });

    assert_eq!(confirmations.len(), n);
    assert_eq!(matches.load(Ordering::SeqCst), n);
    for b in &confirmations {
        assert!(*b, "current_thread_index must match BroadcastContext::index");
    }


    let squares: Vec<usize> = broadcast(|ctx| ctx.index() * ctx.index());
    assert_eq!(squares.len(), n);
    let mut sq_sorted = squares.clone();
    sq_sorted.sort();
    let expected_sq: Vec<usize> = (0..n).map(|i| i * i).collect();
    assert_eq!(sq_sorted, expected_sq);
}

#[test]
fn spawn_broadcast_runs_on_each_worker() {
    let n = current_num_threads();
    assert!(n >= 1);

    let count = Arc::new(AtomicUsize::new(0));
    let cc = count.clone();
    assert_eq!(count.load(Ordering::SeqCst), 0);

    spawn_broadcast(move |_ctx: BroadcastContext<'_>| {
        cc.fetch_add(1, Ordering::SeqCst);
    });

    let start = Instant::now();
    while count.load(Ordering::SeqCst) < n {
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(count.load(Ordering::SeqCst), n);


    let count2 = Arc::new(AtomicUsize::new(0));
    let c2 = count2.clone();
    spawn_broadcast(move |_| {
        c2.fetch_add(2, Ordering::SeqCst);
    });
    let c3 = count2.clone();
    spawn_broadcast(move |_| {
        c3.fetch_add(3, Ordering::SeqCst);
    });

    let target = 5 * n;
    let start = Instant::now();
    while count2.load(Ordering::SeqCst) < target {
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
        std::thread::yield_now();
    }

    assert_eq!(count2.load(Ordering::SeqCst), target);
    assert!(count2.load(Ordering::SeqCst) >= n);
    assert!(count2.load(Ordering::SeqCst) >= 2 * n);
    assert_ne!(count2.load(Ordering::SeqCst), 0);


    let seen: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let seen_c = seen.clone();
    spawn_broadcast(move |ctx: BroadcastContext<'_>| {
        seen_c.lock().unwrap().push(ctx.index());
    });

    let start = Instant::now();
    loop {
        let len = seen.lock().unwrap().len();
        if len >= n {
            break;
        }
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
        std::thread::yield_now();
    }
    let mut indices = seen.lock().unwrap().clone();
    indices.sort();
    let expected: Vec<usize> = (0..n).collect();
    assert_eq!(indices, expected);
    assert_eq!(indices.len(), n);
}

#[test]
fn spawn_fifo_completes_all_submitted_jobs() {
    let count = Arc::new(AtomicUsize::new(0));
    let total = Arc::new(AtomicUsize::new(0));

    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(total.load(Ordering::SeqCst), 0);

    for i in 1..=10usize {
        let count = count.clone();
        let total = total.clone();
        spawn_fifo(move || {
            count.fetch_add(1, Ordering::SeqCst);
            total.fetch_add(i, Ordering::SeqCst);
        });
    }

    let start = Instant::now();
    while count.load(Ordering::SeqCst) < 10 {
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
        std::thread::yield_now();
    }

    assert_eq!(count.load(Ordering::SeqCst), 10);
    assert_eq!(total.load(Ordering::SeqCst), 55);
    assert_ne!(count.load(Ordering::SeqCst), 0);
    assert_ne!(total.load(Ordering::SeqCst), 0);
    assert!(count.load(Ordering::SeqCst) >= 10);
    assert!(total.load(Ordering::SeqCst) > count.load(Ordering::SeqCst));


    for _ in 0..5 {
        let c = count.clone();
        spawn_fifo(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }
    let start = Instant::now();
    while count.load(Ordering::SeqCst) < 15 {
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(count.load(Ordering::SeqCst), 15);
    assert_eq!(total.load(Ordering::SeqCst), 55);
}