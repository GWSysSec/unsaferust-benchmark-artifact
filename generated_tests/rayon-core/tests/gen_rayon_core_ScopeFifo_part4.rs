use rayon_core::{in_place_scope_fifo, join, scope_fifo, ThreadPoolBuilder};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::Mutex;

#[test]
fn scope_fifo_spawn_fifo_with_join_inside_task() {
    let pool = ThreadPoolBuilder::new().num_threads(2).build().unwrap();

    let total = AtomicI64::new(0);
    let tasks = AtomicUsize::new(0);

    assert_eq!(total.load(Ordering::SeqCst), 0);
    assert_eq!(tasks.load(Ordering::SeqCst), 0);

    pool.install(|| {
        scope_fifo(|s| {
            for k in 1..=5i64 {
                let t_ref = &total;
                let tc_ref = &tasks;
                s.spawn_fifo(move |_| {
                    let (a, b) = join(|| k * k, || k * k * k);
                    t_ref.fetch_add(a + b, Ordering::SeqCst);
                    tc_ref.fetch_add(1, Ordering::SeqCst);
                });
            }
        });
    });

    let expected: i64 = (1..=5i64).map(|k| k * k + k * k * k).sum();
    let got = total.load(Ordering::SeqCst);
    let task_count = tasks.load(Ordering::SeqCst);

    assert_eq!(got, expected);
    assert_eq!(task_count, 5);
    assert_ne!(got, 0);
    assert_ne!(task_count, 0);
    assert_eq!(expected, (1 + 1) + (4 + 8) + (9 + 27) + (16 + 64) + (25 + 125));
    assert!(got > 0);
}

#[test]
fn scope_fifo_single_thread_strict_fifo_order() {
    let pool = ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let log: Mutex<Vec<usize>> = Mutex::new(Vec::new());

    assert_eq!(log.lock().unwrap().len(), 0);

    pool.install(|| {
        scope_fifo(|s| {
            for i in 0..16usize {
                let l = &log;
                s.spawn_fifo(move |_| {
                    l.lock().unwrap().push(i);
                });
            }
        });
    });

    let recorded = log.into_inner().unwrap();
    let expected: Vec<usize> = (0..16).collect();
    assert_eq!(recorded.len(), 16);
    assert_eq!(recorded, expected);
    assert_eq!(*recorded.first().unwrap(), 0);
    assert_eq!(*recorded.last().unwrap(), 15);
    assert_ne!(recorded.first(), recorded.last());
    let sum: usize = recorded.iter().sum();
    assert_eq!(sum, (0..16).sum());
    assert_ne!(sum, 0);
}

#[test]
fn in_place_scope_fifo_recursive_spawn_fifo_tree() {

    let leaves = AtomicUsize::new(0);
    let internal = AtomicUsize::new(0);

    assert_eq!(leaves.load(Ordering::SeqCst), 0);
    assert_eq!(internal.load(Ordering::SeqCst), 0);

    fn recurse<'a>(
        s: &rayon_core::ScopeFifo<'a>,
        depth: usize,
        leaves: &'a AtomicUsize,
        internal: &'a AtomicUsize,
    ) {
        if depth == 3 {
            leaves.fetch_add(1, Ordering::SeqCst);
            return;
        }
        internal.fetch_add(1, Ordering::SeqCst);
        for _ in 0..2 {
            s.spawn_fifo(move |s2| recurse(s2, depth + 1, leaves, internal));
        }
    }

    in_place_scope_fifo(|s| {
        recurse(s, 0, &leaves, &internal);
    });

    let leaf_count = leaves.load(Ordering::SeqCst);
    let internal_count = internal.load(Ordering::SeqCst);


    assert_eq!(leaf_count, 8);
    assert_eq!(internal_count, 7);
    assert_ne!(leaf_count, 0);
    assert_ne!(internal_count, 0);
    assert!(leaf_count > internal_count - 7 + 1);
    assert_eq!(leaf_count + internal_count, 15);
}