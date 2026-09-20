use async_lock::{Semaphore, SemaphoreGuard};
use futures_lite::future;
use std::sync::Arc;

#[test]
fn forget_reduces_permits_permanently() {
    future::block_on(async {
        let sem = Semaphore::new(3);

        // Pre-state: 3 permits available
        let g1 = sem.try_acquire().unwrap();
        let g2 = sem.try_acquire().unwrap();
        let g3 = sem.try_acquire().unwrap();

        // No more permits available now
        assert!(sem.try_acquire().is_none());

        // Drop one normally — permit should be returned
        drop(g1);
        let g1b = sem.try_acquire();
        assert!(g1b.is_some());
        let g1b = g1b.unwrap();

        // Forget another guard — permit should NOT be returned
        SemaphoreGuard::forget(g2);

        // After forget, no permit returned, so try_acquire should fail
        assert!(sem.try_acquire().is_none());

        // Drop g3 normally, that permit comes back
        drop(g3);
        let g3b = sem.try_acquire();
        assert!(g3b.is_some());

        // Now total available should be 0
        assert!(sem.try_acquire().is_none());

        // Cleanup
        drop(g1b);
        drop(g3b);

        // Only 2 permits should ever be retrievable now (1 was forgotten)
        let a = sem.try_acquire().unwrap();
        let b = sem.try_acquire().unwrap();
        assert!(sem.try_acquire().is_none());
        drop(a);
        drop(b);
    });
}

#[test]
fn forget_multiple_exhausts_semaphore() {
    future::block_on(async {
        let sem = Semaphore::new(5);

        let mut count = 0;
        for _ in 0..5 {
            let g = sem.try_acquire().unwrap();
            SemaphoreGuard::forget(g);
            count += 1;
        }
        assert_eq!(count, 5);

        // All permits forgotten — semaphore should be permanently exhausted
        assert!(sem.try_acquire().is_none());
        assert!(sem.try_acquire().is_none());

        // Even after acquire().await would block — verify with try
        for _ in 0..10 {
            assert!(sem.try_acquire().is_none());
        }

        // Add a new permit by adding capacity (not available — verify exhaustion)
        let still_none = sem.try_acquire().is_none();
        assert_eq!(still_none, true);
        assert_ne!(still_none, false);
    });
}

#[test]
fn forget_then_acquire_async_with_release() {
    future::block_on(async {
        let sem = Arc::new(Semaphore::new(2));

        let g1 = sem.try_acquire().unwrap();
        let g2 = sem.try_acquire().unwrap();
        assert!(sem.try_acquire().is_none());

        // Forget g1 — permit lost
        SemaphoreGuard::forget(g1);
        assert!(sem.try_acquire().is_none());

        // Drop g2 — permit returned
        drop(g2);

        // Now exactly one permit available
        let g3 = sem.acquire().await;
        assert!(sem.try_acquire().is_none());

        // Drop g3, retrieve again
        drop(g3);
        let g4 = sem.try_acquire();
        assert!(g4.is_some());
        assert!(sem.try_acquire().is_none());

        // Total capacity is now 1 (one was forgotten)
        drop(g4);
        let a = sem.try_acquire().unwrap();
        assert!(sem.try_acquire().is_none());
        drop(a);
    });
}