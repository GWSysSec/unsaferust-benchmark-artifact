use rayon_core::ThreadPoolBuilder;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn poll_until<F: Fn() -> bool>(cond: F, max_ms: u64) -> bool {
    let start = Instant::now();
    loop {
        if cond() {
            return true;
        }
        if start.elapsed() > Duration::from_millis(max_ms) {
            return cond();
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn test_builder_thread_name_start_exit_handlers() {
    let start_count = Arc::new(AtomicUsize::new(0));
    let exit_count = Arc::new(AtomicUsize::new(0));
    let names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let start_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let exit_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));


    assert_eq!(start_count.load(Ordering::SeqCst), 0);
    assert_eq!(exit_count.load(Ordering::SeqCst), 0);
    assert_eq!(names.lock().unwrap().len(), 0);
    assert_eq!(start_indices.lock().unwrap().len(), 0);
    assert_eq!(exit_indices.lock().unwrap().len(), 0);

    let n_workers: usize = 3;
    let sc = Arc::clone(&start_count);
    let si = Arc::clone(&start_indices);
    let ec = Arc::clone(&exit_count);
    let ei = Arc::clone(&exit_indices);

    {
        let pool = ThreadPoolBuilder::new()
            .num_threads(n_workers)
            .thread_name(|i| format!("tpb-name-{}", i))
            .start_handler(move |idx| {
                sc.fetch_add(1, Ordering::SeqCst);
                si.lock().unwrap().push(idx);
            })
            .exit_handler(move |idx| {
                ec.fetch_add(1, Ordering::SeqCst);
                ei.lock().unwrap().push(idx);
            })
            .build()
            .expect("pool build should succeed");


        let names_in = Arc::clone(&names);
        let n_tasks = n_workers * 4;
        let acc = Arc::new(AtomicUsize::new(0));
        let acc_outer = Arc::clone(&acc);
        let returned = pool.install(move || {
            let acc_in = Arc::clone(&acc_outer);
            rayon_core::scope(|s| {
                for i in 0..n_tasks {
                    let acc_t = Arc::clone(&acc_in);
                    let nm_t = Arc::clone(&names_in);
                    s.spawn(move |_| {
                        let nm = std::thread::current()
                            .name()
                            .unwrap_or("<none>")
                            .to_string();
                        nm_t.lock().unwrap().push(nm);
                        acc_t.fetch_add(i, Ordering::SeqCst);
                    });
                }
            });
            acc_in.load(Ordering::SeqCst)
        });
        let expected_sum: usize = (0..n_tasks).sum();
        assert_eq!(returned, expected_sum, "scope must complete all tasks");
        assert_eq!(acc.load(Ordering::SeqCst), expected_sum);


        let started = poll_until(
            || start_count.load(Ordering::SeqCst) >= n_workers,
            5000,
        );
        assert!(started, "all start handlers must fire");
        assert_eq!(start_count.load(Ordering::SeqCst), n_workers);

        let mut sorted = start_indices.lock().unwrap().clone();
        sorted.sort();
        let expected: Vec<usize> = (0..n_workers).collect();
        assert_eq!(sorted, expected, "start indices should be 0..n exactly once");


        let recorded = names.lock().unwrap().clone();
        assert_eq!(recorded.len(), n_tasks, "one name recorded per task");
        for nm in &recorded {
            assert!(nm.starts_with("tpb-name-"), "unexpected thread name: {}", nm);
            let suffix = &nm["tpb-name-".len()..];
            let parsed: usize = suffix.parse().expect("numeric suffix");
            assert!(parsed < n_workers, "index {} out of range", parsed);
        }
        let mut uniq = recorded.clone();
        uniq.sort();
        uniq.dedup();
        assert!(uniq.len() <= n_workers, "distinct names <= worker count");
        assert!(!uniq.is_empty(), "at least one distinct name observed");


        assert_eq!(exit_count.load(Ordering::SeqCst), 0);
        assert_eq!(exit_indices.lock().unwrap().len(), 0);
    }


    let exited = poll_until(
        || exit_count.load(Ordering::SeqCst) >= n_workers,
        5000,
    );
    assert!(exited, "all exit handlers should fire after pool drops");
    assert_eq!(exit_count.load(Ordering::SeqCst), n_workers);

    let mut sorted_exit = exit_indices.lock().unwrap().clone();
    sorted_exit.sort();
    assert_eq!(sorted_exit, (0..n_workers).collect::<Vec<_>>());
}

#[test]
fn test_builder_panic_handler_isolates_panics() {
    let panic_count = Arc::new(AtomicUsize::new(0));
    let panic_msgs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    assert_eq!(panic_count.load(Ordering::SeqCst), 0);
    assert_eq!(panic_msgs.lock().unwrap().len(), 0);

    let pc = Arc::clone(&panic_count);
    let pm = Arc::clone(&panic_msgs);

    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .panic_handler(move |payload| {
            pc.fetch_add(1, Ordering::SeqCst);
            let msg = if let Some(s) = payload.downcast_ref::<&'static str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "<non-string>".to_string()
            };
            pm.lock().unwrap().push(msg);
        })
        .build()
        .expect("pool with panic handler builds");


    let n_panics: usize = 4;
    pool.install(|| {
        for i in 0..n_panics {
            if i % 2 == 0 {
                rayon_core::spawn(|| panic!("intentional-static"));
            } else {
                rayon_core::spawn(|| panic!("{}", String::from("intentional-string")));
            }
        }
    });

    let ok = poll_until(
        || panic_count.load(Ordering::SeqCst) >= n_panics,
        5000,
    );
    assert!(ok, "panic_handler should fire for every panicking task");
    assert_eq!(panic_count.load(Ordering::SeqCst), n_panics);

    let captured = panic_msgs.lock().unwrap().clone();
    assert_eq!(captured.len(), n_panics);
    let static_count = captured.iter().filter(|m| m.contains("intentional-static")).count();
    let string_count = captured.iter().filter(|m| m.contains("intentional-string")).count();
    assert_eq!(static_count, 2, "two &'static str panics");
    assert_eq!(string_count, 2, "two String panics");


    let r = pool.install(|| 42usize);
    assert_eq!(r, 42);

    let acc = Arc::new(AtomicUsize::new(0));
    let acc_in = Arc::clone(&acc);
    pool.install(move || {
        let inner = Arc::clone(&acc_in);
        rayon_core::scope(|s| {
            for i in 0..4 {
                let a = Arc::clone(&inner);
                s.spawn(move |_| {
                    a.fetch_add(i, Ordering::SeqCst);
                });
            }
        });
    });
    assert_eq!(acc.load(Ordering::SeqCst), 0 + 1 + 2 + 3);
}