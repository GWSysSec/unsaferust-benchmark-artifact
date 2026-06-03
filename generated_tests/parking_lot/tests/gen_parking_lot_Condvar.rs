use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn test_wait_while_basic() {
    let pair = Arc::new((Mutex::new(0u32), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    let initial = *pair.0.lock();
    assert_eq!(initial, 0);

    let handle = thread::spawn(move || {
        let (lock, cvar) = &*pair2;
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(5));
            let mut g = lock.lock();
            *g += 1;
            cvar.notify_one();
        }
    });

    let (lock, cvar) = &*pair;
    let mut guard = lock.lock();
    assert_eq!(*guard, 0);
    cvar.wait_while(&mut guard, |v| *v < 5);
    assert_eq!(*guard, 5);
    assert!(*guard >= 5);
    assert_ne!(*guard, 0);
    drop(guard);

    handle.join().unwrap();

    let final_val = *pair.0.lock();
    assert_eq!(final_val, 5);
}

#[test]
fn test_wait_while_condition_already_false() {

    let m = Mutex::new(42i32);
    let cv = Condvar::new();

    let pre = *m.lock();
    assert_eq!(pre, 42);

    let start = Instant::now();
    let mut guard = m.lock();
    assert_eq!(*guard, 42);
    cv.wait_while(&mut guard, |v| *v < 0);
    let elapsed = start.elapsed();

    assert_eq!(*guard, 42);
    assert!(elapsed < Duration::from_millis(100));
    assert_ne!(*guard, 0);
    assert!(*guard > 0);
    drop(guard);
    assert_eq!(*m.lock(), 42);
}

#[test]
fn test_wait_while_for_timeout() {
    let m = Mutex::new(0u32);
    let cv = Condvar::new();

    assert_eq!(*m.lock(), 0);

    let start = Instant::now();
    let mut guard = m.lock();
    let result = cv.wait_while_for(&mut guard, |v| *v == 0, Duration::from_millis(50));
    let elapsed = start.elapsed();

    assert!(result.timed_out());
    assert_eq!(result.timed_out(), true);
    assert_eq!(*guard, 0);
    assert!(elapsed >= Duration::from_millis(40));
    assert!(elapsed < Duration::from_millis(500));
    drop(guard);
    assert_eq!(*m.lock(), 0);
}

#[test]
fn test_wait_while_for_notified_before_timeout() {
    let pair = Arc::new((Mutex::new(0u32), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    assert_eq!(*pair.0.lock(), 0);

    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        let (lock, cvar) = &*pair2;
        let mut g = lock.lock();
        *g = 99;
        cvar.notify_one();
    });

    let (lock, cvar) = &*pair;
    let mut guard = lock.lock();
    assert_eq!(*guard, 0);
    let result = cvar.wait_while_for(&mut guard, |v| *v == 0, Duration::from_secs(5));

    assert_eq!(result.timed_out(), false);
    assert!(!result.timed_out());
    assert_eq!(*guard, 99);
    assert_ne!(*guard, 0);
    drop(guard);

    handle.join().unwrap();
    assert_eq!(*pair.0.lock(), 99);
}

#[test]
fn test_wait_while_until_timeout() {
    let m = Mutex::new(7u32);
    let cv = Condvar::new();

    assert_eq!(*m.lock(), 7);

    let deadline = Instant::now() + Duration::from_millis(50);
    let mut guard = m.lock();
    let result = cv.wait_while_until(&mut guard, |v| *v == 7, deadline);
    let now = Instant::now();

    assert!(result.timed_out());
    assert_eq!(result.timed_out(), true);
    assert_eq!(*guard, 7);
    assert!(now >= deadline);
    assert_ne!(*guard, 0);
    drop(guard);
    assert_eq!(*m.lock(), 7);
}

#[test]
fn test_wait_while_until_notified() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    assert_eq!(*pair.0.lock(), false);

    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        let (lock, cvar) = &*pair2;
        let mut g = lock.lock();
        *g = true;
        cvar.notify_all();
    });

    let (lock, cvar) = &*pair;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut guard = lock.lock();
    assert_eq!(*guard, false);
    let result = cvar.wait_while_until(&mut guard, |v| !*v, deadline);

    assert!(!result.timed_out());
    assert_eq!(result.timed_out(), false);
    assert_eq!(*guard, true);
    assert_ne!(*guard, false);
    drop(guard);

    handle.join().unwrap();
    assert_eq!(*pair.0.lock(), true);
}

#[test]
fn test_wait_while_multi_step_workflow() {
    let pair = Arc::new((Mutex::new(Vec::<i32>::new()), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    assert_eq!(pair.0.lock().len(), 0);
    assert!(pair.0.lock().is_empty());

    let producer = thread::spawn(move || {
        let (lock, cvar) = &*pair2;
        for i in 1..=4 {
            thread::sleep(Duration::from_millis(5));
            let mut g = lock.lock();
            g.push(i);
            cvar.notify_one();
        }
    });

    let (lock, cvar) = &*pair;


    let mut guard = lock.lock();
    cvar.wait_while(&mut guard, |v| v.len() < 2);
    assert!(guard.len() >= 2);
    assert_ne!(guard.len(), 0);
    let snapshot_len = guard.len();
    drop(guard);


    let mut guard = lock.lock();
    let res = cvar.wait_while_for(&mut guard, |v| v.len() < 4, Duration::from_secs(5));
    assert_eq!(res.timed_out(), false);
    assert_eq!(guard.len(), 4);
    assert_eq!(*guard, vec![1, 2, 3, 4]);
    assert!(snapshot_len <= 4);
    drop(guard);

    producer.join().unwrap();
    assert_eq!(pair.0.lock().len(), 4);
}