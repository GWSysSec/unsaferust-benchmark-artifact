use parking_lot::{const_fair_mutex, const_mutex, const_reentrant_mutex, const_rwlock};
use parking_lot::{FairMutex, Mutex, ReentrantMutex, RwLock};
use std::sync::Arc;
use std::thread;

static GLOBAL_MUTEX: Mutex<i64> = const_mutex(0);
static GLOBAL_FAIR: FairMutex<u32> = const_fair_mutex(7);
static GLOBAL_REENTRANT: ReentrantMutex<i32> = const_reentrant_mutex(100);
static GLOBAL_RWLOCK: RwLock<u64> = const_rwlock(42);

#[test]
fn test_const_mutex_static_and_threads() {
    {
        let g = GLOBAL_MUTEX.lock();
        assert_eq!(*g, 0);
    }
    assert!(GLOBAL_MUTEX.try_lock().is_some());

    let mut handles = Vec::new();
    for _ in 0..4 {
        handles.push(thread::spawn(|| {
            for _ in 0..100 {
                let mut g = GLOBAL_MUTEX.lock();
                *g += 1;
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let final_val = *GLOBAL_MUTEX.lock();
    assert_eq!(final_val, 400);


    {
        let mut g = GLOBAL_MUTEX.lock();
        assert_eq!(*g, 400);
        *g = -1;
    }
    assert_eq!(*GLOBAL_MUTEX.lock(), -1);

    let local = const_mutex(String::from("hello"));
    {
        let mut s = local.lock();
        assert_eq!(s.len(), 5);
        s.push_str(" world");
    }
    assert_eq!(*local.lock(), "hello world");
    assert!(local.try_lock().is_some());
}

#[test]
fn test_const_fair_mutex() {
    {
        let g = GLOBAL_FAIR.lock();
        assert_eq!(*g, 7);
    }
    {
        let mut g = GLOBAL_FAIR.lock();
        *g = 99;
    }
    assert_eq!(*GLOBAL_FAIR.lock(), 99);

    let local: FairMutex<Vec<i32>> = const_fair_mutex(Vec::new());
    assert!(local.try_lock().unwrap().is_empty());
    {
        let mut v = local.lock();
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(v.len(), 3);
    }
    {
        let v = local.lock();
        assert_eq!(v[0], 1);
        assert_eq!(v[2], 3);
    }

    let arc = Arc::new(const_fair_mutex(0i32));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let a = Arc::clone(&arc);
        handles.push(thread::spawn(move || {
            for _ in 0..50 {
                *a.lock() += 1;
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(*arc.lock(), 200);
}

#[test]
fn test_const_reentrant_mutex() {
    {
        let outer = GLOBAL_REENTRANT.lock();
        assert_eq!(*outer, 100);

        let inner = GLOBAL_REENTRANT.lock();
        assert_eq!(*inner, 100);
        let third = GLOBAL_REENTRANT.lock();
        assert_eq!(*third, 100);
        drop(third);
        drop(inner);
        drop(outer);
    }

    let local = const_reentrant_mutex(5u32);
    let a = local.lock();
    assert_eq!(*a, 5);
    let b = local.lock();
    assert_eq!(*b, 5);
    assert!(local.try_lock().is_some());
    drop(b);
    drop(a);


    let arc = Arc::new(const_reentrant_mutex(0i32));
    let a2 = Arc::clone(&arc);
    let handle = thread::spawn(move || {
        let g1 = a2.lock();
        let g2 = a2.lock();
        assert_eq!(*g1, 0);
        assert_eq!(*g2, 0);
        42
    });
    assert_eq!(handle.join().unwrap(), 42);
    assert_eq!(*arc.lock(), 0);
}

#[test]
fn test_const_rwlock() {
    {
        let r = GLOBAL_RWLOCK.read();
        assert_eq!(*r, 42);
        let r2 = GLOBAL_RWLOCK.read();
        assert_eq!(*r2, 42);
    }
    {
        let mut w = GLOBAL_RWLOCK.write();
        *w = 1000;
    }
    assert_eq!(*GLOBAL_RWLOCK.read(), 1000);

    let local = const_rwlock(Vec::<i32>::new());
    assert!(local.read().is_empty());
    {
        let mut w = local.write();
        for i in 0..10 {
            w.push(i);
        }
        assert_eq!(w.len(), 10);
    }
    assert_eq!(local.read().len(), 10);
    assert_eq!(local.read()[5], 5);


    let arc = Arc::new(const_rwlock(123i64));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let a = Arc::clone(&arc);
        handles.push(thread::spawn(move || {
            let r = a.read();
            assert_eq!(*r, 123);
            *r
        }));
    }
    let mut total = 0i64;
    for h in handles {
        total += h.join().unwrap();
    }
    assert_eq!(total, 492);


    assert!(arc.try_write().is_some());
    let w = arc.write();
    assert!(arc.try_read().is_none());
    assert_eq!(*w, 123);
    drop(w);
    assert!(arc.try_read().is_some());
}