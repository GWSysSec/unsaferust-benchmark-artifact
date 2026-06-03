use parking_lot::{const_rwlock, RwLock};
use std::sync::Arc;
use std::thread;

#[test]
fn rwlock_module_const_rwlock_supports_shared_reads_and_exclusive_writes() {
    const INITIAL: RwLock<Vec<i32>> = const_rwlock(Vec::new());

    let shared = Arc::new(INITIAL);

    {
        let initial_read = shared.read();
        assert!(initial_read.is_empty());
        assert!(
            shared.try_read().is_some(),
            "rwlock should allow multiple readers"
        );
        assert!(
            shared.try_write().is_none(),
            "rwlock should not allow a writer while a reader is active"
        );
    }

    {
        let mut writer = shared.write();
        writer.extend([40, 2]);
        assert_eq!(writer.as_slice(), &[40, 2]);
        assert!(
            shared.try_read().is_none(),
            "rwlock should not allow readers while a writer is active"
        );
        assert!(
            shared.try_write().is_none(),
            "rwlock should not allow another writer while a writer is active"
        );
    }

    {
        let read_after_write = shared.read();
        assert_eq!(read_after_write.len(), 2);
        assert!(read_after_write.contains(&40));
        assert!(read_after_write.contains(&2));
    }

    let mut handles = Vec::new();
    for value in [3, 1, 50] {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            {
                let mut writer = shared.write();
                writer.push(value);
            }

            let reader = shared.read();
            assert!(
                reader.contains(&value),
                "thread should observe the value it inserted"
            );
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    {
        let mut writer = shared.write();
        writer.sort();
        assert_eq!(writer.as_slice(), &[1, 2, 3, 40, 50]);
    }

    let final_read = shared.read();
    assert_eq!(final_read.len(), 5);
    assert_eq!(final_read.first(), Some(&1));
    assert_eq!(final_read.last(), Some(&50));
}