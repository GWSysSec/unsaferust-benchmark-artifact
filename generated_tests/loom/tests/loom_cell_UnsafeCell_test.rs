#![deny(warnings, rust_2018_idioms)]

use loom::cell::UnsafeCell;
use loom::sync::atomic::AtomicBool;
use loom::thread;

use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::Arc;

#[test]
fn unsafe_cell_publish_and_read_after_acquire() {
    struct Packet {
        fields: UnsafeCell<(u32, u32)>,
        ready: AtomicBool,
    }

    loom::model(|| {
        let packet = Arc::new(Packet {
            fields: UnsafeCell::new((0, 0)),
            ready: AtomicBool::new(false),
        });

        let writer = {
            let packet = Arc::clone(&packet);
            thread::spawn(move || {
                let computed = packet.fields.with_mut(|ptr| unsafe {
                    assert_eq!((*ptr).0, 0);
                    assert_eq!((*ptr).1, 0);

                    (*ptr).0 = 10;
                    (*ptr).1 = 32;

                    (*ptr).0 + (*ptr).1
                });

                assert_eq!(computed, 42);
                assert!(!packet.ready.swap(true, Release));
            })
        };

        let reader = {
            let packet = Arc::clone(&packet);
            thread::spawn(move || {
                if packet.ready.load(Acquire) {
                    let snapshot = packet.fields.with(|ptr| unsafe { *ptr });

                    assert_eq!(snapshot, (10, 32));
                    assert_eq!(snapshot.0 + snapshot.1, 42);
                    assert!(snapshot.0 < snapshot.1);
                }
            })
        };

        writer.join().unwrap();
        reader.join().unwrap();

        assert!(packet.ready.load(Acquire));

        let final_snapshot = packet.fields.with(|ptr| unsafe { *ptr });
        assert_eq!(final_snapshot, (10, 32));
    });
}

#[test]
fn unsafe_cell_with_mut_can_build_state_then_with_observes_it() {
    loom::model(|| {
        let cell = UnsafeCell::new(Vec::<i32>::new());

        let first_len = cell.with_mut(|ptr| unsafe {
            (*ptr).push(7);
            (*ptr).len()
        });
        assert_eq!(first_len, 1);

        let removed = cell.with_mut(|ptr| unsafe {
            (*ptr).push(11);
            (*ptr).push(13);
            (*ptr).remove(0)
        });
        assert_eq!(removed, 7);

        let stats = cell.with(|ptr| unsafe {
            let values = &*ptr;
            (values.len(), values[0], values[1], values.iter().sum::<i32>())
        });
        assert_eq!(stats, (2, 11, 13, 24));

        let last = cell.with_mut(|ptr| unsafe { (*ptr).pop() });
        assert_eq!(last, Some(13));

        let remaining = cell.with(|ptr| unsafe { (*ptr).clone() });
        assert_eq!(remaining, vec![11]);

        assert_eq!(cell.into_inner(), vec![11]);
    });
}