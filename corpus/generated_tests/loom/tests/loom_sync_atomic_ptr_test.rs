#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicPtr, Ordering};

#[test]
fn atomic_ptr_sequential_with_mut_swap_compare_exchange_weak_and_fetch_update() {
    loom::model(|| {
        let first = Box::into_raw(Box::new(10_i32));
        let second = Box::into_raw(Box::new(20_i32));
        let third = Box::into_raw(Box::new(30_i32));

        let mut atomic = AtomicPtr::new(first);

        let value_observed_through_with_mut = atomic.with_mut(|slot| {
            assert_eq!(*slot, first);
            let observed = unsafe { **slot };
            *slot = second;
            observed
        });
        assert_eq!(value_observed_through_with_mut, 10);
        assert_eq!(atomic.load(Ordering::SeqCst), second);

        let swapped_out = atomic.swap(third, Ordering::SeqCst);
        assert_eq!(swapped_out, second);
        assert_eq!(atomic.load(Ordering::SeqCst), third);
        assert_eq!(unsafe { *atomic.load(Ordering::SeqCst) }, 30);

        let failed_exchange =
            atomic.compare_exchange_weak(first, second, Ordering::SeqCst, Ordering::SeqCst);
        assert_eq!(failed_exchange, Err(third));
        assert_eq!(atomic.load(Ordering::SeqCst), third);

        let update_result = atomic.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |current| {
                assert_eq!(current, third);
                Some(first)
            },
        );
        assert_eq!(update_result, Ok(third));
        assert_eq!(atomic.load(Ordering::SeqCst), first);
        assert_eq!(unsafe { *atomic.load(Ordering::SeqCst) }, 10);

        let no_update_result = atomic.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |current| {
                assert_eq!(current, first);
                None
            },
        );
        assert_eq!(no_update_result, Err(first));
        assert_eq!(atomic.load(Ordering::SeqCst), first);

        unsafe {
            drop(Box::from_raw(first));
            drop(Box::from_raw(second));
            drop(Box::from_raw(third));
        }
    });
}

#[test]
fn atomic_ptr_fetch_update_can_move_between_null_and_valid_pointer_states() {
    loom::model(|| {
        let payload = Box::into_raw(Box::new(77_i32));
        let mut atomic = AtomicPtr::<i32>::new(std::ptr::null_mut());

        let initially_null = atomic.with_mut(|slot| {
            assert!((*slot).is_null());
            *slot = payload;
            (*slot).is_null()
        });
        assert!(!initially_null);
        assert_eq!(atomic.load(Ordering::SeqCst), payload);
        assert_eq!(unsafe { *payload }, 77);

        let removed = atomic.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |current| {
                if current.is_null() {
                    None
                } else {
                    Some(std::ptr::null_mut())
                }
            },
        );
        assert_eq!(removed, Ok(payload));
        assert!(atomic.load(Ordering::SeqCst).is_null());

        let previous = atomic.swap(payload, Ordering::SeqCst);
        assert!(previous.is_null());
        assert_eq!(atomic.load(Ordering::SeqCst), payload);

        let failed_exchange = atomic.compare_exchange_weak(
            std::ptr::null_mut(),
            payload,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert_eq!(failed_exchange, Err(payload));
        assert_eq!(atomic.load(Ordering::SeqCst), payload);

        unsafe {
            drop(Box::from_raw(payload));
        }
    });
}