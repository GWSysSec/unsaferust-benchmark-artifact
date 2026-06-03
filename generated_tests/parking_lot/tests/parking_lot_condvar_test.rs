use parking_lot::{Condvar, Mutex};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct SharedState {
    ready: bool,
    value: usize,
    predicate_checks: usize,
    events: Vec<&'static str>,
}

#[test]
fn wait_while_blocks_until_predicate_becomes_false_and_preserves_state_changes() {
    let shared = Arc::new((
        Mutex::new(SharedState {
            ready: false,
            value: 0,
            predicate_checks: 0,
            events: vec!["created"],
        }),
        Condvar::new(),
    ));

    let (predicate_entered_tx, predicate_entered_rx) = mpsc::channel();
    let waiter_shared = Arc::clone(&shared);

    let waiter = thread::spawn(move || {
        let (mutex, condvar) = &*waiter_shared;
        let mut guard = mutex.lock();
        let mut announced = Some(predicate_entered_tx);

        condvar.wait_while(&mut guard, |state| {
            state.predicate_checks += 1;
            state.events.push("predicate_checked");
            if let Some(tx) = announced.take() {
                tx.send(()).expect("test should receive first predicate check");
            }
            !state.ready
        });

        guard.events.push("waiter_resumed");
        guard.value += 10;

        (
            guard.ready,
            guard.value,
            guard.predicate_checks,
            guard.events.clone(),
        )
    });

    predicate_entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("waiter should evaluate predicate before blocking");

    {
        let (mutex, condvar) = &*shared;
        let mut guard = mutex.lock();
        assert!(!guard.ready);
        assert_eq!(guard.value, 0);

        guard.ready = true;
        guard.value = 32;
        guard.events.push("notifier_updated");

        condvar.notify_one();
    }

    let (ready, value, predicate_checks, events) =
        waiter.join().expect("waiter thread should not panic");

    assert!(ready);
    assert_eq!(value, 42);
    assert!(predicate_checks >= 2);
    assert!(events.contains(&"created"));
    assert!(events.contains(&"notifier_updated"));
    assert!(events.contains(&"waiter_resumed"));
}

#[test]
fn wait_while_for_ignores_notification_while_predicate_remains_true_then_completes() {
    let shared = Arc::new((
        Mutex::new(SharedState {
            ready: false,
            value: 0,
            predicate_checks: 0,
            events: Vec::new(),
        }),
        Condvar::new(),
    ));

    let (first_check_tx, first_check_rx) = mpsc::channel();
    let (spurious_seen_tx, spurious_seen_rx) = mpsc::channel();
    let waiter_shared = Arc::clone(&shared);

    let waiter = thread::spawn(move || {
        let (mutex, condvar) = &*waiter_shared;
        let mut guard = mutex.lock();
        let mut announced_first = Some(first_check_tx);
        let mut announced_spurious = Some(spurious_seen_tx);

        let result = condvar.wait_while_for(
            &mut guard,
            |state| {
                state.predicate_checks += 1;

                if let Some(tx) = announced_first.take() {
                    tx.send(()).expect("test should receive first predicate check");
                }

                if state.predicate_checks >= 2 && !state.ready {
                    if let Some(tx) = announced_spurious.take() {
                        tx.send(())
                            .expect("test should receive predicate check after spurious notify");
                    }
                }

                !state.ready
            },
            Duration::from_secs(2),
        );

        guard.events.push("completed");
        (
            result.timed_out(),
            guard.ready,
            guard.value,
            guard.predicate_checks,
        )
    });

    first_check_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("waiter should enter wait_while_for predicate");

    {
        let (mutex, condvar) = &*shared;

        let guard = mutex.lock();
        assert!(!guard.ready);
        assert_eq!(guard.value, 0);

        condvar.notify_one();
        drop(guard);
    }

    spurious_seen_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("waiter should re-check predicate after notification");

    {
        let (mutex, condvar) = &*shared;
        let mut guard = mutex.lock();
        assert!(!guard.ready);
        assert_eq!(guard.value, 0);

        guard.ready = true;
        guard.value = 7;
        condvar.notify_one();
    }

    let (timed_out, ready, value, predicate_checks) =
        waiter.join().expect("waiter thread should not panic");

    assert!(!timed_out);
    assert!(ready);
    assert_eq!(value, 7);
    assert!(predicate_checks >= 3);
}

#[test]
fn wait_while_for_reports_timeout_and_returns_with_guard_reacquired() {
    let mutex = Mutex::new(SharedState {
        ready: false,
        value: 5,
        predicate_checks: 0,
        events: vec!["before_wait"],
    });
    let condvar = Condvar::new();

    let mut guard = mutex.lock();
    let result = condvar.wait_while_for(
        &mut guard,
        |state| {
            state.predicate_checks += 1;
            state.events.push("timeout_predicate_checked");
            !state.ready
        },
        Duration::from_millis(30),
    );

    assert!(result.timed_out());
    assert!(!guard.ready);
    assert_eq!(guard.value, 5);
    assert!(guard.predicate_checks >= 1);

    guard.value += 1;
    guard.events.push("after_timeout_mutation");

    assert_eq!(guard.value, 6);
    assert!(guard.events.contains(&"before_wait"));
    assert!(guard.events.contains(&"after_timeout_mutation"));
}

#[test]
fn wait_while_until_times_out_at_deadline_when_predicate_stays_true() {
    let mutex = Mutex::new(SharedState {
        ready: false,
        value: 11,
        predicate_checks: 0,
        events: Vec::new(),
    });
    let condvar = Condvar::new();

    let started = Instant::now();
    let deadline = started + Duration::from_millis(30);

    let mut guard = mutex.lock();
    let result = condvar.wait_while_until(
        &mut guard,
        |state| {
            state.predicate_checks += 1;
            state.events.push("deadline_predicate_checked");
            !state.ready
        },
        deadline,
    );

    assert!(result.timed_out());
    assert!(Instant::now() >= deadline);
    assert!(!guard.ready);
    assert_eq!(guard.value, 11);
    assert!(guard.predicate_checks >= 1);
    assert!(guard.events.contains(&"deadline_predicate_checked"));
}

#[test]
fn wait_while_until_returns_before_deadline_after_state_update() {
    let shared = Arc::new((
        Mutex::new(SharedState {
            ready: false,
            value: 1,
            predicate_checks: 0,
            events: Vec::new(),
        }),
        Condvar::new(),
    ));

    let (entered_tx, entered_rx) = mpsc::channel();
    let waiter_shared = Arc::clone(&shared);

    let waiter = thread::spawn(move || {
        let (mutex, condvar) = &*waiter_shared;
        let mut guard = mutex.lock();
        let mut announced = Some(entered_tx);
        let deadline = Instant::now() + Duration::from_secs(2);

        let result = condvar.wait_while_until(
            &mut guard,
            |state| {
                state.predicate_checks += 1;
                if let Some(tx) = announced.take() {
                    tx.send(()).expect("test should receive predicate entry");
                }
                state.value < 100
            },
            deadline,
        );

        (result.timed_out(), guard.value, guard.predicate_checks)
    });

    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("waiter should evaluate wait_while_until predicate");

    {
        let (mutex, condvar) = &*shared;
        let mut guard = mutex.lock();
        assert_eq!(guard.value, 1);
        guard.value = 100;
        condvar.notify_one();
    }

    let (timed_out, value, predicate_checks) =
        waiter.join().expect("waiter thread should not panic");

    assert!(!timed_out);
    assert_eq!(value, 100);
    assert!(predicate_checks >= 2);
}