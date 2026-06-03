use ordered_float::NotNan;

#[test]
fn test_unchecked_new_basic_values() {
    let a = unsafe { NotNan::unchecked_new(1.5_f64) };
    let b = unsafe { NotNan::unchecked_new(-3.25_f64) };
    let c = unsafe { NotNan::unchecked_new(0.0_f64) };
    let d = unsafe { NotNan::unchecked_new(f64::INFINITY) };
    let e = unsafe { NotNan::unchecked_new(f64::NEG_INFINITY) };

    assert_eq!(a.into_inner(), 1.5_f64);
    assert_eq!(b.into_inner(), -3.25_f64);
    assert_eq!(c.into_inner(), 0.0_f64);
    assert_eq!(d.into_inner(), f64::INFINITY);
    assert_eq!(e.into_inner(), f64::NEG_INFINITY);

    let checked = NotNan::new(1.5_f64).unwrap();
    assert_eq!(checked, a);
    assert_ne!(a, b);
    assert!(b < a);
    assert!(a < d);
    assert!(e < b);
}

#[test]
fn test_unchecked_new_matches_checked_new() {
    let values = [0.0_f64, 1.0, -1.0, 1e308, -1e308, 1e-308, std::f64::consts::PI];
    let mut sum_unchecked = 0.0_f64;
    let mut sum_checked = 0.0_f64;
    let mut count = 0;

    for &v in &values {
        let u = unsafe { NotNan::unchecked_new(v) };
        let c = NotNan::new(v).expect("not nan");
        assert_eq!(u, c);
        assert_eq!(u.into_inner(), c.into_inner());
        assert_eq!(u.into_inner(), v);
        sum_unchecked += u.into_inner();
        sum_checked += c.into_inner();
        count += 1;
    }

    assert_eq!(count, 7);
    assert_eq!(sum_unchecked, sum_checked);
    assert_ne!(sum_checked, 0.0);
}

#[test]
fn test_as_f32_conversion() {
    let original = NotNan::new(1.5_f64).unwrap();
    let converted = original.as_f32();
    assert_eq!(converted.into_inner(), 1.5_f32);
    assert_eq!(original.into_inner(), 1.5_f64);

    let neg = NotNan::new(-2.5_f64).unwrap();
    let neg_f32 = neg.as_f32();
    assert_eq!(neg_f32.into_inner(), -2.5_f32);

    let zero = NotNan::new(0.0_f64).unwrap();
    let zero_f32 = zero.as_f32();
    assert_eq!(zero_f32.into_inner(), 0.0_f32);

    let inf = NotNan::new(f64::INFINITY).unwrap();
    let inf_f32 = inf.as_f32();
    assert_eq!(inf_f32.into_inner(), f32::INFINITY);

    assert!(neg_f32 < zero_f32);
    assert!(zero_f32 < converted);
    assert!(converted < inf_f32);
}

#[test]
fn test_as_f32_overflow_to_infinity() {
    let huge = NotNan::new(1.0e40_f64).unwrap();
    let huge_f32 = huge.as_f32();
    assert_eq!(huge_f32.into_inner(), f32::INFINITY);

    let tiny = NotNan::new(-1.0e40_f64).unwrap();
    let tiny_f32 = tiny.as_f32();
    assert_eq!(tiny_f32.into_inner(), f32::NEG_INFINITY);

    let underflow = NotNan::new(1.0e-50_f64).unwrap();
    let underflow_f32 = underflow.as_f32();
    assert_eq!(underflow_f32.into_inner(), 0.0_f32);

    assert!(tiny_f32 < underflow_f32);
    assert!(underflow_f32 < huge_f32);
    assert_ne!(huge_f32, tiny_f32);
    assert_eq!(huge.into_inner(), 1.0e40_f64);
    assert_eq!(tiny.into_inner(), -1.0e40_f64);
}

#[test]
fn test_unchecked_new_then_as_f32_workflow() {
    let raw_values = [0.0_f64, 1.25, -1.25, 100.5, -100.5];
    let mut collected: Vec<NotNan<f32>> = Vec::new();

    for &v in &raw_values {
        let n = unsafe { NotNan::unchecked_new(v) };
        assert_eq!(n.into_inner(), v);
        collected.push(n.as_f32());
    }

    assert_eq!(collected.len(), 5);
    assert_eq!(collected[0].into_inner(), 0.0_f32);
    assert_eq!(collected[1].into_inner(), 1.25_f32);
    assert_eq!(collected[2].into_inner(), -1.25_f32);
    assert_eq!(collected[3].into_inner(), 100.5_f32);
    assert_eq!(collected[4].into_inner(), -100.5_f32);

    collected.sort();
    assert_eq!(collected[0].into_inner(), -100.5_f32);
    assert_eq!(collected[1].into_inner(), -1.25_f32);
    assert_eq!(collected[2].into_inner(), 0.0_f32);
    assert_eq!(collected[3].into_inner(), 1.25_f32);
    assert_eq!(collected[4].into_inner(), 100.5_f32);
}