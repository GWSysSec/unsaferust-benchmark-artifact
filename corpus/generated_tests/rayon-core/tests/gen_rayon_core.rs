use rayon_core::{
    broadcast, in_place_scope, in_place_scope_fifo, initialize, join, join_context, scope,
    scope_fifo, spawn_broadcast, spawn_fifo, BroadcastContext, Configuration, FnContext,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| {
        let config = Configuration::new();
        let _ = initialize(config);
    });
}

#[test]
fn test_join() {
    init();
    let (a, b) = join(|| 1 + 1, || 2 + 2);
    assert_eq!(a, 2);
    assert_eq!(b, 4);
}

#[test]
fn test_join_context() {
    init();
    let (a, b) = join_context(
        |_ctx: FnContext| 10,
        |_ctx: FnContext| 20,
    );
    assert_eq!(a, 10);
    assert_eq!(b, 20);
}

#[test]
fn test_scope() {
    init();
    let counter = AtomicUsize::new(0);
    scope(|s| {
        for _ in 0..4 {
            s.spawn(|_| {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn test_scope_fifo() {
    init();
    let counter = AtomicUsize::new(0);
    scope_fifo(|s| {
        for _ in 0..4 {
            s.spawn_fifo(|_| {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn test_in_place_scope() {
    init();
    let counter = AtomicUsize::new(0);
    in_place_scope(|s| {
        for _ in 0..4 {
            s.spawn(|_| {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn test_in_place_scope_fifo() {
    init();
    let counter = AtomicUsize::new(0);
    in_place_scope_fifo(|s| {
        for _ in 0..4 {
            s.spawn_fifo(|_| {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn test_broadcast() {
    init();
    let results = broadcast(|ctx: BroadcastContext<'_>| {
        ctx.index()
    });
    let set: HashSet<usize> = results.into_iter().collect();
    assert!(!set.is_empty());
}

#[test]
fn test_spawn_broadcast() {
    init();
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();
    spawn_broadcast(move |_ctx: BroadcastContext<'_>| {
        flag_clone.store(true, Ordering::SeqCst);
    });
    let start = Instant::now();
    while !flag.load(Ordering::SeqCst) {
        if start.elapsed() > Duration::from_secs(5) {
            break;
        }
        std::thread::yield_now();
    }
    assert!(flag.load(Ordering::SeqCst));
}

#[test]
fn test_spawn_fifo() {
    init();
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();
    spawn_fifo(move || {
        done_clone.store(true, Ordering::SeqCst);
    });
    let start = Instant::now();
    while !done.load(Ordering::SeqCst) {
        if start.elapsed() > Duration::from_secs(5) {
            break;
        }
        std::thread::yield_now();
    }
    assert!(done.load(Ordering::SeqCst));
}

#[test]
fn test_mutex_with_try_lock() {
    init();
    let data = Arc::new(Mutex::new(Vec::new()));
    scope(|s| {
        for i in 0..4 {
            let data = data.clone();
            s.spawn(move |_| {
                let start = Instant::now();
                loop {
                    if let Ok(mut guard) = data.try_lock() {
                        guard.push(i);
                        break;
                    }
                    if start.elapsed() > Duration::from_secs(5) {
                        panic!("timed out waiting for lock");
                    }
                    std::thread::yield_now();
                }
            });
        }
    });
    let guard = data.lock().expect("should be able to lock after scope");
    assert_eq!(guard.len(), 4);
}