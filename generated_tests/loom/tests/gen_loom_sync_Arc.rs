#![deny(warnings, rust_2018_idioms)]

use loom::sync::Arc;
use loom::thread;

#[test]
fn arc_pin_basic() {
    loom::model(|| {
        let pinned = Arc::pin(42u64);
        let inner = pinned.as_ref().get_ref();
        assert_eq!(*inner, 42u64);

        let pinned2 = Arc::pin(String::from("hello"));
        let inner2 = pinned2.as_ref().get_ref();
        assert_eq!(inner2, "hello");
        assert_eq!(inner2.len(), 5);

        let pinned3 = Arc::pin(vec![1, 2, 3]);
        let inner3 = pinned3.as_ref().get_ref();
        assert_eq!(inner3.len(), 3);
        assert_eq!(inner3[0], 1);
        assert_eq!(inner3[1], 2);
        assert_eq!(inner3[2], 3);
    });
}

#[test]
fn arc_strong_count_single_thread() {
    loom::model(|| {
        let a = Arc::new(100u32);
        assert_eq!(Arc::strong_count(&a), 1);

        let b = Arc::clone(&a);
        assert_eq!(Arc::strong_count(&a), 2);
        assert_eq!(Arc::strong_count(&b), 2);

        let c = Arc::clone(&b);
        assert_eq!(Arc::strong_count(&a), 3);
        assert_eq!(Arc::strong_count(&b), 3);
        assert_eq!(Arc::strong_count(&c), 3);

        drop(c);
        assert_eq!(Arc::strong_count(&a), 2);

        drop(b);
        assert_eq!(Arc::strong_count(&a), 1);
    });
}

#[test]
fn arc_strong_count_multithreaded() {
    loom::model(|| {
        let a = Arc::new(77i32);
        assert_eq!(Arc::strong_count(&a), 1);

        let b = Arc::clone(&a);
        assert_eq!(Arc::strong_count(&a), 2);

        let handle = thread::spawn(move || {
            let count = Arc::strong_count(&b);
            assert!(count >= 1);
            assert!(count <= 2);
            drop(b);
        });

        handle.join().unwrap();
        assert_eq!(Arc::strong_count(&a), 1);
        assert_eq!(*a, 77i32);
    });
}

#[test]
fn arc_ptr_eq_basic() {
    loom::model(|| {
        let a = Arc::new(999u64);
        let b = Arc::clone(&a);
        let c = Arc::new(999u64);

        assert!(Arc::ptr_eq(&a, &b));
        assert!(Arc::ptr_eq(&b, &a));
        assert!(!Arc::ptr_eq(&a, &c));
        assert!(!Arc::ptr_eq(&b, &c));

        let d = Arc::clone(&a);
        assert!(Arc::ptr_eq(&a, &d));
        assert!(Arc::ptr_eq(&b, &d));
        assert!(!Arc::ptr_eq(&c, &d));

        drop(b);
        assert!(Arc::ptr_eq(&a, &d));
    });
}

#[test]
fn arc_into_raw_and_as_ptr() {
    loom::model(|| {
        let a = Arc::new(42usize);
        let b = Arc::clone(&a);

        let ptr_a = Arc::as_ptr(&a);
        let ptr_b = Arc::as_ptr(&b);
        assert_eq!(ptr_a, ptr_b);

        let val = unsafe { *ptr_a };
        assert_eq!(val, 42usize);

        let raw = Arc::into_raw(b);
        assert_eq!(raw, ptr_a);

        let val_raw = unsafe { *raw };
        assert_eq!(val_raw, 42usize);




        assert_eq!(Arc::strong_count(&a), 2);


        let reconstructed = unsafe { Arc::from_raw(raw) };
        assert_eq!(*reconstructed, 42usize);
        assert_eq!(Arc::strong_count(&a), 2);
    });
}

#[test]
fn arc_as_ptr_consistency() {
    loom::model(|| {
        let a = Arc::new(String::from("test_value"));
        let ptr1 = Arc::as_ptr(&a);
        let ptr2 = Arc::as_ptr(&a);
        assert_eq!(ptr1, ptr2);

        let b = Arc::clone(&a);
        let ptr3 = Arc::as_ptr(&b);
        assert_eq!(ptr1, ptr3);

        let val = unsafe { &*ptr1 };
        assert_eq!(val, "test_value");
        assert_eq!(val.len(), 10);

        drop(b);
        let ptr4 = Arc::as_ptr(&a);
        assert_eq!(ptr1, ptr4);
    });
}

#[test]
fn arc_increment_decrement_strong_count() {
    loom::model(|| {
        let a = Arc::new(55u32);
        assert_eq!(Arc::strong_count(&a), 1);

        let raw = Arc::into_raw(a);


        unsafe { Arc::increment_strong_count(raw) };


        let recovered1 = unsafe { Arc::from_raw(raw) };
        assert_eq!(Arc::strong_count(&recovered1), 2);
        assert_eq!(*recovered1, 55u32);

        unsafe { Arc::decrement_strong_count(raw) };

        assert_eq!(Arc::strong_count(&recovered1), 1);
        assert_eq!(*recovered1, 55u32);
    });
}

#[test]
fn arc_increment_strong_count_multiple() {
    loom::model(|| {
        let a = Arc::new(123i64);
        let raw = Arc::as_ptr(&a);

        assert_eq!(Arc::strong_count(&a), 1);

        unsafe { Arc::increment_strong_count(raw) };
        assert_eq!(Arc::strong_count(&a), 2);

        unsafe { Arc::increment_strong_count(raw) };
        assert_eq!(Arc::strong_count(&a), 3);

        unsafe { Arc::decrement_strong_count(raw) };
        assert_eq!(Arc::strong_count(&a), 2);

        unsafe { Arc::decrement_strong_count(raw) };
        assert_eq!(Arc::strong_count(&a), 1);

        assert_eq!(*a, 123i64);
    });
}

#[test]
fn arc_into_raw_roundtrip() {
    loom::model(|| {
        let original_val = vec![10u8, 20, 30, 40];
        let a = Arc::new(original_val);
        assert_eq!(Arc::strong_count(&a), 1);

        let b = Arc::clone(&a);
        assert_eq!(Arc::strong_count(&a), 2);

        let raw = Arc::into_raw(b);


        assert_eq!(Arc::strong_count(&a), 2);

        let val_ref = unsafe { &*raw };
        assert_eq!(val_ref.len(), 4);
        assert_eq!(val_ref[0], 10u8);
        assert_eq!(val_ref[3], 40u8);

        let recovered = unsafe { Arc::from_raw(raw) };
        assert_eq!(Arc::strong_count(&a), 2);
        assert!(Arc::ptr_eq(&a, &recovered));
    });
}

#[test]
fn arc_ptr_eq_multithreaded() {
    loom::model(|| {
        let a = Arc::new(200u32);
        let b = Arc::clone(&a);
        let c = Arc::clone(&a);

        assert!(Arc::ptr_eq(&a, &b));
        assert!(Arc::ptr_eq(&a, &c));

        let handle = thread::spawn(move || {
            assert!(Arc::ptr_eq(&b, &c));
            assert_eq!(*b, 200u32);
            assert_eq!(*c, 200u32);
            let not_same = Arc::new(200u32);
            assert!(!Arc::ptr_eq(&b, &not_same));
        });

        handle.join().unwrap();
        assert_eq!(Arc::strong_count(&a), 1);
        assert_eq!(*a, 200u32);
    });
}

#[test]
fn arc_pin_with_clone_and_strong_count() {
    loom::model(|| {
        let pinned = Arc::pin(88u32);
        let inner = pinned.as_ref().get_ref();
        assert_eq!(*inner, 88u32);


        let pinned2 = pinned.clone();
        let inner2 = pinned2.as_ref().get_ref();
        assert_eq!(*inner2, 88u32);




        assert_eq!(*pinned.as_ref().get_ref(), *pinned2.as_ref().get_ref());

        drop(pinned2);
        assert_eq!(*pinned.as_ref().get_ref(), 88u32);
    });
}