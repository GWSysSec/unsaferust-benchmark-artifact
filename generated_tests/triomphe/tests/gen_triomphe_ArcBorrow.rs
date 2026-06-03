#![allow(unused_unsafe)]

extern crate triomphe;

use triomphe::{Arc, ArcBorrow};

#[test]
fn test_arc_borrow_from_ptr_scalar_roundtrip() {
    let arc: Arc<i32> = Arc::new(42);
    let ptr: *const i32 = &*arc as *const i32;

    let borrow = unsafe { ArcBorrow::from_ptr(ptr) };


    let got: &i32 = borrow.get();
    assert_eq!(*got, 42);
    assert_eq!(got as *const i32, ptr);


    let cloned: Arc<i32> = borrow.clone_arc();
    assert_eq!(*cloned, 42);
    assert_eq!(&*cloned as *const i32, ptr);
    assert_eq!(*cloned, *arc);


    let doubled: i32 = borrow.with_arc(|a| **a * 2);
    assert_eq!(doubled, 84);

    let ptr_in_cb: *const i32 = borrow.with_arc(|a| &**a as *const i32);
    assert_eq!(ptr_in_cb, ptr);


    assert_eq!(*arc, 42);
    assert_ne!(*arc, 0);
}

#[test]
fn test_arc_borrow_from_ptr_string_computations() {
    let arc: Arc<String> = Arc::new(String::from("hello world"));
    let ptr: *const String = &*arc as *const String;

    let borrow = unsafe { ArcBorrow::from_ptr(ptr) };


    assert_eq!(borrow.get(), "hello world");
    assert_eq!(borrow.get().len(), 11);
    assert_eq!(borrow.get().as_bytes()[0], b'h');


    let len: usize = borrow.with_arc(|a| a.len());
    assert_eq!(len, 11);

    let uppered: String = borrow.with_arc(|a| a.to_uppercase());
    assert_eq!(uppered, "HELLO WORLD");

    let contains_world: bool = borrow.with_arc(|a| a.contains("world"));
    assert_eq!(contains_world, true);

    let first: Option<char> = borrow.with_arc(|a| a.chars().next());
    assert_eq!(first, Some('h'));


    let cloned = borrow.clone_arc();
    assert_eq!(&*cloned, "hello world");
    assert_eq!(cloned.len(), 11);
    assert_eq!(&*cloned as *const String, ptr);
}

#[test]
fn test_arc_borrow_multiple_borrows_share_identity() {
    let arc: Arc<i64> = Arc::new(1_000_000);
    let ptr: *const i64 = &*arc as *const i64;

    let b1 = unsafe { ArcBorrow::from_ptr(ptr) };
    let b2 = unsafe { ArcBorrow::from_ptr(ptr) };
    let b3 = unsafe { ArcBorrow::from_ptr(ptr) };


    assert_eq!(*b1.get(), 1_000_000);
    assert_eq!(*b2.get(), 1_000_000);
    assert_eq!(*b3.get(), 1_000_000);


    assert_eq!(b1.get() as *const i64, ptr);
    assert_eq!(b2.get() as *const i64, ptr);
    assert_eq!(b3.get() as *const i64, ptr);


    let a1 = b1.clone_arc();
    let a2 = b2.clone_arc();
    let a3 = b3.clone_arc();

    assert_eq!(&*a1 as *const i64, ptr);
    assert_eq!(&*a2 as *const i64, ptr);
    assert_eq!(&*a3 as *const i64, ptr);


    let p1: *const i64 = b1.with_arc(|a| &**a as *const i64);
    let p2: *const i64 = b2.with_arc(|a| &**a as *const i64);
    assert_eq!(p1, p2);
    assert_eq!(p1, ptr);


    let total: i64 =
        b1.with_arc(|a| **a) + b2.with_arc(|a| **a) + b3.with_arc(|a| **a);
    assert_eq!(total, 3_000_000);
}

#[test]
fn test_arc_borrow_over_vec_workflow() {
    let arc: Arc<Vec<u32>> = Arc::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let ptr: *const Vec<u32> = &*arc as *const Vec<u32>;

    let borrow = unsafe { ArcBorrow::from_ptr(ptr) };


    let v_ref: &Vec<u32> = borrow.get();
    assert_eq!(v_ref.len(), 10);
    assert_eq!(v_ref[0], 1);
    assert_eq!(v_ref[9], 10);
    assert_eq!(v_ref.iter().copied().sum::<u32>(), 55);


    let sum: u32 = borrow.with_arc(|a| a.iter().sum());
    assert_eq!(sum, 55);

    let max: u32 = borrow.with_arc(|a| *a.iter().max().unwrap());
    assert_eq!(max, 10);

    let even_count: usize = borrow.with_arc(|a| a.iter().filter(|n| **n % 2 == 0).count());
    assert_eq!(even_count, 5);

    let product_first_three: u32 = borrow.with_arc(|a| a.iter().take(3).product());
    assert_eq!(product_first_three, 6);


    let cloned: Arc<Vec<u32>> = borrow.clone_arc();
    assert_eq!(cloned.len(), 10);
    assert_eq!(&*cloned as *const Vec<u32>, ptr);
    assert_eq!(cloned[4], 5);
}

#[test]
fn test_arc_borrow_struct_and_reborrow_chain() {
    #[derive(Debug, PartialEq)]
    struct Point {
        x: f64,
        y: f64,
        label: String,
    }

    let arc: Arc<Point> = Arc::new(Point {
        x: 3.0,
        y: 4.0,
        label: String::from("origin"),
    });
    let ptr: *const Point = &*arc as *const Point;

    let borrow = unsafe { ArcBorrow::from_ptr(ptr) };


    assert_eq!(borrow.get().x, 3.0);
    assert_eq!(borrow.get().y, 4.0);
    assert_eq!(borrow.get().label, "origin");
    assert_eq!(borrow.get().label.len(), 6);


    let dist: f64 = borrow.with_arc(|a| (a.x * a.x + a.y * a.y).sqrt());
    assert_eq!(dist, 5.0);

    let label_upper: String = borrow.with_arc(|a| a.label.to_uppercase());
    assert_eq!(label_upper, "ORIGIN");


    let a2: Arc<Point> = borrow.clone_arc();
    let ptr2: *const Point = &*a2 as *const Point;
    assert_eq!(ptr2, ptr);

    let borrow2 = unsafe { ArcBorrow::from_ptr(ptr2) };
    assert_eq!(borrow2.get().x, 3.0);
    assert_eq!(borrow2.get().y, 4.0);
    assert_eq!(borrow2.get() as *const Point, ptr);


    let a3 = borrow2.clone_arc();
    assert_eq!(&*a3 as *const Point, ptr);
    assert_eq!(a3.label, "origin");


    assert_eq!(arc.x, 3.0);
    assert_eq!(arc.label, "origin");
}