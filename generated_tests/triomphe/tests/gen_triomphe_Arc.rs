#![allow(unused_unsafe)]

extern crate triomphe;

use triomphe::{Arc, OffsetArc, UniqueArc};
use std::ffi::c_void;

#[test]
fn test_arc_into_raw_offset_roundtrip_scalar() {
    let arc: Arc<i32> = Arc::new(42);
    let ptr: *const i32 = &*arc as *const i32;


    let offset: OffsetArc<i32> = Arc::into_raw_offset(arc);


    let val_via_offset: i32 = offset.with_arc(|a| **a);
    assert_eq!(val_via_offset, 42);

    let ptr_via_offset: *const i32 = offset.with_arc(|a| &**a as *const i32);
    assert_eq!(ptr_via_offset, ptr);


    let restored: Arc<i32> = Arc::from_raw_offset(offset);
    assert_eq!(*restored, 42);
    assert_eq!(&*restored as *const i32, ptr);
    assert_ne!(*restored, 0);
    assert_ne!(*restored, 41);


    let cloned = restored.clone();
    assert_eq!(*cloned, 42);
    assert_eq!(&*cloned as *const i32, ptr);
}

#[test]
fn test_arc_with_raw_offset_arc_on_string() {
    let arc: Arc<String> = Arc::new(String::from("triomphe"));
    let original_ptr: *const String = &*arc as *const String;

    let len: usize = arc.with_raw_offset_arc(|offset| offset.with_arc(|a| a.len()));
    assert_eq!(len, 8);

    let uppered: String =
        arc.with_raw_offset_arc(|offset| offset.with_arc(|a| a.to_uppercase()));
    assert_eq!(uppered, "TRIOMPHE");

    let starts_with_t: bool =
        arc.with_raw_offset_arc(|offset| offset.with_arc(|a| a.starts_with('t')));
    assert_eq!(starts_with_t, true);

    let ptr_inside: *const String =
        arc.with_raw_offset_arc(|offset| offset.with_arc(|a| &**a as *const String));
    assert_eq!(ptr_inside, original_ptr);

    let first_byte: u8 =
        arc.with_raw_offset_arc(|offset| offset.with_arc(|a| a.as_bytes()[0]));
    assert_eq!(first_byte, b't');


    assert_eq!(&*arc, "triomphe");
    assert_eq!(arc.len(), 8);
    assert_eq!(&*arc as *const String, original_ptr);
}

#[test]
fn test_arc_heap_ptr_stability_and_sharing() {
    let arc: Arc<u64> = Arc::new(123456789);
    let ptr1: *const c_void = arc.heap_ptr();


    let ptr2: *const c_void = arc.heap_ptr();
    assert_eq!(ptr1, ptr2);


    let cloned = arc.clone();
    let ptr3: *const c_void = cloned.heap_ptr();
    assert_eq!(ptr1, ptr3);
    assert_eq!(*cloned, 123456789);


    let other: Arc<u64> = Arc::new(123456789);
    let other_ptr: *const c_void = other.heap_ptr();
    assert_ne!(ptr1, other_ptr);
    assert_eq!(*other, *arc);


    assert_ne!(ptr1, std::ptr::null());
    assert_ne!(other_ptr, std::ptr::null());
}

#[test]
fn test_arc_heap_ptr_preserved_across_offset_conversion() {
    let arc: Arc<Vec<i32>> = Arc::new(vec![10, 20, 30, 40, 50]);
    let heap_before: *const c_void = arc.heap_ptr();
    let data_ptr: *const Vec<i32> = &*arc as *const Vec<i32>;


    let offset: OffsetArc<Vec<i32>> = Arc::into_raw_offset(arc);

    let sum_via_offset: i32 = offset.with_arc(|a| a.iter().sum());
    assert_eq!(sum_via_offset, 150);

    let len_via_offset: usize = offset.with_arc(|a| a.len());
    assert_eq!(len_via_offset, 5);


    let restored: Arc<Vec<i32>> = Arc::from_raw_offset(offset);

    let heap_after: *const c_void = restored.heap_ptr();
    assert_eq!(heap_before, heap_after);
    assert_eq!(&*restored as *const Vec<i32>, data_ptr);

    assert_eq!(restored.len(), 5);
    assert_eq!(restored[0], 10);
    assert_eq!(restored[4], 50);
    assert_eq!(restored.iter().sum::<i32>(), 150);
}

#[test]
fn test_arc_make_unique_on_sole_reference() {
    let mut arc: Arc<String> = Arc::new(String::from("hello"));
    let ptr_before: *const String = &*arc as *const String;
    let heap_before: *const c_void = arc.heap_ptr();


    {
        let unique: &mut UniqueArc<String> = Arc::make_unique(&mut arc);
        assert_eq!(unique.len(), 5);
        assert!(unique.starts_with("hello"));
        unique.push_str(", world");
        assert_eq!(unique.len(), 12);
        assert!(unique.contains("world"));
    }


    let ptr_after: *const String = &*arc as *const String;
    assert_eq!(ptr_before, ptr_after);
    assert_eq!(arc.heap_ptr(), heap_before);
    assert_eq!(&*arc, "hello, world");
    assert_eq!(arc.len(), 12);
}

#[test]
fn test_arc_make_unique_clones_on_shared_reference() {
    let mut arc: Arc<Vec<i32>> = Arc::new(vec![1, 2, 3, 4, 5]);
    let other = arc.clone();
    let other_ptr: *const Vec<i32> = &*other as *const Vec<i32>;
    let other_heap: *const c_void = other.heap_ptr();

    assert_eq!(&*arc as *const Vec<i32>, other_ptr);
    assert_eq!(arc.heap_ptr(), other_heap);


    {
        let unique: &mut UniqueArc<Vec<i32>> = Arc::make_unique(&mut arc);
        assert_eq!(unique.len(), 5);
        assert_eq!(unique[0], 1);
        unique.push(6);
        unique.push(7);
        assert_eq!(unique.len(), 7);
    }


    let ptr_after: *const Vec<i32> = &*arc as *const Vec<i32>;
    assert_ne!(ptr_after, other_ptr);
    assert_ne!(arc.heap_ptr(), other_heap);


    assert_eq!(arc.len(), 7);
    assert_eq!(arc[5], 6);
    assert_eq!(arc[6], 7);


    assert_eq!(other.len(), 5);
    assert_eq!(other[0], 1);
    assert_eq!(other[4], 5);
    assert_eq!(&*other as *const Vec<i32>, other_ptr);
    assert_eq!(other.heap_ptr(), other_heap);
}

#[test]
fn test_arc_unwrap_or_clone_unique_returns_inner() {
    let arc: Arc<String> = Arc::new(String::from("unwrap me"));


    let inner: String = Arc::unwrap_or_clone(arc);
    assert_eq!(inner, "unwrap me");
    assert_eq!(inner.len(), 9);
    assert_eq!(inner.as_bytes()[0], b'u');
    assert_eq!(inner.chars().count(), 9);
    assert_ne!(inner, "");
    assert_ne!(inner, "other");
    assert!(inner.contains("unwrap"));
    assert!(inner.ends_with("me"));
}

#[test]
fn test_arc_unwrap_or_clone_shared_forces_clone() {
    #[derive(Clone, Debug, PartialEq)]
    struct Data {
        name: String,
        values: Vec<u32>,
    }

    let arc: Arc<Data> = Arc::new(Data {
        name: String::from("shared"),
        values: vec![1, 2, 3],
    });
    let other = arc.clone();
    let shared_ptr: *const Data = &*other as *const Data;
    let shared_heap: *const c_void = other.heap_ptr();


    let owned: Data = Arc::unwrap_or_clone(arc);

    assert_eq!(owned.name, "shared");
    assert_eq!(owned.values, vec![1, 2, 3]);
    assert_eq!(owned.values.len(), 3);
    assert_eq!(owned.values[0], 1);
    assert_eq!(owned.values[2], 3);


    assert_ne!(&owned as *const Data, shared_ptr);


    assert_eq!(other.name, "shared");
    assert_eq!(other.values, vec![1, 2, 3]);
    assert_eq!(&*other as *const Data, shared_ptr);
    assert_eq!(other.heap_ptr(), shared_heap);
}

#[test]
fn test_arc_offset_conversion_chain() {
    let arc: Arc<u32> = Arc::new(99);
    let original_ptr: *const u32 = &*arc as *const u32;
    let original_heap: *const c_void = arc.heap_ptr();


    let offset1: OffsetArc<u32> = Arc::into_raw_offset(arc);
    let val1: u32 = offset1.with_arc(|a| **a);
    assert_eq!(val1, 99);

    let arc2: Arc<u32> = Arc::from_raw_offset(offset1);
    assert_eq!(*arc2, 99);
    assert_eq!(&*arc2 as *const u32, original_ptr);
    assert_eq!(arc2.heap_ptr(), original_heap);

    let offset2: OffsetArc<u32> = Arc::into_raw_offset(arc2);
    let val2: u32 = offset2.with_arc(|a| **a);
    assert_eq!(val2, 99);
    let ptr_mid: *const u32 = offset2.with_arc(|a| &**a as *const u32);
    assert_eq!(ptr_mid, original_ptr);

    let arc3: Arc<u32> = Arc::from_raw_offset(offset2);
    assert_eq!(*arc3, 99);
    assert_eq!(&*arc3 as *const u32, original_ptr);
    assert_eq!(arc3.heap_ptr(), original_heap);

    let cloned = arc3.clone();
    assert_eq!(*cloned, 99);
    assert_eq!(&*cloned as *const u32, original_ptr);
}

#[test]
fn test_arc_with_raw_offset_arc_nested_computation() {
    let arc: Arc<Vec<String>> = Arc::new(vec![
        String::from("alpha"),
        String::from("beta"),
        String::from("gamma"),
        String::from("delta"),
    ]);
    let original_ptr: *const Vec<String> = &*arc as *const Vec<String>;
    let original_heap: *const c_void = arc.heap_ptr();

    let total_chars: usize = arc.with_raw_offset_arc(|offset| {
        offset.with_arc(|a| a.iter().map(|s| s.len()).sum())
    });
    assert_eq!(total_chars, 5 + 4 + 5 + 5);

    let count: usize =
        arc.with_raw_offset_arc(|offset| offset.with_arc(|a| a.len()));
    assert_eq!(count, 4);

    let first: String =
        arc.with_raw_offset_arc(|offset| offset.with_arc(|a| a[0].clone()));
    assert_eq!(first, "alpha");

    let joined: String =
        arc.with_raw_offset_arc(|offset| offset.with_arc(|a| a.join(",")));
    assert_eq!(joined, "alpha,beta,gamma,delta");

    let has_gamma: bool = arc.with_raw_offset_arc(|offset| {
        offset.with_arc(|a| a.iter().any(|s| s == "gamma"))
    });
    assert_eq!(has_gamma, true);


    assert_eq!(&*arc as *const Vec<String>, original_ptr);
    assert_eq!(arc.heap_ptr(), original_heap);
    assert_eq!(arc.len(), 4);
    assert_eq!(arc[0], "alpha");
    assert_eq!(arc[3], "delta");
}

#[test]
fn test_arc_make_unique_then_unwrap_or_clone() {
    let mut arc: Arc<Vec<u8>> = Arc::new(vec![0u8; 16]);
    let heap_before: *const c_void = arc.heap_ptr();


    {
        let unique: &mut UniqueArc<Vec<u8>> = Arc::make_unique(&mut arc);
        assert_eq!(unique.len(), 16);
        for i in 0..16 {
            unique[i] = i as u8;
        }
        assert_eq!(unique[0], 0);
        assert_eq!(unique[15], 15);
    }


    assert_eq!(arc.heap_ptr(), heap_before);
    assert_eq!(arc.len(), 16);
    assert_eq!(arc[7], 7);


    let inner: Vec<u8> = Arc::unwrap_or_clone(arc);
    assert_eq!(inner.len(), 16);
    assert_eq!(inner[0], 0);
    assert_eq!(inner[15], 15);
    assert_eq!(inner.iter().map(|&b| b as u32).sum::<u32>(), (0..16u32).sum());
}