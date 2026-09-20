#![allow(deprecated)]

use rayon_core::{
    broadcast, current_num_threads, initialize, max_num_threads, scope, spawn,
    BroadcastContext, Configuration,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn configuration_panic_payload_and_name_format() {
    // Keep the worker count modest. `Configuration` doesn't expose `num_threads`
    // in our allowed surface, so we lean on RAYON_NUM_THREADS, which rayon-core
    // honors at initialize time.
    std::env::set_var("RAYON_NUM_THREADS", "3");

    let start_count = Arc::new(AtomicUsize::new(0));
    let exit_count = Arc::new(AtomicUsize::new(0));
    let panic_count = Arc::new(AtomicUsize::new(0));
    let panic_msg_match = Arc::new(AtomicUsize::new(0));
    let start_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let exit_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    // Pre-state: every accumulator starts empty.
    assert_eq!(start_count.load(Ordering::SeqCst), 0);
    assert_eq!(exit_count.load(Ordering::SeqCst), 0);
    assert_eq!(panic_count.load(Ordering::SeqCst), 0);
    assert_eq!(panic_msg_match.load(Ordering::SeqCst), 0);
    assert_eq!(start_indices.lock().unwrap().len(), 0);
    assert_eq!(exit_indices.lock().unwrap().len(), 0);

    let sc = Arc::clone(&start_count);
    let si = Arc::clone(&start_indices);
    let ec = Arc::clone(&exit_count);
    let ei = Arc::clone(&exit_indices);
    let pc = Arc::clone(&panic_count);
    let pm = Arc::clone(&panic_msg_match);

    let config = Configuration::new()
        .thread_name(|i| format!("payload-w{}", i))
        .start_handler(move |idx| {
            sc.fetch_add(1, Ordering::SeqCst);
            si.lock().unwrap().push(idx);
        })
        .exit_handler(move |idx| {
            ec.fetch_add(1, Ordering::SeqCst);
            ei.lock().unwrap().push(idx);
        })
        .panic_handler(move |payload| {
            pc.fetch_add(1, Ordering::SeqCst);
            let matched = if let Some(s) = payload.downcast_ref::<&'static str>() {
                *s == "rcore-test-panic-marker"
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s == "rcore-test-panic-marker"
            } else {
                false
            };
            if matched {
                pm.fetch_add(1, Ordering::SeqCst);
            }
        });

    let init_result = initialize(config);
    assert!(
        init_result.is_ok(),
        "initialize must succeed in a fresh binary: {:?}",
        init_result.err()
    );

    let n = current_num_threads();
    let max_n = max_num_threads();
    assert!(n >= 1, "expected at least 1 worker, got {}", n);
    assert!(n <= max_n, "current {} should be <= max {}", n, max_n);

    // Force every worker to wake so thread_name + start_handler fire universally.
    let observed_names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let names_c = Arc::clone(&observed_names);
    let indices: Vec<usize> = broadcast(move |ctx: BroadcastContext<'_>| {
        let nm = std::thread::current()
            .name()
            .unwrap_or("<unnamed>")
            .to_string();
        names_c.lock().unwrap().push(nm);
        ctx.index()
    });

    // broadcast yields exactly one value per worker, indexed 0..n.
    assert_eq!(indices.len(), n, "broadcast yields one result per worker");
    let mut sorted = indices.clone();
    sorted.sort();
    let expected_idx: Vec<usize> = (0..n).collect();
    assert_eq!(sorted, expected_idx, "broadcast indices must cover 0..n");

    // thread_name closure output must propagate into std::thread::current().name().
    let names = observed_names.lock().unwrap().clone();
    assert_eq!(names.len(), n, "one observed name per worker");
    for nm in &names {
        assert!(nm.starts_with("payload-w"), "unexpected thread name: {}", nm);
        let suffix = &nm["payload-w".len()..];
        let parsed: usize = suffix
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric suffix in '{}'", nm));
        assert!(parsed < n, "thread index {} out of range 0..{}", parsed, n);
    }

    // Names must be unique per worker.
    let mut uniq: Vec<String> = names.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), n, "every worker must have a distinct name");

    // start_handler must have fired exactly once per worker, on its own index.
    assert_eq!(
        start_count.load(Ordering::SeqCst),
        n,
        "start_handler invocation count should equal worker count"
    );
    let mut start_idx_sorted = start_indices.lock().unwrap().clone();
    start_idx_sorted.sort();
    assert_eq!(
        start_idx_sorted, expected_idx,
        "start_handler should observe each worker index once"
    );

    // Trigger a panic with a known payload and verify panic_handler sees it.
    spawn(|| {
        panic!("rcore-test-panic-marker");
    });

    let mut waited_ms: u32 = 0;
    while panic_count.load(Ordering::SeqCst) == 0 && waited_ms < 5000 {
        std::thread::sleep(Duration::from_millis(10));
        waited_ms += 10;
    }
    assert_eq!(
        panic_count.load(Ordering::SeqCst),
        1,
        "panic_handler must fire exactly once for a single panicking spawn"
    );
    assert_eq!(
        panic_msg_match.load(Ordering::SeqCst),
        1,
        "panic_handler must receive the marker payload"
    );

    // Pool stays usable after a panic. Drive a sum-of-squares scope.
    let acc = Arc::new(AtomicUsize::new(0));
    let acc_in = Arc::clone(&acc);
    scope(|s| {
        for i in 1..=10usize {
            let a = Arc::clone(&acc_in);
            s.spawn(move |_| {
                a.fetch_add(i * i, Ordering::SeqCst);
            });
        }
    });
    let expected_sum: usize = (1..=10usize).map(|x| x * x).sum();
    assert_eq!(
        acc.load(Ordering::SeqCst),
        expected_sum,
        "scope must complete every spawned task before returning"
    );

    // exit_handler must not have fired: workers are still alive.
    assert_eq!(
        exit_count.load(Ordering::SeqCst),
        0,
        "exit_handler must not run while workers are active"
    );
    assert_eq!(
        exit_indices.lock().unwrap().len(),
        0,
        "no exit indices should be recorded yet"
    );

    // start_handler should not re-fire on already-running workers.
    assert_eq!(
        start_count.load(Ordering::SeqCst),
        n,
        "start_handler should not re-fire for the post-panic workload"
    );

    // No spurious panic_handler invocations from the post-panic workload.
    assert_eq!(
        panic_count.load(Ordering::SeqCst),
        1,
        "non-panicking work must not trigger panic_handler"
    );

    // Trigger a second panic to prove the handler stays installed.
    spawn(|| {
        panic!("rcore-test-panic-marker");
    });
    let mut waited2_ms: u32 = 0;
    while panic_count.load(Ordering::SeqCst) < 2 && waited2_ms < 5000 {
        std::thread::sleep(Duration::from_millis(10));
        waited2_ms += 10;
    }
    assert_eq!(
        panic_count.load(Ordering::SeqCst),
        2,
        "panic_handler must fire for the second panicking spawn"
    );
    assert_eq!(
        panic_msg_match.load(Ordering::SeqCst),
        2,
        "second panic payload must also match the marker"
    );

    // Final exit_handler check: still no exit since pool isn't shutting down.
    assert_eq!(
        exit_count.load(Ordering::SeqCst),
        0,
        "exit_handler must remain unfired during the test"
    );
}