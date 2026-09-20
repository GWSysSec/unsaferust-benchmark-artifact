use deranged::*;

#[test]
fn test_new_unchecked_exercises_unsafe_wrapper() {
    type R = RangedI32<-100, 100>;

    // Safe construction
    let a = R::new(50).unwrap();
    let b = R::new(-100).unwrap();
    let c = R::new(100).unwrap();
    assert_eq!(a.get(), 50);
    assert_eq!(b.get(), -100);
    assert_eq!(c.get(), 100);

    // Unsafe construction with valid values
    let u1 = unsafe { R::new_unchecked(0) };
    let u2 = unsafe { R::new_unchecked(-50) };
    let u3 = unsafe { R::new_unchecked(99) };
    assert_eq!(u1.get(), 0);
    assert_eq!(u2.get(), -50);
    assert_eq!(u3.get(), 99);

    // Debug/display pass through the wrapper
    let dbg = format!("{:?}", a);
    let disp = format!("{}", a);
    assert!(!dbg.is_empty());
    assert_eq!(disp, "50");

    // Equality and ordering go through wrapper
    assert_eq!(a, R::new(50).unwrap());
    assert_ne!(a, b);
    assert!(b < c);
    assert!(a > b);
}

#[test]
fn test_unsafe_wrapper_via_various_ranged_types() {
    type U = RangedU8<10, 200>;
    type I = RangedI16<-1000, 1000>;

    let u_min = U::new(10).unwrap();
    let u_max = U::new(200).unwrap();
    let u_mid = unsafe { U::new_unchecked(100) };
    assert_eq!(u_min.get(), 10);
    assert_eq!(u_max.get(), 200);
    assert_eq!(u_mid.get(), 100);

    assert!(U::new(9).is_none());
    assert!(U::new(201).is_none());

    let i_a = unsafe { I::new_unchecked(-1000) };
    let i_b = unsafe { I::new_unchecked(1000) };
    let i_c = unsafe { I::new_unchecked(0) };
    assert_eq!(i_a.get(), -1000);
    assert_eq!(i_b.get(), 1000);
    assert_eq!(i_c.get(), 0);

    // Clone/Copy operate on the Unsafe inner
    let copy = i_c;
    let clone = i_c.clone();
    assert_eq!(copy.get(), 0);
    assert_eq!(clone.get(), i_c.get());

    // Hash set usage exercises wrapper hash impl
    use std::collections::HashSet;
    let mut set: HashSet<U> = HashSet::new();
    set.insert(u_min);
    set.insert(u_max);
    set.insert(u_mid);
    set.insert(unsafe { U::new_unchecked(100) });
    assert_eq!(set.len(), 3);
    assert!(set.contains(&u_min));
    assert!(set.contains(&u_max));
}

#[test]
fn test_unsafe_wrapper_boundary_and_conversion() {
    type R = RangedI8<-10, 10>;

    let vals: [i8; 5] = [-10, -5, 0, 5, 10];
    let mut total: i32 = 0;
    for &v in &vals {
        let r = unsafe { R::new_unchecked(v) };
        total += r.get() as i32;
        assert!(r.get() >= -10);
        assert!(r.get() <= 10);
    }
    assert_eq!(total, 0);

    // Expand/narrow through try_from
    type Big = RangedI32<-1000, 1000>;
    let r = unsafe { R::new_unchecked(7) };
    let big: Big = Big::try_from(r.get() as i32).unwrap();
    assert_eq!(big.get(), 7);

    let narrowed: Result<R, _> = R::try_from(big.get() as i8);
    assert!(narrowed.is_ok());
    assert_eq!(narrowed.unwrap().get(), 7);

    // Out-of-range try_from from wider type
    let too_big = Big::new(500).unwrap();
    let narrow_fail: Result<R, _> = R::try_from(too_big.get() as i8);
    assert!(narrow_fail.is_err());
}