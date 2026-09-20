use async_lock::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};

#[test]
fn write_guard_downgrade_preserves_changes_and_allows_shared_readers() {
    let lock = RwLock::new(vec![1, 2, 3]);

    let mut writer = lock.write_blocking();
    writer.push(4);
    writer[0] = 10;

    assert!(lock.try_read().is_none());
    assert!(lock.try_write().is_none());

    let reader = RwLockWriteGuard::downgrade(writer);

    assert_eq!(reader.as_slice(), &[10, 2, 3, 4]);
    assert!(lock.try_write().is_none());

    {
        let second_reader = lock
            .try_read()
            .expect("downgraded write guard should now be a shared reader");
        assert_eq!(second_reader.len(), 4);
        assert_eq!(second_reader[3], 4);
    }

    assert!(lock.try_write().is_none());
    drop(reader);

    {
        let mut next_writer = lock
            .try_write()
            .expect("writer should be available after downgraded reader is dropped");
        next_writer.push(5);
        next_writer[1] = 20;
    }

    let final_reader = lock.read_blocking();
    assert_eq!(final_reader.as_slice(), &[10, 20, 3, 4, 5]);
}

#[test]
fn write_guard_downgrade_to_upgradable_can_be_read_then_upgraded() {
    let lock = RwLock::new(String::from("start"));

    let mut writer = lock.write_blocking();
    writer.push_str("-written");

    assert!(lock.try_read().is_none());
    assert!(lock.try_upgradable_read().is_none());
    assert!(lock.try_write().is_none());

    let upgradable = RwLockWriteGuard::downgrade_to_upgradable(writer);

    assert_eq!(upgradable.as_str(), "start-written");
    assert!(lock.try_write().is_none());
    assert!(lock.try_upgradable_read().is_none());

    {
        let shared_reader = lock
            .try_read()
            .expect("upgradable read guard should allow ordinary shared readers");
        assert_eq!(shared_reader.as_str(), "start-written");
        assert!(
            RwLockUpgradableReadGuard::try_upgrade(upgradable).is_err(),
            "upgrade should fail while another reader is active"
        );
    }

    let upgradable = lock
        .try_upgradable_read()
        .expect("previous failed upgrade should return and drop the upgradable guard");
    assert_eq!(upgradable.as_str(), "start-written");

    let mut upgraded = RwLockUpgradableReadGuard::try_upgrade(upgradable)
        .expect("upgrade should succeed when no other readers are active");
    upgraded.push_str("-upgraded");
    assert!(lock.try_read().is_none());

    let reader = RwLockWriteGuard::downgrade(upgraded);
    assert_eq!(reader.as_str(), "start-written-upgraded");
    drop(reader);

    let final_reader = lock.read_blocking();
    assert_eq!(final_reader.as_str(), "start-written-upgraded");
}