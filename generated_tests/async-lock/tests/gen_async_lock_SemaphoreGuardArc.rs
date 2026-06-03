use async_lock::{Semaphore, SemaphoreGuardArc};
use futures_lite::future;
use std::sync::Arc;

#[test]
fn forget_arc_reduces_permits_permanently() {
    future::block_on(async {
        let sem = Arc::new(Semaphore::new(3));

        let g1 = sem.try_acquire_arc().unwrap();
        let g2 = sem.try_acquire_arc().unwrap();
        let g3 = sem.try_acquire_arc().unwrap();


        assert!(sem.try_acquire_arc().is_none());


        drop(g1);
        let g1b = sem.try_acquire_arc();
        assert!(g1b.is_some());
        let g1b = g1b.unwrap();


        SemaphoreGuardArc::forget(g2);
        assert!(sem.try_acquire_arc().is_none());


        drop(g3);
        let g3b = sem.try_acquire_arc();
        assert!(g3b.is_some());

        assert!(sem.try_acquire_arc().is_none());

        drop(g1b);
        drop(g3b);


        let a = sem.try_acquire_arc().unwrap();
        let b = sem.try_acquire_arc().unwrap();
        assert!(sem.try_acquire_arc().is_none());
        drop(a);
        drop(b);
    });
}

#[test]
fn forget_arc_all_exhausts() {
    future::block_on(async {
        let sem = Arc::new(Semaphore::new(4));

        let mut forgotten = 0;
        for _ in 0..4 {
            let g = sem.try_acquire_arc().unwrap();
            SemaphoreGuardArc::forget(g);
            forgotten += 1;
        }
        assert_eq!(forgotten, 4);


        for _ in 0..8 {
            assert!(sem.try_acquire_arc().is_none());
        }

        let none_state = sem.try_acquire_arc().is_none();
        assert_eq!(none_state, true);
        assert_ne!(none_state, false);
    });
}

#[test]
fn forget_arc_mixed_with_async_acquire() {
    future::block_on(async {
        let sem = Arc::new(Semaphore::new(2));

        let g1 = sem.try_acquire_arc().unwrap();
        let g2 = sem.try_acquire_arc().unwrap();
        assert!(sem.try_acquire_arc().is_none());


        SemaphoreGuardArc::forget(g1);
        assert!(sem.try_acquire_arc().is_none());


        drop(g2);


        let g3 = sem.acquire_arc().await;
        assert!(sem.try_acquire_arc().is_none());

        drop(g3);
        let g4 = sem.try_acquire_arc();
        assert!(g4.is_some());
        assert!(sem.try_acquire_arc().is_none());

        drop(g4);


        let only = sem.try_acquire_arc().unwrap();
        assert!(sem.try_acquire_arc().is_none());
        drop(only);
    });
}