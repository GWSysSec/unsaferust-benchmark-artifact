#![deny(warnings, rust_2018_idioms)]

use loom::sync::Arc;
use std::pin::Pin;

#[test]
fn pinned_arc_workflow_tracks_identity_and_counts() {
    loom::model(|| {
        let pinned: Pin<Arc<(u32, &'static str)>> = Arc::pin((42, "answer"));
        assert_eq!(pinned.0, 42);
        assert_eq!(pinned.1, "answer");

        let original = Pin::into_inner(pinned);
        assert_eq!(Arc::strong_count(&original), 1);

        let cloned = Arc::clone(&original);
        assert_eq!(Arc::strong_count(&original), 2);
        assert_eq!(Arc::strong_count(&cloned), 2);
        assert!(Arc::ptr_eq(&original, &cloned));
        assert_eq!(Arc::as_ptr(&original), Arc::as_ptr(&cloned));

        let same_value_different_allocation = Pin::into_inner(Arc::pin((42, "answer")));
        assert_eq!(*same_value_different_allocation, *original);
        assert!(!Arc::ptr_eq(&original, &same_value_different_allocation));
        assert_ne!(
            Arc::as_ptr(&original),
            Arc::as_ptr(&same_value_different_allocation)
        );

        drop(cloned);
        assert_eq!(Arc::strong_count(&original), 1);
    });
}

#[test]
fn raw_round_trip_and_manual_strong_count_increment_create_independent_owners() {
    loom::model(|| {
        let original = Pin::into_inner(Arc::pin(String::from("loom raw arc")));
        assert_eq!(Arc::strong_count(&original), 1);

        let raw = Arc::into_raw(original);

        unsafe {
            assert_eq!(&*raw, "loom raw arc");

            Arc::increment_strong_count(raw);

            let first_owner = Arc::from_raw(raw);
            assert_eq!(Arc::strong_count(&first_owner), 2);
            assert_eq!(first_owner.as_str(), "loom raw arc");

            let second_owner = Arc::from_raw(raw);
            assert_eq!(Arc::strong_count(&first_owner), 2);
            assert!(Arc::ptr_eq(&first_owner, &second_owner));
            assert_eq!(Arc::as_ptr(&first_owner), Arc::as_ptr(&second_owner));

            drop(second_owner);
            assert_eq!(Arc::strong_count(&first_owner), 1);
        }
    });
}

#[test]
fn manual_decrement_balances_increment_before_recovering_from_raw() {
    loom::model(|| {
        let arc_value = Pin::into_inner(Arc::pin(vec![1_u8, 2, 3, 4]));
        assert_eq!(Arc::strong_count(&arc_value), 1);

        let raw = Arc::into_raw(arc_value);

        unsafe {
            assert_eq!((*raw).as_slice(), &[1, 2, 3, 4]);

            Arc::increment_strong_count(raw);
            Arc::decrement_strong_count(raw);

            let recovered = Arc::from_raw(raw);
            assert_eq!(Arc::strong_count(&recovered), 1);
            assert_eq!(recovered.len(), 4);
            assert_eq!(recovered[0], 1);
            assert_eq!(recovered[3], 4);
        }
    });
}

#[test]
fn raw_pointer_can_be_temporarily_recovered_without_changing_allocation_identity() {
    loom::model(|| {
        let first = Pin::into_inner(Arc::pin([10_i32, 20, 30]));
        let first_ptr = Arc::as_ptr(&first);
        assert_eq!(Arc::strong_count(&first), 1);

        let raw = Arc::into_raw(first);

        unsafe {
            let recovered = Arc::from_raw(raw);
            assert_eq!(Arc::strong_count(&recovered), 1);
            assert_eq!(Arc::as_ptr(&recovered), first_ptr);
            assert_eq!(recovered[1], 20);

            let clone = Arc::clone(&recovered);
            assert_eq!(Arc::strong_count(&recovered), 2);
            assert!(Arc::ptr_eq(&recovered, &clone));
            assert_eq!(Arc::as_ptr(&clone), first_ptr);

            drop(clone);
            assert_eq!(Arc::strong_count(&recovered), 1);
        }
    });
}