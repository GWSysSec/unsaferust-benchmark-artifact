#![allow(deprecated)]

use rayon_core::{
    broadcast, current_num_threads, initialize, max_num_threads, scope, spawn,
    BroadcastContext, Configuration,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn configuration_chained_handlers_alternate_order() {
    // Keep workers small but >1 to verify per-worker behavior. `num_threads` is
    // not in the allowed surface, so we use the env var rayon-core honors at init.
    std::env::set_var("RAYON_NUM_THREADS", "2");

    let starts = Arc::new(AtomicUsize::new(0));
    let exits = Arc::new(AtomicUsize::new(0));
    let panics = Arc::new(AtomicUsize::new(0));
    let observed_names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let start_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    // Pre-state: every counter and log starts empty.
    assert_eq!(starts.load(Ordering::SeqCst), 0, "starts should begin at 0");
    assert_eq!(exits.load(Ordering::SeqCst), 0, "exits should begin at 0");
    assert_eq!(panics.load(Ordering::SeqCst), 0, "panics should begin at 0");
    assert_eq!(observed_names.lock().unwrap().len(), 0);
    assert_eq!(start_indices.lock().unwrap().len(), 0);

    let s = Arc::clone(&starts);
    let si = Arc::clone(&start_indices);
    let e = Arc::clone(&exits);
    let p = Arc::clone(&panics);

    // Chain handlers in panic -> exit -> start -> name order to exercise that
    // builder methods are commutative w.r.t. final pool config.
    let cfg = Configuration::new()
        .panic_handler(move |_payload| {
            p.fetch_add(1, Ordering::SeqCst);
        })
        .exit_handler(move |_idx: usize| {
            e.fetch_add(1, Ordering::SeqCst);
        })
        .start_handler(move |idx: usize| {
            s.fetch_add(1, Ordering::SeqCst);
            si.lock().unwrap().push(idx);
        })
        .thread_name(|i| format!("alt-worker-#{}", i));

    let init_result = initialize(cfg);
    assert!(
        init_result.is_ok(),
        "initialize must succeed in a fresh test binary: {:?}",
        init_result.err()
    );

    let n = current_num_threads();
    let max_n = max_num_threads();
    assert!(n >= 1, "current_num_threads must be at least 1, got {}", n);
    assert!(n <= max_n, "current {} should not exceed max {}", n, max_n);

    // Force every worker to run so handlers + names propagate.
    let names_c = Arc::clone(&observed_names);
    let indices: Vec<usize> = broadcast(move |ctx: BroadcastContext<'_>| {
        let nm = std::thread::current()
            .name()
            .unwrap_or("<unnamed>")
            .to_string();
        names_c.lock().unwrap().push(nm);
        ctx.index()
    });

    assert_eq!(indices.len(), n, "broadcast yields one result per worker");
    let mut sorted_idx = indices.clone();
    sorted_idx.sort();
    let expected_idx: Vec<usize> = (0..n).collect();
    assert_eq!(sorted_idx, expected_idx, "broadcast indices cover 0..n");

    // start_handler must have fired exactly once per worker, with each index seen.
    assert_eq!(
        starts.load(Ordering::SeqCst),
        n,
        "start_handler invocations should equal worker count"
    );
    let mut seen_start = start_indices.lock().unwrap().clone();
    seen_start.sort();
    assert_eq!(
        seen_start, expected_idx,
        "start_handler should observe each worker index exactly once"
    );

    // thread_name closure must have set distinct names on each worker.
    let names = observed_names.lock().unwrap().clone();
    assert_eq!(names.len(), n, "one observed name per worker");
    for nm in &names {
        assert!(
            nm.starts_with("alt-worker-#"),
            "thread name '{}' missing expected prefix",
            nm
        );
        let suffix = &nm["alt-worker-#".len()..];
        let parsed: usize = suffix
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric suffix in '{}'", nm));
        assert!(parsed < n, "parsed index {} out of range 0..{}", parsed, n);
    }

    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), n, "every worker must have a distinct name");

    // Drive panic_handler with multiple fire-and-forget panics. Each must be counted.
    let panic_target: usize = 3;
    assert_eq!(panics.load(Ordering::SeqCst), 0, "no panics before triggering");
    for _ in 0..panic_target {
        spawn(|| {
            panic!("intentional panic exercising panic_handler");
        });
    }

    // Poll with 10ms increments, well under 100ms cap and 60s test ceiling.
    let mut waited_ms: u32 = 0;
    while panics.load(Ordering::SeqCst) < panic_target && waited_ms < 5000 {
        std::thread::sleep(Duration::from_millis(10));
        waited_ms += 10;
    }
    assert_eq!(
        panics.load(Ordering::SeqCst),
        panic_target,
        "panic_handler must fire once per panicking spawn"
    );

    // Pool remains usable after panics: drive a scope and verify the exact total.
    let acc = Arc::new(AtomicUsize::new(0));
    let acc_in = Arc::clone(&acc);
    scope(|sc| {
        for i in 1..=10usize {
            let a = Arc::clone(&acc_in);
            sc.spawn(move |_| {
                a.fetch_add(i * 2, Ordering::SeqCst);
            });
        }
    });
    let expected_sum: usize = (1..=10).map(|i| i * 2).sum();
    assert_eq!(
        acc.load(Ordering::SeqCst),
        expected_sum,
        "scope must complete all 10 spawned tasks"
    );

    // exit_handler should not have run while workers are still alive.
    assert_eq!(
        exits.load(Ordering::SeqCst),
        0,
        "exit_handler must not run while pool is alive"
    );

    // start_handler must not re-fire on already-running workers.
    assert_eq!(
        starts.load(Ordering::SeqCst),
        n,
        "start_handler should not re-fire after init"
    );

    // No spurious panic_handler invocations from healthy scope work.
    assert_eq!(
        panics.load(Ordering::SeqCst),
        panic_target,
        "no extra panic_handler invocations from successful scope"
    );

    // A second initialize call must fail because the global pool is now set.
    let second = initialize(Configuration::new());
    assert!(
        second.is_err(),
        "second initialize must fail once global pool is set"
    );
}