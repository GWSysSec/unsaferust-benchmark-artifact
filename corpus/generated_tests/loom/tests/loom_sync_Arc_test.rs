#![deny(warnings, rust_2018_idioms)]

use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::sync::Arc;
use loom::thread;
use std::pin::Pin;

#[test]
fn pinned_arc_can_be_unpinned_cloned_and_compared_by_allocation_identity() {
    loom::model(|| {
        let pinned: Pin<Arc<(u32, &'static str)>> = Arc::pin((7, "loom"));
        assert_eq!(pinned.0, 7);
        assert_eq!(pinned.1, "loom");

        let arc = Pin::into_inner(pinned);
        assert_eq!(Arc::strong_count(&arc), 1);

        let cloned = Arc::clone(&arc);
        assert_eq!(Arc::strong_count(&arc), 2);
        assert_eq!(Arc::strong_count(&cloned), 2);
        assert!(Arc::ptr_eq(&arc, &cloned));
        assert_eq!(Arc::as_ptr(&arc), Arc::as_ptr(&cloned));

        let independent = Arc::pin((7, "loom"));
        let independent = Pin::into_inner(independent);
        assert_eq!(*independent, *arc);
        assert!(!Arc::ptr_eq(&arc, &independent));
        assert_ne!(Arc::as_ptr(&arc), Arc::as_ptr(&independent));

        drop(cloned);
        assert_eq!(Arc::strong_count(&arc), 1);
    });
}

#[test]
fn raw_arc_round_trip_preserves_value_and_manual_increment_creates_extra_owner() {
    loom::model(|| {
        let original = Pin::into_inner(Arc::pin(String::from("raw round trip")));
        assert_eq!(Arc::strong_count(&original), 1);

        let raw = Arc::into_raw(original);
        assert!(!raw.is_null());

        unsafe {
            Arc::increment_strong_count(raw);

            let first = Arc::from_raw(raw);
            assert_eq!(first.as_str(), "raw round trip");
            assert_eq!(Arc::strong_count(&first), 2);

            let second = Arc::from_raw(raw);
            assert_eq!(second.as_str(), "raw round trip");
            assert_eq!(Arc::strong_count(&first), 2);
            assert!(Arc::ptr_eq(&first, &second));
            assert_eq!(Arc::as_ptr(&first), raw);
            assert_eq!(Arc::as_ptr(&second), raw);

            drop(second);
            assert_eq!(Arc::strong_count(&first), 1);
        }
    });
}

#[test]
fn manual_decrement_releases_an_incremented_raw_strong_reference() {
    loom::model(|| {
        let arc = Pin::into_inner(Arc::pin(vec![1_u8, 2, 3, 5, 8]));
        assert_eq!(Arc::strong_count(&arc), 1);
        assert_eq!(arc.iter().copied().sum::<u8>(), 19);

        let raw = Arc::into_raw(arc);

        unsafe {
            Arc::increment_strong_count(raw);
            Arc::decrement_strong_count(raw);

            let restored = Arc::from_raw(raw);
            assert_eq!(Arc::strong_count(&restored), 1);
            assert_eq!(restored.as_slice(), &[1, 2, 3, 5, 8]);
            assert_eq!(Arc::as_ptr(&restored), raw);
        }
    });
}

#[test]
fn raw_arc_pointer_can_be_rebuilt_after_concurrent_observation_workflow() {
    loom::model(|| {
        let shared = Pin::into_inner(Arc::pin(AtomicUsize::new(0)));
        let raw_before_threads = Arc::as_ptr(&shared);

        let worker_arc = Arc::clone(&shared);
        assert_eq!(Arc::strong_count(&shared), 2);
        assert!(Arc::ptr_eq(&shared, &worker_arc));

        let worker = thread::spawn(move || {
            let previous = worker_arc.fetch_add(1, Ordering::SeqCst);
            assert_eq!(previous, 0);
            assert_eq!(worker_arc.load(Ordering::SeqCst), 1);
        });

        worker.join().expect("worker thread should complete");
        assert_eq!(shared.load(Ordering::SeqCst), 1);
        assert_eq!(Arc::strong_count(&shared), 1);
        assert_eq!(Arc::as_ptr(&shared), raw_before_threads);

        let raw = Arc::into_raw(shared);

        unsafe {
            Arc::increment_strong_count(raw);

            let owner = Arc::from_raw(raw);
            assert_eq!(owner.load(Ordering::SeqCst), 1);
            assert_eq!(Arc::strong_count(&owner), 2);

            let duplicate_owner = Arc::from_raw(raw);
            assert!(Arc::ptr_eq(&owner, &duplicate_owner));
            assert_eq!(duplicate_owner.fetch_add(4, Ordering::SeqCst), 1);
            assert_eq!(owner.load(Ordering::SeqCst), 5);
            assert_eq!(Arc::strong_count(&owner), 2);

            drop(duplicate_owner);
            assert_eq!(Arc::strong_count(&owner), 1);
        }
    });
}