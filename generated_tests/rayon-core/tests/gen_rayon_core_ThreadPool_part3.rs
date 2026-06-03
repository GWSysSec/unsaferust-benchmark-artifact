use rayon_core::{BroadcastContext, ThreadPoolBuilder};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn build_pool(n: usize) -> rayon_core::ThreadPool {
    ThreadPoolBuilder::new()
        .num_threads(n)
        .build()
        .expect("pool build must succeed")
}

#[test]
fn threadpool_install_returns_value_and_runs_on_pool() {
    let pool = build_pool(3);


    let counter = Arc::new(AtomicUsize::new(0));
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let c = Arc::clone(&counter);
    let result: i64 = pool.install(move || {

        let idx = rayon_core::current_thread_index();
        assert!(idx.is_some(), "install body must run on pool worker");
        let i = idx.unwrap();
        assert!(i < 3, "thread index {} must be < 3", i);


        let n = rayon_core::current_num_threads();
        assert_eq!(n, 3, "current_num_threads inside install must be 3");

        c.fetch_add(7, Ordering::SeqCst);
        42
    });

    assert_eq!(result, 42, "install must return the closure's value");
    assert_eq!(counter.load(Ordering::SeqCst), 7);


    let outside_result: String = pool.install(|| String::from("hello"));
    assert_eq!(outside_result, "hello");
    assert_eq!(outside_result.len(), 5);
}

#[test]
fn threadpool_broadcast_visits_every_worker_exactly_once() {
    let pool = build_pool(4);

    let visited = Arc::new(Mutex::new(Vec::<usize>::new()));
    let v = Arc::clone(&visited);


    assert_eq!(v.lock().unwrap().len(), 0);

    let results: Vec<usize> = pool.broadcast(move |ctx: BroadcastContext<'_>| {
        let idx = ctx.index();

        let start = Instant::now();
        loop {
            if let Ok(mut g) = v.try_lock() {
                g.push(idx);
                break;
            }
            if start.elapsed() > Duration::from_secs(5) {
                panic!("could not acquire visited lock");
            }
            std::thread::yield_now();
        }
        idx * 10
    });

    assert_eq!(results.len(), 4, "one result per worker");
    let mut sorted = results.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 10, 20, 30]);

    let mut visited_idx = visited.lock().unwrap().clone();
    visited_idx.sort();
    assert_eq!(visited_idx, vec![0, 1, 2, 3]);
    assert_ne!(results, vec![0, 0, 0, 0]);
    assert_eq!(visited_idx.len(), 4);
}

#[test]
fn threadpool_join_runs_both_and_returns_pair() {
    let pool = build_pool(2);

    let a_done = Arc::new(AtomicBool::new(false));
    let b_done = Arc::new(AtomicBool::new(false));
    let a1 = Arc::clone(&a_done);
    let b1 = Arc::clone(&b_done);

    assert!(!a_done.load(Ordering::SeqCst));
    assert!(!b_done.load(Ordering::SeqCst));

    let (ra, rb) = pool.join(
        move || {
            a1.store(true, Ordering::SeqCst);
            100i32
        },
        move || {
            b1.store(true, Ordering::SeqCst);
            200i32
        },
    );

    assert_eq!(ra, 100);
    assert_eq!(rb, 200);
    assert_eq!(ra + rb, 300);
    assert!(a_done.load(Ordering::SeqCst));
    assert!(b_done.load(Ordering::SeqCst));


    let (x, y) = pool.install(|| rayon_core::join(|| 5u32, || 6u32));
    assert_eq!(x, 5);
    assert_eq!(y, 6);
}

#[test]
fn threadpool_scope_collects_all_spawns() {
    let pool = build_pool(3);

    let counter = Arc::new(AtomicUsize::new(0));
    let sum = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(sum.load(Ordering::SeqCst), 0);

    let result: &'static str = pool.scope(|s| {
        for i in 1..=10usize {
            let c = Arc::clone(&counter);
            let sm = Arc::clone(&sum);
            s.spawn(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                sm.fetch_add(i, Ordering::SeqCst);
            });
        }
        "scope-done"
    });

    assert_eq!(result, "scope-done");
    assert_eq!(counter.load(Ordering::SeqCst), 10);
    let expected: usize = (1..=10).sum();
    assert_eq!(sum.load(Ordering::SeqCst), expected);
    assert_eq!(expected, 55);
    assert_ne!(counter.load(Ordering::SeqCst), 0);


    let nested = Arc::new(AtomicUsize::new(0));
    let n2 = Arc::clone(&nested);
    pool.scope(|s| {
        s.spawn(move |_| {
            n2.fetch_add(99, Ordering::SeqCst);
        });
    });
    assert_eq!(nested.load(Ordering::SeqCst), 99);
}

#[test]
fn threadpool_scope_fifo_runs_all_tasks() {
    let pool = build_pool(2);
    let counter = Arc::new(AtomicUsize::new(0));
    let order: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(order.lock().unwrap().len(), 0);

    let ret: i32 = pool.scope_fifo(|s| {
        for i in 0..8usize {
            let c = Arc::clone(&counter);
            let o = Arc::clone(&order);
            s.spawn_fifo(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                let start = Instant::now();
                loop {
                    if let Ok(mut g) = o.try_lock() {
                        g.push(i);
                        break;
                    }
                    if start.elapsed() > Duration::from_secs(5) {
                        panic!("order lock timeout");
                    }
                    std::thread::yield_now();
                }
            });
        }
        7
    });

    assert_eq!(ret, 7);
    assert_eq!(counter.load(Ordering::SeqCst), 8);
    let recorded = order.lock().unwrap().clone();
    assert_eq!(recorded.len(), 8);
    let mut sorted = recorded.clone();
    sorted.sort();
    assert_eq!(sorted, (0..8).collect::<Vec<usize>>());
    assert_ne!(recorded.iter().sum::<usize>(), 0);
}

#[test]
fn threadpool_in_place_scope_executes_outside_pool() {
    let pool = build_pool(2);



    let outside_idx = rayon_core::current_thread_index();
    assert!(outside_idx.is_none(), "test thread is not a rayon worker");

    let counter = Arc::new(AtomicUsize::new(0));
    let on_worker = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(on_worker.load(Ordering::SeqCst), 0);

    let r: usize = pool.in_place_scope(|s| {
        for _ in 0..6 {
            let c = Arc::clone(&counter);
            let w = Arc::clone(&on_worker);
            s.spawn(move |_| {
                if rayon_core::current_thread_index().is_some() {
                    w.fetch_add(1, Ordering::SeqCst);
                }
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        123
    });

    assert_eq!(r, 123);
    assert_eq!(counter.load(Ordering::SeqCst), 6);
    assert_eq!(on_worker.load(Ordering::SeqCst), 6);
    assert_ne!(counter.load(Ordering::SeqCst), 0);
}

#[test]
fn threadpool_in_place_scope_fifo_executes_all() {
    let pool = build_pool(2);
    let counter = Arc::new(AtomicUsize::new(0));
    let total = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(total.load(Ordering::SeqCst), 0);

    let returned: bool = pool.in_place_scope_fifo(|s| {
        for i in 1..=5usize {
            let c = Arc::clone(&counter);
            let t = Arc::clone(&total);
            s.spawn_fifo(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                t.fetch_add(i, Ordering::SeqCst);
            });
        }
        true
    });

    assert!(returned);
    assert_eq!(counter.load(Ordering::SeqCst), 5);
    assert_eq!(total.load(Ordering::SeqCst), 15);
    assert_ne!(total.load(Ordering::SeqCst), 0);
    assert_eq!(returned, true);
}

#[test]
fn threadpool_spawn_fifo_runs_in_pool() {
    let pool = build_pool(2);

    let done = Arc::new(AtomicBool::new(false));
    let in_pool = Arc::new(AtomicBool::new(false));
    let value = Arc::new(AtomicUsize::new(0));

    assert!(!done.load(Ordering::SeqCst));
    assert!(!in_pool.load(Ordering::SeqCst));
    assert_eq!(value.load(Ordering::SeqCst), 0);

    let d = Arc::clone(&done);
    let p = Arc::clone(&in_pool);
    let v = Arc::clone(&value);

    pool.spawn_fifo(move || {
        if rayon_core::current_thread_index().is_some() {
            p.store(true, Ordering::SeqCst);
        }
        v.store(777, Ordering::SeqCst);
        d.store(true, Ordering::SeqCst);
    });

    let start = Instant::now();
    while !done.load(Ordering::SeqCst) {
        if start.elapsed() > Duration::from_secs(10) {
            panic!("spawn_fifo task never completed");
        }
        std::thread::yield_now();
    }

    assert!(done.load(Ordering::SeqCst));
    assert!(in_pool.load(Ordering::SeqCst));
    assert_eq!(value.load(Ordering::SeqCst), 777);
    assert_ne!(value.load(Ordering::SeqCst), 0);
}

#[test]
fn threadpool_spawn_broadcast_visits_every_worker() {
    let pool = build_pool(4);

    let visits = Arc::new(AtomicUsize::new(0));
    let indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    assert_eq!(visits.load(Ordering::SeqCst), 0);
    assert_eq!(indices.lock().unwrap().len(), 0);

    let v = Arc::clone(&visits);
    let ix = Arc::clone(&indices);
    pool.spawn_broadcast(move |ctx: BroadcastContext<'_>| {
        let idx = ctx.index();
        let start = Instant::now();
        loop {
            if let Ok(mut g) = ix.try_lock() {
                g.push(idx);
                break;
            }
            if start.elapsed() > Duration::from_secs(5) {
                panic!("indices lock timeout");
            }
            std::thread::yield_now();
        }
        v.fetch_add(1, Ordering::SeqCst);
    });


    let start = Instant::now();
    while visits.load(Ordering::SeqCst) < 4 {
        if start.elapsed() > Duration::from_secs(10) {
            panic!(
                "spawn_broadcast did not reach all workers; got {}",
                visits.load(Ordering::SeqCst)
            );
        }
        std::thread::yield_now();
    }

    assert_eq!(visits.load(Ordering::SeqCst), 4);
    let mut got = indices.lock().unwrap().clone();
    got.sort();
    assert_eq!(got, vec![0, 1, 2, 3]);
    assert_eq!(got.len(), 4);
    assert_ne!(got, vec![0, 0, 0, 0]);
}

#[test]
fn threadpool_combined_workflow() {
    let pool = build_pool(3);


    let sum = pool.install(|| {
        let (a, b) = rayon_core::join(|| (1..=50u64).sum::<u64>(), || (51..=100u64).sum::<u64>());
        a + b
    });
    let expected: u64 = (1..=100).sum();
    assert_eq!(sum, expected);
    assert_eq!(sum, 5050);


    let idxs = pool.broadcast(|ctx| ctx.index());
    assert_eq!(idxs.len(), 3);
    let mut sorted = idxs.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2]);


    let acc = Arc::new(AtomicUsize::new(0));
    pool.scope(|s| {
        for _ in 0..3 {
            let a = Arc::clone(&acc);
            let s_val = sum as usize;
            s.spawn(move |_| {
                a.fetch_add(s_val, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(acc.load(Ordering::SeqCst), 5050 * 3);
    assert_eq!(acc.load(Ordering::SeqCst), 15150);


    let flag = Arc::new(AtomicBool::new(false));
    let f2 = Arc::clone(&flag);
    pool.spawn_fifo(move || {
        f2.store(true, Ordering::SeqCst);
    });
    let start = Instant::now();
    while !flag.load(Ordering::SeqCst) {
        if start.elapsed() > Duration::from_secs(10) {
            panic!("spawn_fifo never completed");
        }
        std::thread::yield_now();
    }
    assert!(flag.load(Ordering::SeqCst));
    assert_ne!(sum, 0);
}