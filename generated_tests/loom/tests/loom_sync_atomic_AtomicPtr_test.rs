#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

#[test]
fn atomic_ptr_sequential_pointer_replacement_workflow() {
    loom::model(|| {
        let p1 = Box::into_raw(Box::new(10_i32));
        let p2 = Box::into_raw(Box::new(20_i32));
        let p3 = Box::into_raw(Box::new(30_i32));
        let p4 = Box::into_raw(Box::new(40_i32));

        let mut atomic = AtomicPtr::new(p1);

        let with_mut_saw_initial = atomic.with_mut(|slot| {
            assert_eq!(*slot, p1);
            *slot = p2;
            *slot == p2
        });
        assert!(with_mut_saw_initial);
        assert_eq!(atomic.load(Ordering::SeqCst), p2);

        let swapped_out = atomic.swap(p3, Ordering::SeqCst);
        assert_eq!(swapped_out, p2);
        assert_eq!(atomic.load(Ordering::SeqCst), p3);

        let failed_exchange =
            atomic.compare_exchange_weak(p2, p1, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed_exchange, Err(p3));
        assert_eq!(atomic.load(Ordering::SeqCst), p3);

        let updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, p3);
            Some(p4)
        });
        assert_eq!(updated, Ok(p3));
        assert_eq!(atomic.load(Ordering::SeqCst), p4);

        let not_updated = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, p4);
            None
        });
        assert_eq!(not_updated, Err(p4));
        assert_eq!(atomic.load(Ordering::SeqCst), p4);

        unsafe {
            drop(Box::from_raw(p1));
            drop(Box::from_raw(p2));
            drop(Box::from_raw(p3));
            drop(Box::from_raw(p4));
        }
    });
}

#[test]
fn atomic_ptr_null_edge_case_and_reinitialization_workflow() {
    loom::model(|| {
        let value = Box::into_raw(Box::new(55_i32));
        let replacement = Box::into_raw(Box::new(89_i32));
        let null = ptr::null_mut::<i32>();

        let mut atomic = AtomicPtr::new(null);

        let was_null_before_initialization = atomic.with_mut(|slot| {
            assert!((*slot).is_null());
            *slot = value;
            (*slot).is_null()
        });
        assert!(!was_null_before_initialization);
        assert_eq!(atomic.load(Ordering::SeqCst), value);

        let previous = atomic.swap(null, Ordering::SeqCst);
        assert_eq!(previous, value);
        assert!(atomic.load(Ordering::SeqCst).is_null());

        let failed_exchange =
            atomic.compare_exchange_weak(value, replacement, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed_exchange, Err(null));
        assert!(atomic.load(Ordering::SeqCst).is_null());

        let update_from_null = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert!(current.is_null());
            Some(replacement)
        });
        assert_eq!(update_from_null, Ok(null));
        assert_eq!(atomic.load(Ordering::SeqCst), replacement);

        let reject_non_null = atomic.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            assert_eq!(current, replacement);
            None
        });
        assert_eq!(reject_non_null, Err(replacement));
        assert_eq!(atomic.load(Ordering::SeqCst), replacement);

        unsafe {
            drop(Box::from_raw(value));
            drop(Box::from_raw(replacement));
        }
    });
}