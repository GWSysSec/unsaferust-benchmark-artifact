#![allow(deprecated)]

use rayon_core::{
    broadcast, current_num_threads, initialize, max_num_threads, scope, spawn, Configuration,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

static INIT: Once = Once::new();

struct Shared {
    start_count: Arc<AtomicUsize>,
    panic_count: Arc<AtomicUsize>,
    exit_count: Arc<AtomicUsize>,
    names: Arc<Mutex<Vec<String>>>,
    start_indices: Arc<Mutex<Vec<usize>>>,
}

fn shared() -> &'static Shared {
    use std::sync::OnceLock;
    static S: OnceLock<Shared> = OnceLock::new();
    S.get_or_init(|| Shared {
        start_count: Arc::new(AtomicUsize::new(0)),
        panic_count: Arc::new(AtomicUsize::new(0)),
        exit_count: Arc::new(AtomicUsize::new(0)),
        names: Arc::new(Mutex::new(Vec::new())),
        start_indices: Arc::new(Mutex::new(Vec::new())),
    })
}

fn ensure_init() {
    INIT.call_once(|| {
        std::env::set_var("RAYON_NUM_THREADS", "3");
        let s = shared();
        let sc = Arc::clone(&s.start_count);
        let si = Arc::clone(&s.start_indices);
        let pc = Arc::clone(&s.panic_count);
        let ec = Arc::clone(&s.exit_count);

        let cfg = Configuration::new()
            .thread_name(|i| format!("rc-part3-{}", i))
            .start_handler(move |idx| {
                sc.fetch_add(1, Ordering::SeqCst);
                si.lock().unwrap().push(idx);
            })
            .exit_handler(move |_idx| {
                ec.fetch_add(1, Ordering::SeqCst);
            })
            .panic_handler(move |_payload| {
                pc.fetch_add(1, Ordering::SeqCst);
            });

        let res = initialize(cfg);
        assert!(res.is_ok(), "initialize failed: {:?}", res.err());
    });
}

#[test]
fn config_thread_name_and_start_handler() {
    ensure_init();
    let s = shared();

    let n = current_num_threads();
    let max_n = max_num_threads();
    assert!(n >= 1);
    assert!(n <= max_n);
    assert_ne!(max_n, 0);

    let names_log = Arc::clone(&s.names);
    let collected: Vec<usize> = broadcast(move |ctx| {
        let nm = std::thread::current()
            .name()
            .unwrap_or("<unnamed>")
            .to_string();
        names_log.lock().unwrap().push(nm);
        ctx.index()
    });

    assert_eq!(collected.len(), n);
    let mut sorted = collected.clone();
    sorted.sort();
    let expected: Vec<usize> = (0..n).collect();
    assert_eq!(sorted, expected);


    assert_eq!(s.start_count.load(Ordering::SeqCst), n);
    let mut start_idx = s.start_indices.lock().unwrap().clone();
    start_idx.sort();
    assert_eq!(start_idx, expected);


    let names = s.names.lock().unwrap().clone();
    assert!(names.len() >= n);
    let mut prefix_hits = 0usize;
    for nm in &names {
        if nm.starts_with("rc-part3-") {
            prefix_hits += 1;
            let suffix = &nm["rc-part3-".len()..];
            let parsed: usize = suffix.parse().expect("numeric suffix");
            assert!(parsed < n);
        }
    }
    assert_eq!(prefix_hits, names.len());
    assert_ne!(prefix_hits, 0);
}

#[test]
fn config_panic_handler_fires_and_pool_recovers() {
    ensure_init();
    let s = shared();

    let before = s.panic_count.load(Ordering::SeqCst);

    spawn(|| {
        panic!("intentional panic for panic_handler test");
    });

    let mut waited_ms: u32 = 0;
    while s.panic_count.load(Ordering::SeqCst) <= before && waited_ms < 5000 {
        std::thread::sleep(Duration::from_millis(10));
        waited_ms += 10;
    }
    let after = s.panic_count.load(Ordering::SeqCst);
    assert_eq!(after, before + 1);
    assert_ne!(after, before);


    let acc = Arc::new(AtomicUsize::new(0));
    let pre = acc.load(Ordering::SeqCst);
    assert_eq!(pre, 0);

    let acc_in = Arc::clone(&acc);
    scope(|sc| {
        for i in 1..=10usize {
            let a = Arc::clone(&acc_in);
            sc.spawn(move |_| {
                a.fetch_add(i, Ordering::SeqCst);
            });
        }
    });
    let expected: usize = (1..=10).sum();
    assert_eq!(acc.load(Ordering::SeqCst), expected);
    assert_ne!(acc.load(Ordering::SeqCst), 0);


    assert_eq!(s.exit_count.load(Ordering::SeqCst), 0);


    assert_eq!(s.panic_count.load(Ordering::SeqCst), after);
}

#[test]
fn config_start_handler_does_not_refire() {
    ensure_init();
    let s = shared();
    let n = current_num_threads();

    let before = s.start_count.load(Ordering::SeqCst);
    assert!(before >= n);


    let counter = Arc::new(AtomicUsize::new(0));
    let c_in = Arc::clone(&counter);
    scope(|sc| {
        for _ in 0..16usize {
            let c = Arc::clone(&c_in);
            sc.spawn(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(counter.load(Ordering::SeqCst), 16);
    assert_ne!(counter.load(Ordering::SeqCst), 0);

    let after = s.start_count.load(Ordering::SeqCst);
    assert_eq!(after, before);
    assert_eq!(s.exit_count.load(Ordering::SeqCst), 0);
}