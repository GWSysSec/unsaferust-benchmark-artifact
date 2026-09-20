use parking_lot::{Condvar, Mutex};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct ReadyState {
    ready: bool,
    wait_checks: usize,
    payload: Vec<&'static str>,
}

#[test]
fn wait_while_releases_mutex_waits_and_reacquires_after_notification() {
    let shared = Arc::new((
        Mutex::new(ReadyState {
            ready: false,
            wait_checks: 0,
            payload: vec!["initial"],
        }),
        Condvar::new(),
    ));

    let (entered_wait_tx, entered_wait_rx) = mpsc::channel();
    let waiter_shared = Arc::clone(&shared);

    let waiter = thread::spawn(move || {
        let (mutex, condvar) = &*waiter_shared;
        let mut guard = mutex.lock();
        let mut announced = Some(entered_wait_tx);

        condvar.wait_while(&mut guard, |state| {
            state.wait_checks += 1;
            if let Some(tx) = announced.take() {
                tx.send(()).expect("main thread should receive wait notification");
            }
            !state.ready
        });

        guard.payload.push("waiter observed ready");
        (guard.ready, guard.wait_checks, guard.payload.clone())
    });

    entered_wait_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("waiter should evaluate the wait predicate");

    {
        let (mutex, condvar) = &*shared;
        let mut guard = mutex.lock();
        assert!(!guard.ready);
        assert_eq!(guard.payload, vec!["initial"]);

        guard.payload.push("main prepared state");
        guard.ready = true;
        drop(guard);

        assert!(condvar.notify_one(), "notify_one should wake the waiting thread");
    }

    let (ready, wait_checks, payload) = waiter.join().expect("waiter thread should not panic");

    assert!(ready);
    assert!(
        wait_checks >= 2,
        "predicate should be checked before sleeping and again after notification"
    );
    assert_eq!(
        payload,
        vec!["initial", "main prepared state", "waiter observed ready"]
    );
}

#[test]
fn wait_while_for_times_out_when_condition_remains_true_and_returns_immediately_when_false() {
    let mutex = Mutex::new(ReadyState {
        ready: false,
        wait_checks: 0,
        payload: Vec::new(),
    });
    let condvar = Condvar::new();

    let started = Instant::now();
    let mut guard = mutex.lock();

    let timeout = condvar.wait_while_for(&mut guard, |state| {
        state.wait_checks += 1;
        !state.ready
    }, Duration::from_millis(40));

    assert!(timeout.timed_out());
    assert!(!guard.ready);
    assert!(
        guard.wait_checks >= 1,
        "predicate should be evaluated even when the wait times out"
    );
    assert!(
        started.elapsed() >= Duration::from_millis(20),
        "timeout wait should block for a noticeable amount of time"
    );

    guard.ready = true;
    let checks_before_immediate_wait = guard.wait_checks;

    let immediate = condvar.wait_while_for(&mut guard, |state| {
        state.wait_checks += 1;
        !state.ready
    }, Duration::from_secs(5));

    assert!(!immediate.timed_out());
    assert!(guard.ready);
    assert_eq!(guard.wait_checks, checks_before_immediate_wait + 1);
}

#[test]
fn wait_while_until_wakes_before_deadline_after_state_change() {
    let shared = Arc::new((
        Mutex::new(ReadyState {
            ready: false,
            wait_checks: 0,
            payload: vec!["created"],
        }),
        Condvar::new(),
    ));

    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));

        let (mutex, condvar) = &*notifier_shared;
        let mut guard = mutex.lock();
        guard.ready = true;
        guard.payload.push("notified");
        drop(guard);

        condvar.notify_all()
    });

    let (mutex, condvar) = &*shared;
    let mut guard = mutex.lock();
    let deadline = Instant::now() + Duration::from_secs(2);

    let result = condvar.wait_while_until(&mut guard, |state| {
        state.wait_checks += 1;
        !state.ready
    }, deadline);

    let notified_threads = notifier.join().expect("notifier thread should not panic");

    assert!(!result.timed_out());
    assert!(guard.ready);
    assert_eq!(guard.payload, vec!["created", "notified"]);
    assert!(
        guard.wait_checks >= 2,
        "predicate should be rechecked after notification"
    );
    assert!(
        notified_threads >= 1,
        "notify_all should report at least one awakened waiter"
    );
}

#[test]
fn wait_while_until_reports_timeout_for_unmet_condition_at_deadline() {
    let mutex = Mutex::new(ReadyState {
        ready: false,
        wait_checks: 0,
        payload: vec!["unchanged"],
    });
    let condvar = Condvar::new();

    let mut guard = mutex.lock();
    let result = condvar.wait_while_until(&mut guard, |state| {
        state.wait_checks += 1;
        !state.ready
    }, Instant::now() + Duration::from_millis(30));

    assert!(result.timed_out());
    assert!(!guard.ready);
    assert_eq!(guard.payload, vec!["unchanged"]);
    assert!(guard.wait_checks >= 1);
}