use rayon_core::{in_place_scope_fifo, scope_fifo, ThreadPoolBuilder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[test]
fn scope_fifo_spawn_fifo_pipeline_stages() {
    let pool = ThreadPoolBuilder::new().num_threads(1).build().unwrap();

    let stage_a: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    let stage_b: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    let stage_c = AtomicUsize::new(0);

    assert_eq!(stage_a.lock().unwrap().len(), 0);
    assert_eq!(stage_b.lock().unwrap().len(), 0);
    assert_eq!(stage_c.load(Ordering::SeqCst), 0);

    let inputs: Vec<u32> = (1..=6).collect();
    assert_eq!(inputs.len(), 6);

    pool.install(|| {
        scope_fifo(|s| {
            for &x in &inputs {
                let a_ref = &stage_a;
                let b_ref = &stage_b;
                let c_ref = &stage_c;
                s.spawn_fifo(move |s2| {
                    let v = x * 2;
                    a_ref.lock().unwrap().push(v);
                    s2.spawn_fifo(move |s3| {
                        let w = v + 1;
                        b_ref.lock().unwrap().push(w);
                        s3.spawn_fifo(move |_| {
                            c_ref.fetch_add(w as usize, Ordering::SeqCst);
                        });
                    });
                });
            }
        });
    });

    let a = stage_a.into_inner().unwrap();
    let b = stage_b.into_inner().unwrap();
    let c = stage_c.load(Ordering::SeqCst);

    assert_eq!(a.len(), 6);
    assert_eq!(b.len(), 6);

    assert_eq!(a, vec![2, 4, 6, 8, 10, 12]);
    assert_eq!(b, vec![3, 5, 7, 9, 11, 13]);
    let expected_c: usize = (3 + 5 + 7 + 9 + 11 + 13) as usize;
    assert_eq!(c, expected_c);
    assert_ne!(c, 0);
    assert_ne!(a, b);
}

#[test]
fn in_place_scope_fifo_spawn_fifo_partitioned_sum() {
    let data: Vec<u64> = (1..=100).collect();
    assert_eq!(data.len(), 100);
    assert_eq!(*data.first().unwrap(), 1);
    assert_eq!(*data.last().unwrap(), 100);

    let chunk_sums: Mutex<Vec<u64>> = Mutex::new(Vec::new());
    let task_count = AtomicUsize::new(0);

    assert_eq!(chunk_sums.lock().unwrap().len(), 0);
    assert_eq!(task_count.load(Ordering::SeqCst), 0);

    in_place_scope_fifo(|s| {
        for chunk in data.chunks(10) {
            let sums_ref = &chunk_sums;
            let tc_ref = &task_count;
            s.spawn_fifo(move |_| {
                let local: u64 = chunk.iter().sum();
                sums_ref.lock().unwrap().push(local);
                tc_ref.fetch_add(1, Ordering::SeqCst);
            });
        }
    });

    let sums = chunk_sums.into_inner().unwrap();
    let tasks = task_count.load(Ordering::SeqCst);

    assert_eq!(tasks, 10);
    assert_eq!(sums.len(), 10);
    let total: u64 = sums.iter().sum();
    assert_eq!(total, 5050);
    assert_ne!(total, 0);

    let mut sorted = sums.clone();
    sorted.sort();
    assert_eq!(sorted.first().copied(), Some(55));
    assert_eq!(sorted.last().copied(), Some(955));
}

#[test]
fn scope_fifo_spawn_fifo_returns_aggregate() {
    let pool = ThreadPoolBuilder::new().num_threads(2).build().unwrap();

    let counter = AtomicUsize::new(0);
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let result: i64 = pool.install(|| {
        scope_fifo(|s| {
            for _ in 0..20usize {
                let cr = &counter;
                s.spawn_fifo(move |_| {
                    cr.fetch_add(1, Ordering::SeqCst);
                });
            }
            -7i64
        })
    });

    let final_count = counter.load(Ordering::SeqCst);
    assert_eq!(result, -7);
    assert_eq!(final_count, 20);
    assert_ne!(final_count, 0);
    assert_ne!(result, 0);
    assert!(result < 0);
    assert!(final_count > 10);
}