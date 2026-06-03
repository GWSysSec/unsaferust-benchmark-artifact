use rayon_core::{
    in_place_scope_fifo, scope_fifo, BroadcastContext, ScopeFifo,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[test]
fn scope_fifo_spawn_fifo_completes_all_tasks() {
    let counter = Arc::new(AtomicUsize::new(0));
    let recorded: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));


    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(recorded.lock().unwrap().len(), 0);
    assert!(recorded.lock().unwrap().is_empty());

    let n_tasks: usize = 32;
    let returned: usize = scope_fifo(|s: &ScopeFifo<'_>| {
        for i in 0..n_tasks {
            let c = Arc::clone(&counter);
            let r = Arc::clone(&recorded);
            s.spawn_fifo(move |_inner: &ScopeFifo<'_>| {
                c.fetch_add(1, Ordering::SeqCst);
                let start = Instant::now();
                loop {
                    if let Ok(mut g) = r.try_lock() {
                        g.push(i);
                        break;
                    }
                    if start.elapsed() > Duration::from_secs(5) {
                        panic!("timed out acquiring recorded mutex");
                    }
                    std::thread::yield_now();
                }
            });
        }
        n_tasks * 2
    });


    assert_eq!(returned, n_tasks * 2);
    assert_eq!(counter.load(Ordering::SeqCst), n_tasks);

    let final_record = recorded.lock().expect("scope finished, lock free");
    assert_eq!(final_record.len(), n_tasks);


    let mut sorted = final_record.clone();
    sorted.sort();
    let expected: Vec<usize> = (0..n_tasks).collect();
    assert_eq!(sorted, expected);


    assert!(sorted.contains(&0));
    assert!(sorted.contains(&(n_tasks - 1)));
    assert_ne!(sorted.first(), sorted.last());
}

#[test]
fn scope_fifo_spawn_fifo_nested_recursive() {
    let depth_counter = Arc::new(AtomicUsize::new(0));
    let leaf_counter = Arc::new(AtomicUsize::new(0));

    fn recurse(s: &ScopeFifo<'_>, depth: usize, dc: Arc<AtomicUsize>, lc: Arc<AtomicUsize>) {
        dc.fetch_add(1, Ordering::SeqCst);
        if depth == 0 {
            lc.fetch_add(1, Ordering::SeqCst);
            return;
        }
        for _ in 0..2 {
            let dc2 = Arc::clone(&dc);
            let lc2 = Arc::clone(&lc);
            s.spawn_fifo(move |inner| {
                recurse(inner, depth - 1, dc2, lc2);
            });
        }
    }

    assert_eq!(depth_counter.load(Ordering::SeqCst), 0);
    assert_eq!(leaf_counter.load(Ordering::SeqCst), 0);

    scope_fifo(|s| {
        recurse(s, 4, Arc::clone(&depth_counter), Arc::clone(&leaf_counter));
    });



    let total_nodes: usize = (0..=4).map(|d| 1usize << d).sum();
    assert_eq!(total_nodes, 31);
    assert_eq!(depth_counter.load(Ordering::SeqCst), total_nodes);
    assert_eq!(leaf_counter.load(Ordering::SeqCst), 16);
    assert!(leaf_counter.load(Ordering::SeqCst) < depth_counter.load(Ordering::SeqCst));
    assert_ne!(depth_counter.load(Ordering::SeqCst), 0);
}

#[test]
fn scope_fifo_in_place_scope_fifo_basic() {
    let counter = Arc::new(AtomicUsize::new(0));
    let n_tasks: usize = 16;

    let result: usize = in_place_scope_fifo(|s: &ScopeFifo<'_>| {
        for _ in 0..n_tasks {
            let c = Arc::clone(&counter);
            s.spawn_fifo(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        42
    });

    assert_eq!(result, 42);
    assert_eq!(counter.load(Ordering::SeqCst), n_tasks);
}