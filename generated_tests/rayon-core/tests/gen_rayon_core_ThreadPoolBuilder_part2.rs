use rayon_core::{BroadcastContext, ThreadPoolBuilder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn wait_until<F: Fn() -> bool>(cond: F, timeout: Duration) -> bool {
    let start = Instant::now();
    while !cond() {
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

#[test]
fn builder_thread_name_and_start_handler() {
    let n_workers: usize = 3;
    let names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let start_log: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let start_count = Arc::new(AtomicUsize::new(0));

    assert_eq!(names.lock().unwrap().len(), 0);
    assert_eq!(start_log.lock().unwrap().len(), 0);
    assert_eq!(start_count.load(Ordering::SeqCst), 0);

    let sc = Arc::clone(&start_count);
    let sl = Arc::clone(&start_log);

    let pool = ThreadPoolBuilder::new()
        .num_threads(n_workers)
        .thread_name(|i| format!("tpb-name-{}", i))
        .start_handler(move |idx| {
            sc.fetch_add(1, Ordering::SeqCst);
            sl.lock().unwrap().push(idx);
        })
        .build()
        .expect("pool builds");

    let sc_check = Arc::clone(&start_count);
    let started = wait_until(
        move || sc_check.load(Ordering::SeqCst) >= n_workers,
        Duration::from_secs(5),
    );
    assert!(started, "all workers should reach start_handler");
    assert_eq!(start_count.load(Ordering::SeqCst), n_workers);

    let mut idx_seen = start_log.lock().unwrap().clone();
    idx_seen.sort();
    let expected: Vec<usize> = (0..n_workers).collect();
    assert_eq!(idx_seen, expected, "start_handler indices cover 0..n");

    let nm = Arc::clone(&names);
    let collected: Vec<usize> = pool.install(move || {
        rayon_core::broadcast(move |ctx: BroadcastContext<'_>| {
            let name = std::thread::current().name().unwrap_or("?").to_string();
            nm.lock().unwrap().push(name);
            ctx.index()
        })
    });

    assert_eq!(collected.len(), n_workers, "broadcast hits each worker");
    let mut sorted = collected.clone();
    sorted.sort();
    assert_eq!(sorted, expected, "indices cover 0..n via broadcast");

    let names_seen = names.lock().unwrap().clone();
    assert_eq!(names_seen.len(), n_workers, "one name per worker");
    for nm in &names_seen {
        assert!(
            nm.starts_with("tpb-name-"),
            "name '{}' missing expected prefix",
            nm
        );
    }
    let mut uniq = names_seen.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), n_workers, "names are unique");
    assert_eq!(
        start_count.load(Ordering::SeqCst),
        n_workers,
        "no extra start_handler invocations"
    );
}

#[test]
fn builder_spawn_handler_uses_custom_threads() {
    let n: usize = 2;
    let spawn_calls = Arc::new(AtomicUsize::new(0));
    let sc = Arc::clone(&spawn_calls);

    assert_eq!(spawn_calls.load(Ordering::SeqCst), 0);

    let pool = ThreadPoolBuilder::new()
        .num_threads(n)
        .spawn_handler(move |thread| {
            sc.fetch_add(1, Ordering::SeqCst);
            std::thread::Builder::new()
                .name(format!("custom-spawn-{}", thread.index()))
                .spawn(move || thread.run())?;
            Ok(())
        })
        .build()
        .expect("custom spawner builds");

    let sc_check = Arc::clone(&spawn_calls);
    let ok = wait_until(
        move || sc_check.load(Ordering::SeqCst) >= n,
        Duration::from_secs(5),
    );
    assert!(ok, "spawn_handler must be invoked for each worker");
    assert_eq!(
        spawn_calls.load(Ordering::SeqCst),
        n,
        "exactly n spawn_handler invocations"
    );

    let names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let n_clone = Arc::clone(&names);
    let collected: Vec<usize> = pool.install(move || {
        rayon_core::broadcast(move |ctx: BroadcastContext<'_>| {
            let nm = std::thread::current().name().unwrap_or("?").to_string();
            n_clone.lock().unwrap().push(nm);
            ctx.index()
        })
    });

    assert_eq!(collected.len(), n, "broadcast covers every custom-spawned worker");
    let mut sorted = collected.clone();
    sorted.sort();
    let expected: Vec<usize> = (0..n).collect();
    assert_eq!(sorted, expected);

    let observed = names.lock().unwrap().clone();
    assert_eq!(observed.len(), n, "one observation per worker");
    for nm in &observed {
        assert!(
            nm.starts_with("custom-spawn-"),
            "thread name '{}' must reflect custom spawner",
            nm
        );
    }
    let mut uniq = observed.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), n, "custom spawner produced unique names");
}

#[test]
fn builder_panic_handler_routes_panics() {
    let panics = Arc::new(AtomicUsize::new(0));
    let p = Arc::clone(&panics);

    assert_eq!(panics.load(Ordering::SeqCst), 0, "no panics yet");

    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .panic_handler(move |_payload| {
            p.fetch_add(1, Ordering::SeqCst);
        })
        .build()
        .expect("pool builds");

    assert_eq!(panics.load(Ordering::SeqCst), 0, "still none after build");

    pool.install(|| {
        rayon_core::spawn(|| {
            panic!("intentional panic in spawn");
        });
    });

    let p_check = Arc::clone(&panics);
    let fired = wait_until(
        move || p_check.load(Ordering::SeqCst) >= 1,
        Duration::from_secs(5),
    );
    assert!(fired, "panic_handler must fire for first panic");
    assert_eq!(panics.load(Ordering::SeqCst), 1, "one panic observed");

    pool.install(|| {
        rayon_core::spawn(|| {
            panic!("second intentional panic");
        });
    });

    let p_check2 = Arc::clone(&panics);
    let fired2 = wait_until(
        move || p_check2.load(Ordering::SeqCst) >= 2,
        Duration::from_secs(5),
    );
    assert!(fired2, "panic_handler must fire again for second panic");
    assert_eq!(panics.load(Ordering::SeqCst), 2, "two panics observed");

    let counter = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&counter);
    pool.install(move || {
        rayon_core::scope(|s| {
            for i in 1..=10usize {
                let cc = Arc::clone(&c);
                s.spawn(move |_| {
                    cc.fetch_add(i, Ordering::SeqCst);
                });
            }
        });
    });
    let expected: usize = (1..=10).sum();
    assert_eq!(expected, 55, "sanity check on expected sum");
    assert_eq!(
        counter.load(Ordering::SeqCst),
        expected,
        "pool stays usable after panic_handler fires"
    );
    assert_eq!(
        panics.load(Ordering::SeqCst),
        2,
        "no spurious panic_handler invocations from non-panicking work"
    );
}

#[test]
fn builder_exit_handler_fires_on_drop() {
    let exits = Arc::new(AtomicUsize::new(0));
    let exit_log: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let n: usize = 2;

    assert_eq!(exits.load(Ordering::SeqCst), 0);
    assert_eq!(exit_log.lock().unwrap().len(), 0);

    let e = Arc::clone(&exits);
    let el = Arc::clone(&exit_log);

    let touched = Arc::new(AtomicUsize::new(0));
    {
        let pool = ThreadPoolBuilder::new()
            .num_threads(n)
            .exit_handler(move |idx| {
                e.fetch_add(1, Ordering::SeqCst);
                el.lock().unwrap().push(idx);
            })
            .build()
            .expect("pool builds");

        let t = Arc::clone(&touched);
        let res: Vec<usize> = pool.install(move || {
            rayon_core::broadcast(move |ctx: BroadcastContext<'_>| {
                t.fetch_add(1, Ordering::SeqCst);
                ctx.index()
            })
        });
        assert_eq!(res.len(), n, "broadcast yielded one per worker");
        assert_eq!(touched.load(Ordering::SeqCst), n, "all workers ran");
        assert_eq!(
            exits.load(Ordering::SeqCst),
            0,
            "no exit_handler invocations while pool alive"
        );
    }

    let e_check = Arc::clone(&exits);
    let exited = wait_until(
        move || e_check.load(Ordering::SeqCst) >= n,
        Duration::from_secs(10),
    );
    assert!(exited, "exit_handler must fire for each worker on drop");
    assert_eq!(
        exits.load(Ordering::SeqCst),
        n,
        "exactly n exit_handler invocations"
    );

    let mut log = exit_log.lock().unwrap().clone();
    log.sort();
    let expected: Vec<usize> = (0..n).collect();
    assert_eq!(log, expected, "exit indices match worker indices");
    assert_eq!(
        touched.load(Ordering::SeqCst),
        n,
        "touched count unchanged after drop"
    );
}

#[test]
fn builder_build_scoped_borrows_stack_data() {
    let stack_data: Vec<i32> = vec![10, 20, 30, 40, 50];
    let expected_local: i32 = stack_data.iter().sum();
    assert_eq!(expected_local, 150);
    assert_eq!(stack_data.len(), 5);

    let n: usize = 2;
    let runs = Arc::new(AtomicUsize::new(0));
    let r = Arc::clone(&runs);

    let sd_ref: &Vec<i32> = &stack_data;

    let result: Result<i32, _> = ThreadPoolBuilder::new()
        .num_threads(n)
        .build_scoped(
            |thread| thread.run(),
            move |pool| {
                pool.install(|| {
                    let parts: Vec<i32> = rayon_core::broadcast(|_ctx: BroadcastContext<'_>| {
                        r.fetch_add(1, Ordering::SeqCst);
                        sd_ref.iter().sum::<i32>()
                    });
                    parts.iter().sum::<i32>()
                })
            },
        );

    let total = result.expect("build_scoped must succeed");
    assert_eq!(
        total,
        expected_local * (n as i32),
        "sum across n workers equals per-worker sum * n"
    );
    assert_eq!(
        runs.load(Ordering::SeqCst),
        n,
        "broadcast invoked once per worker"
    );



    assert_eq!(stack_data.len(), 5, "stack_data still owned");
    assert_eq!(stack_data[0], 10);
    assert_eq!(stack_data[1], 20);
    assert_eq!(stack_data[4], 50);
    assert_eq!(stack_data.iter().sum::<i32>(), 150);
}