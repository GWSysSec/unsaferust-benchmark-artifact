extern crate triomphe;

use triomphe::{Arc, ArcUnion, ArcUnionBorrow};

#[test]
fn test_arc_union_from_first_basic_workflow() {
    let a: Arc<i32> = Arc::new(42);
    let ptr_a: *const i32 = &*a as *const i32;

    let u: ArcUnion<i32, String> = ArcUnion::from_first(a.clone());


    assert_eq!(u.is_first(), true);
    assert_eq!(u.is_second(), false);


    assert!(u.as_second().is_none());


    let borrowed = u.as_first().expect("as_first must return Some for first variant");
    assert_eq!(*borrowed.get(), 42);
    assert_eq!(borrowed.get() as *const i32, ptr_a);


    let a2: Arc<i32> = borrowed.clone_arc();
    assert_eq!(*a2, 42);
    assert_eq!(&*a2 as *const i32, ptr_a);


    assert_eq!(*a, 42);
}

#[test]
fn test_arc_union_from_second_basic_workflow() {
    let s: Arc<String> = Arc::new(String::from("triomphe"));
    let ptr_s: *const String = &*s as *const String;

    let u: ArcUnion<i32, String> = ArcUnion::from_second(s.clone());


    assert_eq!(u.is_second(), true);
    assert_eq!(u.is_first(), false);


    assert!(u.as_first().is_none());


    let borrowed = u.as_second().expect("as_second must return Some for second variant");
    assert_eq!(borrowed.get(), "triomphe");
    assert_eq!(borrowed.get().len(), 8);
    assert_eq!(borrowed.get() as *const String, ptr_s);


    let s2: Arc<String> = borrowed.clone_arc();
    assert_eq!(&*s2, "triomphe");
    assert_eq!(&*s2 as *const String, ptr_s);


    assert_eq!(&*s, "triomphe");
}

#[test]
fn test_arc_union_ptr_eq_identity_and_inequality() {
    let a1: Arc<i32> = Arc::new(100);
    let a2: Arc<i32> = Arc::new(100);
    let b1: Arc<String> = Arc::new(String::from("abc"));

    let u_a1_first: ArcUnion<i32, String> = ArcUnion::from_first(a1.clone());
    let u_a1_first_again: ArcUnion<i32, String> = ArcUnion::from_first(a1.clone());
    let u_a2_first: ArcUnion<i32, String> = ArcUnion::from_first(a2.clone());
    let u_b1_second: ArcUnion<i32, String> = ArcUnion::from_second(b1.clone());
    let u_b1_second_again: ArcUnion<i32, String> = ArcUnion::from_second(b1.clone());


    assert_eq!(ArcUnion::ptr_eq(&u_a1_first, &u_a1_first_again), true);
    assert_eq!(ArcUnion::ptr_eq(&u_b1_second, &u_b1_second_again), true);


    assert_eq!(ArcUnion::ptr_eq(&u_a1_first, &u_a2_first), false);


    assert_eq!(ArcUnion::ptr_eq(&u_a1_first, &u_b1_second), false);
    assert_eq!(ArcUnion::ptr_eq(&u_a2_first, &u_b1_second_again), false);


    assert_eq!(u_a1_first.is_first(), true);
    assert_eq!(u_b1_second.is_second(), true);
}

#[test]
fn test_arc_union_borrow_strong_count_progression() {
    let a: Arc<Vec<u32>> = Arc::new(vec![10, 20, 30, 40, 50]);


    let u1: ArcUnion<Vec<u32>, String> = ArcUnion::from_first(a.clone());

    let borrow1 = u1.borrow();
    let count_after_u1 = ArcUnionBorrowCount::count(&borrow1);
    assert_eq!(count_after_u1, 2);


    let u2: ArcUnion<Vec<u32>, String> = ArcUnion::from_first(a.clone());
    let borrow2 = u2.borrow();
    let count_after_u2 = ArcUnionBorrowCount::count(&borrow2);
    assert_eq!(count_after_u2, 3);


    assert_eq!(ArcUnion::ptr_eq(&u1, &u2), true);


    let ab1 = u1.as_first().expect("u1 first");
    let ab2 = u2.as_first().expect("u2 first");
    assert_eq!(ab1.get().len(), 5);
    assert_eq!(ab2.get().iter().sum::<u32>(), 150);
    assert!(u1.as_second().is_none());
    assert!(u2.as_second().is_none());


    drop(u2);
    let borrow3 = u1.borrow();
    let count_after_drop = ArcUnionBorrowCount::count(&borrow3);
    assert_eq!(count_after_drop, 2);


    assert_eq!(a.len(), 5);
    assert_eq!(a[0], 10);
    assert_eq!(a[4], 50);
}




struct ArcUnionBorrowCount;
impl ArcUnionBorrowCount {
    fn count<A, B>(b: &ArcUnionBorrow<A, B>) -> usize {
        ArcUnionBorrow::strong_count(b)
    }
}

#[test]
fn test_arc_union_mixed_variants_and_borrow_queries() {
    let first_arc: Arc<u64> = Arc::new(0xDEAD_BEEF);
    let second_arc: Arc<Vec<u8>> = Arc::new(vec![1u8, 2, 3, 4]);

    let ptr_first: *const u64 = &*first_arc as *const u64;
    let ptr_second: *const Vec<u8> = &*second_arc as *const Vec<u8>;

    let uf: ArcUnion<u64, Vec<u8>> = ArcUnion::from_first(first_arc.clone());
    let us: ArcUnion<u64, Vec<u8>> = ArcUnion::from_second(second_arc.clone());


    assert_eq!(uf.is_first(), true);
    assert_eq!(uf.is_second(), false);
    assert_eq!(us.is_first(), false);
    assert_eq!(us.is_second(), true);


    let bf = uf.borrow();
    let bs = us.borrow();
    assert_eq!(ArcUnionBorrowCount::count(&bf), 2);
    assert_eq!(ArcUnionBorrowCount::count(&bs), 2);


    let af = uf.as_first().expect("uf first");
    assert_eq!(*af.get(), 0xDEAD_BEEF);
    assert_eq!(af.get() as *const u64, ptr_first);
    assert!(uf.as_second().is_none());

    let as_ = us.as_second().expect("us second");
    assert_eq!(as_.get().len(), 4);
    assert_eq!(as_.get()[0], 1);
    assert_eq!(as_.get()[3], 4);
    assert_eq!(as_.get() as *const Vec<u8>, ptr_second);
    assert!(us.as_first().is_none());


    assert_eq!(ArcUnion::ptr_eq(&uf, &us), false);


    assert_eq!(ArcUnion::ptr_eq(&uf, &uf), true);
    assert_eq!(ArcUnion::ptr_eq(&us, &us), true);
}

#[test]
fn test_arc_union_with_strings_and_vec_roundtrip() {

    type U = ArcUnion<String, Vec<i32>>;

    let s_arc: Arc<String> = Arc::new(String::from("hello"));
    let v_arc: Arc<Vec<i32>> = Arc::new(vec![7, 8, 9]);

    let u_s: U = ArcUnion::from_first(s_arc.clone());
    let u_v: U = ArcUnion::from_second(v_arc.clone());


    assert_eq!(u_s.is_first(), true);
    assert_eq!(u_v.is_second(), true);
    assert_eq!(u_s.is_second(), false);
    assert_eq!(u_v.is_first(), false);


    let s_borrow = u_s.as_first().expect("u_s first");
    assert_eq!(s_borrow.get(), "hello");
    assert_eq!(s_borrow.get().len(), 5);
    assert_eq!(s_borrow.get().chars().next(), Some('h'));

    let v_borrow = u_v.as_second().expect("u_v second");
    assert_eq!(v_borrow.get().len(), 3);
    assert_eq!(v_borrow.get().iter().sum::<i32>(), 24);
    assert_eq!(v_borrow.get()[1], 8);


    let s_back: Arc<String> = s_borrow.clone_arc();
    assert_eq!(&*s_back, "hello");
    assert_eq!(&*s_back as *const String, &*s_arc as *const String);

    let v_back: Arc<Vec<i32>> = v_borrow.clone_arc();
    assert_eq!(v_back.len(), 3);
    assert_eq!(&*v_back as *const Vec<i32>, &*v_arc as *const Vec<i32>);




    let b_s = u_s.borrow();
    let b_v = u_v.borrow();
    assert_eq!(ArcUnionBorrowCount::count(&b_s), 3);
    assert_eq!(ArcUnionBorrowCount::count(&b_v), 3);


    drop(s_back);
    drop(v_back);
    let b_s2 = u_s.borrow();
    let b_v2 = u_v.borrow();
    assert_eq!(ArcUnionBorrowCount::count(&b_s2), 2);
    assert_eq!(ArcUnionBorrowCount::count(&b_v2), 2);
}