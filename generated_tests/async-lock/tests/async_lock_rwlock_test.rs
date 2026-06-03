use async_lock::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};

#[test]
fn write_guard_downgrade_keeps_written_value_and_releases_exclusive_access() {
    let lock = RwLock::new(String::from("initial"));

    {
        let mut writer = lock.write_blocking();
        writer.push_str("-updated");

        assert!(lock.try_read().is_none());
        assert!(lock.try_upgradable_read().is_none());
        assert!(lock.try_write().is_none());

        let reader = RwLockWriteGuard::downgrade(writer);

        assert_eq!(reader.as_str(), "initial-updated");
        assert!(lock.try_write().is_none());

        {
            let second_reader = lock
                .try_read()
                .expect("downgraded write guard should allow additional shared readers");
            assert_eq!(second_reader.len(), "initial-updated".len());
            assert_eq!(second_reader.as_bytes()[0], b'i');
        }

        assert!(
            lock.try_upgradable_read().is_some(),
            "a normal read guard should not block an upgradable read guard"
        );

        drop(reader);
    }

    {
        let mut writer = lock
            .try_write()
            .expect("exclusive access should be available after downgraded reader is dropped");
        writer.push_str("-again");
    }

    let final_reader = lock.read_blocking();
    assert_eq!(final_reader.as_str(), "initial-updated-again");
}

#[test]
fn write_guard_downgrade_to_upgradable_allows_readers_then_successful_upgrade() {
    let lock = RwLock::new(vec![1, 2, 3]);

    let mut writer = lock.write_blocking();
    writer.push(4);
    writer[0] = 10;

    assert!(lock.try_read().is_none());
    assert!(lock.try_upgradable_read().is_none());
    assert!(lock.try_write().is_none());

    let upgradable = RwLockWriteGuard::downgrade_to_upgradable(writer);

    assert_eq!(upgradable.as_slice(), &[10, 2, 3, 4]);
    assert!(lock.try_upgradable_read().is_none());
    assert!(lock.try_write().is_none());

    {
        let reader = lock
            .try_read()
            .expect("upgradable read guard should allow shared readers");
        assert_eq!(reader.as_slice(), &[10, 2, 3, 4]);

        let failed_upgrade = RwLockUpgradableReadGuard::try_upgrade(upgradable);
        assert!(
            failed_upgrade.is_err(),
            "upgrade should fail while another reader is active"
        );

        let upgradable = failed_upgrade.err().expect("failed upgrade returns the guard");
        assert_eq!(upgradable.len(), 4);
        drop(reader);

        let mut writer = RwLockUpgradableReadGuard::try_upgrade(upgradable)
            .expect("upgrade should succeed after other readers are dropped");
        writer.push(5);
        writer[1] = 20;

        let reader = RwLockWriteGuard::downgrade(writer);
        assert_eq!(reader.as_slice(), &[10, 20, 3, 4, 5]);
        assert!(lock.try_write().is_none());
        drop(reader);
    }

    let final_reader = lock.read_blocking();
    assert_eq!(final_reader.as_slice(), &[10, 20, 3, 4, 5]);
}

#[test]
fn chained_downgrade_to_upgradable_and_downgrade_preserve_lock_state() {
    let lock = RwLock::new(0usize);

    {
        let mut writer = lock.write_blocking();
        *writer = 41;

        let upgradable = RwLockWriteGuard::downgrade_to_upgradable(writer);
        assert_eq!(*upgradable, 41);
        assert!(lock.try_write().is_none());

        let mut writer = RwLockUpgradableReadGuard::upgrade_blocking(upgradable);
        *writer += 1;

        let reader = RwLockWriteGuard::downgrade(writer);
        assert_eq!(*reader, 42);

        let another_reader = lock
            .try_read()
            .expect("downgraded writer should become a shared reader");
        assert_eq!(*another_reader, 42);

        assert!(lock.try_write().is_none());
        drop(another_reader);
        drop(reader);
    }

    let mut final_writer = lock
        .try_write()
        .expect("write lock should be available after all readers are dropped");
    *final_writer += 8;
    assert_eq!(*final_writer, 50);
}