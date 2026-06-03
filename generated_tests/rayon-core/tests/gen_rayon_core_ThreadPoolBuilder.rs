use rayon_core::ThreadPoolBuilder;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_thread_pool_builder_num_threads() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    assert_eq!(pool.current_num_threads(), 4);
}

#[test]
fn test_thread_pool_builder_default() {
    let pool = ThreadPoolBuilder::default()
        .build()
        .unwrap();
    assert!(pool.current_num_threads() > 0);
}

#[test]
fn test_thread_pool_builder_install() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let result = pool.install(|| {
        rayon_core::current_num_threads()
    });
    assert_eq!(result, 2);
}

#[test]
fn test_thread_pool_builder_spawn() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    pool.install(move || {
        rayon_core::scope(|s| {
            s.spawn(|_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
        });
    });
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_thread_pool_builder_broadcast() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .unwrap();
    let results = pool.install(|| {
        rayon_core::broadcast(|ctx| ctx.index())
    });
    assert_eq!(results.len(), 3);
}

#[test]
fn test_thread_pool_builder_join() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let (a, b) = pool.install(|| {
        rayon_core::join(|| 1 + 1, || 2 + 2)
    });
    assert_eq!(a, 2);
    assert_eq!(b, 4);
}

#[test]
fn test_thread_pool_builder_scope() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let counter = AtomicUsize::new(0);
    pool.install(|| {
        rayon_core::scope(|s| {
            for _ in 0..10 {
                s.spawn(|_| {
                    counter.fetch_add(1, Ordering::SeqCst);
                });
            }
        });
    });
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

#[test]
fn test_thread_pool_builder_thread_name() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .thread_name(|index| format!("worker-{}", index))
        .build()
        .unwrap();
    assert_eq!(pool.current_num_threads(), 2);
}

#[test]
fn test_thread_pool_builder_stack_size() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .stack_size(4 * 1024 * 1024)
        .build()
        .unwrap();
    let result = pool.install(|| 42);
    assert_eq!(result, 42);
}

#[test]
fn test_thread_pool_builder_multiple_pools() {
    let pool1 = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let pool2 = ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .unwrap();
    assert_eq!(pool1.current_num_threads(), 2);
    assert_eq!(pool2.current_num_threads(), 3);
}