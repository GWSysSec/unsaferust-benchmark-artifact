use async_lock::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};
use futures_lite::future;

#[test]
fn downgrade_write_to_read() {
    future::block_on(async {
        let lock = RwLock::new(100i32);


        let mut w = lock.write().await;
        assert_eq!(*w, 100);
        *w = 200;
        assert_eq!(*w, 200);

        let r = RwLockWriteGuard::downgrade(w);
        assert_eq!(*r, 200);


        let r2 = lock.read().await;
        assert_eq!(*r2, 200);
        assert_eq!(*r, 200);


        assert!(lock.try_write().is_none());

        drop(r);
        drop(r2);


        let w2 = lock.try_write();
        assert!(w2.is_some());
        let w2 = w2.unwrap();
        assert_eq!(*w2, 200);
        drop(w2);

        assert_eq!(lock.into_inner(), 200);
    });
}

#[test]
fn downgrade_write_to_upgradable() {
    future::block_on(async {
        let lock = RwLock::new(5u64);

        let mut w = lock.write().await;
        assert_eq!(*w, 5);
        *w = 42;
        assert_eq!(*w, 42);

        let upg = RwLockWriteGuard::downgrade_to_upgradable(w);
        assert_eq!(*upg, 42);


        let r = lock.read().await;
        assert_eq!(*r, 42);


        assert!(lock.try_upgradable_read().is_none());

        assert!(lock.try_write().is_none());

        drop(r);


        let mut w2 = RwLockUpgradableReadGuard::upgrade(upg).await;
        assert_eq!(*w2, 42);
        *w2 = 7;
        assert_eq!(*w2, 7);
        drop(w2);

        let final_r = lock.read().await;
        assert_eq!(*final_r, 7);
    });
}

#[test]
fn downgrade_chain_multiple_readers() {
    future::block_on(async {
        let lock = RwLock::new(vec![1, 2, 3]);

        let mut w = lock.write().await;
        w.push(4);
        assert_eq!(w.len(), 4);

        let r1 = RwLockWriteGuard::downgrade(w);
        assert_eq!(r1.len(), 4);
        assert_eq!(r1[3], 4);

        let r2 = lock.read().await;
        let r3 = lock.read().await;
        assert_eq!(r2.len(), 4);
        assert_eq!(r3[0], 1);
        assert_eq!(r2[2], 3);

        assert!(lock.try_write().is_none());

        drop(r1);
        drop(r2);
        drop(r3);

        let w_again = lock.try_write();
        assert!(w_again.is_some());
    });
}