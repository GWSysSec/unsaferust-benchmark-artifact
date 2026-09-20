use rayon_core::{BroadcastContext, ThreadPool, ThreadPoolBuilder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn make_pool(n: usize) -> ThreadPool {
    ThreadPoolBuilder::new().num_threads(n).build().unwrap()
}

fn wait_until<F: Fn() -> bool>(cond: F, max_ms: u64) -> bool {
    let start = Instant::now();
    while !cond() {
        if start.elapsed() > Duration::from_millis(max_ms) {
            return false;
        }
        std::thread::yield_now();
    }
    true
}

#[test]
fn pool_install_runs_op_on_pool_thread() {
    let pool = make_pool(2);
    assert!(rayon_core::current_thread_index().is_none());

    let r = pool.install(|| {
        let idx = rayon_core::current_thread_index();
        assert!(idx.is_some());
        let i = idx.unwrap();
        assert!(i < 2);
        assert_eq!(rayon_core::current_num_threads(), 2);
        i + 100
    });
    assert!(r == 100 || r == 101);
    assert!(rayon_core::current_thread_index().is_none());

    let s: i64 = (0..5i64).map(|i| pool.install(move || i * 10)).sum();
    assert_eq!(s, 100);

    let r2 = pool.install(|| "hello".to_string());
    assert_eq!(r2.as_str(), "hello");
    assert_eq!(r2.len(), 5);
}

#[test]
fn pool_broadcast_runs_on_each_thread() {
    let pool = make_pool(4);

    let results: Vec<usize> = pool.broadcast(|ctx: BroadcastContext<'_>| ctx.index() * 10);
    assert_eq!(results.len(), 4);

    let sum: usize = results.iter().copied().sum();
    assert_eq!(sum, 60);

    let mut sorted = results.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 10, 20, 30]);

    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let res2: Vec<()> = pool.broadcast(move |_| {
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(res2.len(), 4);
    assert_eq!(counter.load(Ordering::SeqCst), 4);

    let indices = pool.broadcast(|ctx| ctx.index());
    let mut seen = [false; 4];
    for &i in &indices {
        assert!(i < 4);
        assert!(!seen[i]);
        seen[i] = true;
    }
    assert!(seen.iter().all(|&b| b));
}

#[test]
fn pool_join_runs_both_and_returns_pair() {
    let pool = make_pool(2);

    let (a, b) = pool.join(|| 1 + 2, || 3 + 4);
    assert_eq!(a, 3);
    assert_eq!(b, 7);

    let counter = Arc::new(AtomicUsize::new(0));
    let c1 = counter.clone();
    let c2 = counter.clone();
    let (ra, rb) = pool.join(
        move || c1.fetch_add(10, Ordering::SeqCst),
        move || c2.fetch_add(20, Ordering::SeqCst),
    );
    assert_eq!(counter.load(Ordering::SeqCst), 30);
    assert!(ra == 0 || ra == 20);
    assert!(rb == 0 || rb == 10);
    assert_ne!(ra, rb);

    fn psum(p: &ThreadPool, v: &[i64]) -> i64 {
        if v.len() <= 4 {
            return v.iter().sum();
        }
        let mid = v.len() / 2;
        let (l, r) = v.split_at(mid);
        let (sl, sr) = p.join(|| psum(p, l), || psum(p, r));
        sl + sr
    }
    let v: Vec<i64> = (1..=100).collect();
    assert_eq!(psum(&pool, &v), 5050);

    let (s1, s2) = pool.join(|| "abc".len(), || "wxyz".len());
    assert_eq!(s1, 3);
    assert_eq!(s2, 4);
}

#[test]
fn pool_scope_spawns_complete_before_return() {
    let pool = make_pool(2);
    let counter = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let res = pool.scope(|s| {
        for _ in 0..16 {
            let c = counter.clone();
            s.spawn(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        7u32
    });

    assert_eq!(res, 7);
    assert_eq!(counter.load(Ordering::SeqCst), 16);

    let nested = Arc::new(AtomicUsize::new(0));
    pool.scope(|s| {
        let n1 = nested.clone();
        s.spawn(move |sub| {
            for _ in 0..3 {
                let n2 = n1.clone();
                sub.spawn(move |_| {
                    n2.fetch_add(2, Ordering::SeqCst);
                });
            }
        });
    });
    assert_eq!(nested.load(Ordering::SeqCst), 6);

    let after = pool.install(|| 99);
    assert_eq!(after, 99);
    assert!(rayon_core::current_thread_index().is_none());
}

#[test]
fn pool_scope_fifo_completes_all_tasks() {
    let pool = make_pool(2);
    let counter = Arc::new(AtomicUsize::new(0));
    let max_seen = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(max_seen.load(Ordering::SeqCst), 0);

    pool.scope_fifo(|s| {
        for i in 0..20usize {
            let c = counter.clone();
            let m = max_seen.clone();
            s.spawn_fifo(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                m.fetch_max(i, Ordering::SeqCst);
            });
        }
    });

    assert_eq!(counter.load(Ordering::SeqCst), 20);
    assert_eq!(max_seen.load(Ordering::SeqCst), 19);

    let v = pool.scope_fifo(|_| vec![1, 2, 3]);
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], 1);
    assert_eq!(v[2], 3);

    let n = pool.scope_fifo(|_| 42);
    assert_eq!(n, 42);
}

#[test]
fn pool_in_place_scope_runs_body_on_caller() {
    let pool = make_pool(2);
    let counter = Arc::new(AtomicUsize::new(0));

    assert!(rayon_core::current_thread_index().is_none());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let body_idx = pool.in_place_scope(|s| {
        for _ in 0..10 {
            let c = counter.clone();
            s.spawn(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        rayon_core::current_thread_index()
    });

    assert!(body_idx.is_none(), "body must run on caller thread");
    assert_eq!(counter.load(Ordering::SeqCst), 10);

    let r = pool.in_place_scope(|_| 12345i64);
    assert_eq!(r, 12345);

    let v = Arc::new(AtomicUsize::new(0));
    pool.in_place_scope(|s| {
        let v1 = v.clone();
        s.spawn(move |_| {
            v1.fetch_add(100, Ordering::SeqCst);
        });
    });
    assert_eq!(v.load(Ordering::SeqCst), 100);
    assert!(rayon_core::current_thread_index().is_none());
}

#[test]
fn pool_in_place_scope_fifo_runs_body_on_caller() {
    let pool = make_pool(2);
    let counter = Arc::new(AtomicUsize::new(0));

    assert!(rayon_core::current_thread_index().is_none());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let body_idx = pool.in_place_scope_fifo(|s| {
        for _ in 0..8 {
            let c = counter.clone();
            s.spawn_fifo(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        rayon_core::current_thread_index()
    });

    assert!(body_idx.is_none());
    assert_eq!(counter.load(Ordering::SeqCst), 8);

    let v = pool.in_place_scope_fifo(|_| vec![10, 20, 30, 40]);
    assert_eq!(v.len(), 4);
    let s: i32 = v.iter().sum();
    assert_eq!(s, 100);
    assert_eq!(v[0], 10);
    assert_eq!(v[3], 40);
    assert!(rayon_core::current_thread_index().is_none());
}

#[test]
fn pool_spawn_fifo_executes_async() {
    let pool = make_pool(2);
    let counter = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);

    for _ in 0..12 {
        let c = counter.clone();
        pool.spawn_fifo(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    let ok = wait_until(|| counter.load(Ordering::SeqCst) == 12, 5000);
    assert!(ok, "spawn_fifo tasks should complete");
    assert_eq!(counter.load(Ordering::SeqCst), 12);

    let counter2 = Arc::new(AtomicUsize::new(0));
    for _ in 0..5 {
        let c = counter2.clone();
        pool.spawn_fifo(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }
    let ok2 = wait_until(|| counter2.load(Ordering::SeqCst) == 5, 5000);
    assert!(ok2);
    assert_eq!(counter2.load(Ordering::SeqCst), 5);

    let r = pool.install(|| 7);
    assert_eq!(r, 7);
}

#[test]
fn pool_spawn_broadcast_runs_on_all_threads() {
    let pool = make_pool(3);
    let counter = Arc::new(AtomicUsize::new(0));
    let bitset = Arc::new(AtomicUsize::new(0));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(bitset.load(Ordering::SeqCst), 0);

    let c = counter.clone();
    let b = bitset.clone();
    pool.spawn_broadcast(move |ctx| {
        c.fetch_add(1, Ordering::SeqCst);
        b.fetch_or(1usize << ctx.index(), Ordering::SeqCst);
    });

    let ok = wait_until(|| counter.load(Ordering::SeqCst) == 3, 5000);
    assert!(ok, "spawn_broadcast must reach every thread");
    assert_eq!(counter.load(Ordering::SeqCst), 3);
    assert_eq!(bitset.load(Ordering::SeqCst), 0b111);

    let counter2 = Arc::new(AtomicUsize::new(0));
    let c2 = counter2.clone();
    pool.spawn_broadcast(move |_| {
        c2.fetch_add(5, Ordering::SeqCst);
    });
    let ok2 = wait_until(|| counter2.load(Ordering::SeqCst) == 15, 5000);
    assert!(ok2);
    assert_eq!(counter2.load(Ordering::SeqCst), 15);
}

#[test]
fn pool_current_thread_has_pending_tasks_returns_correctly() {
    let pool = make_pool(2);
    let other = make_pool(1);

    let outside = pool.current_thread_has_pending_tasks();
    assert!(outside.is_none(), "outside any pool should be None");
    assert_eq!(outside, None);

    let inside = pool.install(|| pool.current_thread_has_pending_tasks());
    assert!(inside.is_some());
    assert_eq!(inside, Some(false));

    let foreign = other.install(|| pool.current_thread_has_pending_tasks());
    assert!(foreign.is_none(), "from different pool must be None");

    let g = pool.install(|| rayon_core::current_thread_has_pending_tasks());
    assert!(g.is_some());

    let after = pool.current_thread_has_pending_tasks();
    assert_eq!(after, None);

    let inside_other = other.install(|| other.current_thread_has_pending_tasks());
    assert_eq!(inside_other, Some(false));
}