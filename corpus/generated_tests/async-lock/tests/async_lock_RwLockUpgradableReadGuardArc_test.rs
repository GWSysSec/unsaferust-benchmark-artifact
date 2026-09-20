use async_lock::{RwLock, RwLockUpgradableReadGuardArc, RwLockWriteGuardArc};
use std::sync::Arc;
use std::thread;

#[test]
fn arc_upgradable_downgrade_allows_new_upgradable_reader_but_keeps_writer_excluded() {
    let lock = Arc::new(RwLock::new(vec!["initial".to_string()]));

    let upgradable = lock.upgradable_read_arc_blocking();
    assert_eq!(upgradable.as_slice(), &["initial".to_string()]);
    assert!(lock.try_upgradable_read_arc().is_none());
    assert!(lock.try_write_arc().is_none());

    let reader = RwLockUpgradableReadGuardArc::downgrade(upgradable);
    assert_eq!(reader.len(), 1);
    assert_eq!(reader[0], "initial");

    let second_upgradable = lock.try_upgradable_read_arc();
    assert!(second_upgradable.is_some());
    assert!(lock.try_write_arc().is_none());
    drop(second_upgradable);

    assert!(lock.try_write_arc().is_none());
    drop(reader);

    {
        let mut writer = lock
            .try_write_arc()
            .expect("write lock should be available after all readers are dropped");
        writer.push("written".to_string());
    }

    let final_reader = lock.read_arc_blocking();
    assert_eq!(
        final_reader.as_slice(),
        &["initial".to_string(), "written".to_string()]
    );
}

#[test]
fn downgraded_arc_reader_can_coexist_with_plain_and_arc_readers() {
    let lock = Arc::new(RwLock::new(7usize));

    let upgradable = lock.upgradable_read_arc_blocking();
    assert_eq!(*upgradable, 7);
    assert!(lock.try_upgradable_read_arc().is_none());

    let downgraded_reader = RwLockUpgradableReadGuardArc::downgrade(upgradable);
    assert_eq!(*downgraded_reader, 7);

    let plain_reader = lock
        .try_read()
        .expect("regular readers should be allowed while an arc read guard is held");
    assert_eq!(*plain_reader, 7);

    let arc_reader = lock
        .try_read_arc()
        .expect("additional arc readers should be allowed while read guards are held");
    assert_eq!(*arc_reader, 7);

    assert!(lock.try_write_arc().is_none());
    assert!(lock.try_write().is_none());

    drop(plain_reader);
    drop(arc_reader);
    assert!(lock.try_write_arc().is_none());

    drop(downgraded_reader);

    {
        let mut writer = lock
            .try_write_arc()
            .expect("writer should acquire lock after downgraded reader is dropped");
        *writer += 1;
    }

    assert_eq!(*lock.read_arc_blocking(), 8);
}

#[test]
fn downgraded_arc_reader_survives_original_arc_and_blocks_cross_thread_writer() {
    let lock = Arc::new(RwLock::new(String::from("start")));

    let upgradable = lock.upgradable_read_arc_blocking();
    assert_eq!(upgradable.as_str(), "start");

    let downgraded_reader = RwLockUpgradableReadGuardArc::downgrade(upgradable);
    assert_eq!(downgraded_reader.as_str(), "start");

    let writer_lock = Arc::clone(&lock);
    let writer_attempt = thread::spawn(move || writer_lock.try_write_arc().is_some());

    assert!(
        !writer_attempt
            .join()
            .expect("writer attempt thread should not panic"),
        "writer must not acquire lock while downgraded reader is alive"
    );

    let second_upgradable = lock
        .try_upgradable_read_arc()
        .expect("downgrade should release the exclusive upgradable-reader slot");
    assert_eq!(second_upgradable.as_str(), "start");
    drop(second_upgradable);

    drop(downgraded_reader);

    let writer_lock = Arc::clone(&lock);
    let writer_update = thread::spawn(move || {
        let mut writer = writer_lock.write_arc_blocking();
        writer.push_str("-updated");
        RwLockWriteGuardArc::downgrade(writer)
    });

    let final_reader = writer_update
        .join()
        .expect("writer update thread should not panic");
    assert_eq!(final_reader.as_str(), "start-updated");
    assert!(lock.try_write_arc().is_none());

    drop(final_reader);

    let lock = match Arc::try_unwrap(lock) {
        Ok(lock) => lock,
        Err(_) => panic!("all Arc clones should have been dropped"),
    };
    assert_eq!(lock.into_inner(), "start-updated");
}