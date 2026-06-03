use async_lock::{RwLock, RwLockUpgradableReadGuardArc, RwLockWriteGuardArc};
use std::sync::Arc;

#[test]
fn arc_write_guard_downgrade_preserves_mutations_and_allows_shared_readers() {
    let lock = Arc::new(RwLock::new(vec![1, 2, 3]));

    let mut writer = lock.write_arc_blocking();
    writer.push(4);
    writer[0] = 10;

    assert!(lock.try_read_arc().is_none());
    assert!(lock.try_write_arc().is_none());

    let reader = RwLockWriteGuardArc::downgrade(writer);

    assert_eq!(reader.as_slice(), &[10, 2, 3, 4]);
    assert!(lock.try_write_arc().is_none());

    {
        let second_reader = lock
            .try_read_arc()
            .expect("downgraded write guard should allow additional shared readers");
        assert_eq!(second_reader.len(), 4);
        assert_eq!(second_reader[3], 4);
    }

    assert!(lock.try_write_arc().is_none());
    drop(reader);

    {
        let mut next_writer = lock
            .try_write_arc()
            .expect("writer should be available after downgraded reader is dropped");
        next_writer.push(5);
        next_writer[1] = 20;
    }

    let final_reader = lock.read_arc_blocking();
    assert_eq!(final_reader.as_slice(), &[10, 20, 3, 4, 5]);
}

#[test]
fn arc_write_guard_downgrade_to_upgradable_can_later_upgrade() {
    let lock = Arc::new(RwLock::new(7usize));

    let mut writer = lock.write_arc_blocking();
    *writer *= 3;

    assert_eq!(*writer, 21);
    assert!(lock.try_read_arc().is_none());
    assert!(lock.try_upgradable_read_arc().is_none());
    assert!(lock.try_write_arc().is_none());

    let upgradable = RwLockWriteGuardArc::downgrade_to_upgradable(writer);

    assert_eq!(*upgradable, 21);
    assert!(lock.try_write_arc().is_none());
    assert!(lock.try_upgradable_read_arc().is_none());

    {
        let reader = lock
            .try_read_arc()
            .expect("upgradable read guard should coexist with regular readers");
        assert_eq!(*reader, 21);
        assert!(RwLockUpgradableReadGuardArc::try_upgrade(upgradable).is_err());
    }

    let upgradable = lock
        .try_upgradable_read_arc()
        .expect("upgradable guard should be obtainable after failed upgrade returns and reader drops");

    let mut writer = RwLockUpgradableReadGuardArc::try_upgrade(upgradable)
        .expect("upgradable guard should upgrade when no other readers exist");
    *writer += 1;

    assert!(lock.try_read_arc().is_none());

    let reader = RwLockWriteGuardArc::downgrade(writer);
    assert_eq!(*reader, 22);
    drop(reader);

    let final_reader = lock.read_arc_blocking();
    assert_eq!(*final_reader, 22);
}