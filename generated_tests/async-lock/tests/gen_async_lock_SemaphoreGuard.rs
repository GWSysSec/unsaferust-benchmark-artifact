use async_lock::{Semaphore, SemaphoreGuard};
use futures_lite::future;
use std::sync::Arc;

#[test]
fn forget_reduces_permits_permanently() {
    future::block_on(async {
        let sem = Semaphore::new(3);


        let g1 = sem.try_acquire().unwrap();
        let g2 = sem.try_acquire().unwrap();
        let g3 = sem.try_acquire().unwrap();


        assert!(sem.try_acquire().is_none());


        drop(g1);
        let g1b = sem.try_acquire();
        assert!(g1b.is_some());
        let g1b = g1b.unwrap();


        SemaphoreGuard::forget(g2);


        assert!(sem.try_acquire().is_none());


        drop(g3);
        let g3b = sem.try_acquire();
        assert!(g3b.is_some());


        assert!(sem.try_acquire().is_none());


        drop(g1b);
        drop(g3b);


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


        assert!(sem.try_acquire().is_none());
        assert!(sem.try_acquire().is_none());


        for _ in 0..10 {
            assert!(sem.try_acquire().is_none());
        }


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


        SemaphoreGuard::forget(g1);
        assert!(sem.try_acquire().is_none());


        drop(g2);


        let g3 = sem.acquire().await;
        assert!(sem.try_acquire().is_none());


        drop(g3);
        let g4 = sem.try_acquire();
        assert!(g4.is_some());
        assert!(sem.try_acquire().is_none());


        drop(g4);
        let a = sem.try_acquire().unwrap();
        assert!(sem.try_acquire().is_none());
        drop(a);
    });
}