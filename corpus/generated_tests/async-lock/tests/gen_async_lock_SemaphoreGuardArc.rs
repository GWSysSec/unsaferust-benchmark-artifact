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

        // No more available
        assert!(sem.try_acquire_arc().is_none());

        // Drop one normally — permit returned
        drop(g1);
        let g1b = sem.try_acquire_arc();
        assert!(g1b.is_some());
        let g1b = g1b.unwrap();

        // Forget another — permit NOT returned
        SemaphoreGuardArc::forget(g2);
        assert!(sem.try_acquire_arc().is_none());

        // Drop g3 normally
        drop(g3);
        let g3b = sem.try_acquire_arc();
        assert!(g3b.is_some());

        assert!(sem.try_acquire_arc().is_none());

        drop(g1b);
        drop(g3b);

        // Only 2 permits should ever be retrievable now
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

        // All permits gone forever
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

        // Forget g1
        SemaphoreGuardArc::forget(g1);
        assert!(sem.try_acquire_arc().is_none());

        // Drop g2 returns its permit
        drop(g2);

        // Exactly one permit available; acquire_arc().await should succeed immediately
        let g3 = sem.acquire_arc().await;
        assert!(sem.try_acquire_arc().is_none());

        drop(g3);
        let g4 = sem.try_acquire_arc();
        assert!(g4.is_some());
        assert!(sem.try_acquire_arc().is_none());

        drop(g4);

        // Effective capacity is 1 due to forget
        let only = sem.try_acquire_arc().unwrap();
        assert!(sem.try_acquire_arc().is_none());
        drop(only);
    });
}