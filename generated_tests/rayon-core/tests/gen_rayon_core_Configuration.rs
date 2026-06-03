#![allow(deprecated)]

use rayon_core::{
    broadcast, current_num_threads, initialize, max_num_threads, scope, spawn, Configuration,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn configuration_all_handlers_lifecycle() {



    std::env::set_var("RAYON_NUM_THREADS", "3");

    let start_count = Arc::new(AtomicUsize::new(0));
    let panic_count = Arc::new(AtomicUsize::new(0));
    let exit_marker = Arc::new(AtomicUsize::new(0));
    let observed_names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let observed_start_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));


    assert_eq!(start_count.load(Ordering::SeqCst), 0);
    assert_eq!(panic_count.load(Ordering::SeqCst), 0);
    assert_eq!(exit_marker.load(Ordering::SeqCst), 0);
    assert_eq!(observed_names.lock().unwrap().len(), 0);
    assert_eq!(observed_start_indices.lock().unwrap().len(), 0);

    let sc = Arc::clone(&start_count);
    let si = Arc::clone(&observed_start_indices);
    let pc = Arc::clone(&panic_count);
    let em = Arc::clone(&exit_marker);

    let config = Configuration::new()
        .thread_name(|i| format!("rcore-cfg-{}", i))
        .start_handler(move |idx| {
            sc.fetch_add(1, Ordering::SeqCst);
            si.lock().unwrap().push(idx);
        })
        .exit_handler(move |_idx| {
            em.fetch_add(1, Ordering::SeqCst);
        })
        .panic_handler(move |_payload| {
            pc.fetch_add(1, Ordering::SeqCst);
        });

    let init_result = initialize(config);
    assert!(
        init_result.is_ok(),
        "initialize must succeed on a fresh binary: {:?}",
        init_result.err()
    );

    let n = current_num_threads();
    let max_n = max_num_threads();
    assert!(n >= 1, "expected at least 1 worker, got {}", n);
    assert!(n <= max_n, "current {} should be <= max {}", n, max_n);


    let names_clone = Arc::clone(&observed_names);
    let collected: Vec<usize> = broadcast(move |ctx| {
        let nm = std::thread::current()
            .name()
            .unwrap_or("<unnamed>")
            .to_string();
        names_clone.lock().unwrap().push(nm);
        ctx.index()
    });


    assert_eq!(collected.len(), n, "broadcast yields one result per worker");
    let mut sorted_idx = collected.clone();
    sorted_idx.sort();
    let expected_idx: Vec<usize> = (0..n).collect();
    assert_eq!(sorted_idx, expected_idx, "broadcast indices cover 0..n");


    assert_eq!(
        start_count.load(Ordering::SeqCst),
        n,
        "start_handler invocation count should equal worker count"
    );
    let mut start_indices = observed_start_indices.lock().unwrap().clone();
    start_indices.sort();
    assert_eq!(
        start_indices, expected_idx,
        "start_handler should observe each worker index exactly once"
    );


    let names = observed_names.lock().unwrap().clone();
    assert_eq!(names.len(), n, "one observed name per worker");
    for name in &names {
        assert!(
            name.starts_with("rcore-cfg-"),
            "thread name '{}' missing expected prefix",
            name
        );
        let suffix = &name["rcore-cfg-".len()..];
        let parsed: usize = suffix
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric suffix in '{}'", name));
        assert!(parsed < n, "thread index {} out of range 0..{}", parsed, n);
    }


    let mut uniq = names.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), n, "every worker must have a distinct name");


    assert_eq!(
        panic_count.load(Ordering::SeqCst),
        0,
        "no panics observed before triggering one"
    );
    spawn(|| {
        panic!("intentional panic exercising panic_handler");
    });



    let mut waited_ms: u32 = 0;
    while panic_count.load(Ordering::SeqCst) == 0 && waited_ms < 5000 {
        std::thread::sleep(Duration::from_millis(10));
        waited_ms += 10;
    }
    assert_eq!(
        panic_count.load(Ordering::SeqCst),
        1,
        "panic_handler must fire exactly once for one panicking spawn"
    );


    let acc = Arc::new(AtomicUsize::new(0));
    let acc_in = Arc::clone(&acc);
    scope(|s| {
        for i in 1..=20usize {
            let a = Arc::clone(&acc_in);
            s.spawn(move |_| {
                a.fetch_add(i, Ordering::SeqCst);
            });
        }
    });
    let expected_sum: usize = (1..=20).sum();
    assert_eq!(
        acc.load(Ordering::SeqCst),
        expected_sum,
        "scope must complete all 20 spawned tasks before returning"
    );


    assert_eq!(
        exit_marker.load(Ordering::SeqCst),
        0,
        "exit_handler must not run while workers are active"
    );


    assert_eq!(
        start_count.load(Ordering::SeqCst),
        n,
        "start_handler should not re-fire on already-running workers"
    );


    assert_eq!(
        panic_count.load(Ordering::SeqCst),
        1,
        "no spurious panic_handler invocations from later workload"
    );
}