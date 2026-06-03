use async_lock::{RwLock, RwLockUpgradableReadGuard};
use std::sync::Arc;
use std::thread;

#[test]
fn downgrade_upgradable_read_guard_allows_new_upgradable_reader_but_keeps_read_lock() {
    let lock = RwLock::new(vec![1, 2, 3]);

    let upgradable = lock.upgradable_read_blocking();
    assert_eq!(upgradable.as_slice(), &[1, 2, 3]);
    assert!(lock.try_upgradable_read().is_none());
    assert!(lock.try_write().is_none());

    let reader = RwLockUpgradableReadGuard::downgrade(upgradable);
    assert_eq!(reader.len(), 3);
    assert_eq!(reader[0], 1);

    let second_upgradable = lock.try_upgradable_read();
    assert!(second_upgradable.is_some());
    drop(second_upgradable);

    assert!(lock.try_write().is_none());
    drop(reader);

    {
        let mut writer = lock.try_write().expect("write lock should be available after readers are dropped");
        writer.push(4);
    }

    let final_reader = lock.read_blocking();
    assert_eq!(final_reader.as_slice(), &[1, 2, 3, 4]);
}

#[test]
fn downgraded_reader_participates_in_shared_read_workflow() {
    let lock = Arc::new(RwLock::new(10usize));

    {
        let mut writer = lock.write_blocking();
        *writer += 5;
    }

    let upgradable = lock.upgradable_read_blocking();
    assert_eq!(*upgradable, 15);
    assert!(lock.try_write().is_none());

    let downgraded_reader = RwLockUpgradableReadGuard::downgrade(upgradable);
    assert_eq!(*downgraded_reader, 15);

    let concurrent_reader = lock.try_read();
    assert!(concurrent_reader.is_some());
    assert_eq!(**concurrent_reader.as_ref().unwrap(), 15);

    let second_upgradable = lock.try_upgradable_read();
    assert!(second_upgradable.is_some());
    assert_eq!(**second_upgradable.as_ref().unwrap(), 15);

    assert!(lock.try_write().is_none());

    drop(second_upgradable);
    drop(concurrent_reader);
    drop(downgraded_reader);

    let mut writer = lock.try_write().expect("write lock should be available after all read guards are dropped");
    *writer *= 2;
    assert_eq!(*writer, 30);
}

#[test]
fn downgraded_reader_blocks_writer_on_another_thread_until_dropped() {
    let lock = Arc::new(RwLock::new(String::from("initial")));

    let upgradable = lock.upgradable_read_blocking();
    let reader = RwLockUpgradableReadGuard::downgrade(upgradable);
    assert_eq!(reader.as_str(), "initial");

    let lock_for_thread = Arc::clone(&lock);
    let handle = thread::spawn(move || {
        assert!(lock_for_thread.try_write().is_none());
        let read_guard = lock_for_thread.read_blocking();
        assert_eq!(read_guard.as_str(), "initial");
    });

    handle.join().expect("reader thread should complete successfully");

    assert!(lock.try_write().is_none());
    drop(reader);

    {
        let mut writer = lock.try_write().expect("writer should acquire lock after downgraded reader is dropped");
        writer.push_str(" updated");
        assert_eq!(writer.as_str(), "initial updated");
    }

    let reader_after_write = lock.read_blocking();
    assert_eq!(reader_after_write.as_str(), "initial updated");
}