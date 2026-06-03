use ordered_float::NotNan;

#[test]
fn unchecked_new_values_can_be_sorted_and_unwrapped() {
    let mut values: Vec<NotNan<f64>> = vec![
        unsafe { NotNan::unchecked_new(3.5_f64) },
        unsafe { NotNan::unchecked_new(-2.0_f64) },
        unsafe { NotNan::unchecked_new(0.0_f64) },
        unsafe { NotNan::unchecked_new(f64::INFINITY) },
        unsafe { NotNan::unchecked_new(-0.0_f64) },
    ];

    values.sort();

    let unwrapped: Vec<f64> = values.into_iter().map(NotNan::into_inner).collect();

    assert_eq!(unwrapped.len(), 5);
    assert_eq!(unwrapped[0], -2.0);
    assert_eq!(unwrapped[1], 0.0);
    assert_eq!(unwrapped[2], -0.0);
    assert_eq!(unwrapped[3], 3.5);
    assert_eq!(unwrapped[4], f64::INFINITY);
    assert!(NotNan::new(f64::NAN).is_err());
}

#[test]
fn unchecked_new_preserves_non_nan_edge_values() {
    let positive_infinity = unsafe { NotNan::unchecked_new(f64::INFINITY) };
    let negative_infinity = unsafe { NotNan::unchecked_new(f64::NEG_INFINITY) };
    let negative_zero = unsafe { NotNan::unchecked_new(-0.0_f64) };

    assert!(positive_infinity.into_inner().is_infinite());
    assert!(negative_infinity.into_inner().is_sign_negative());
    assert!(negative_infinity < unsafe { NotNan::unchecked_new(-1.0_f64) });
    assert_eq!(negative_zero.into_inner().to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn as_f32_narrows_f64_not_nan_values_in_a_workflow() {
    let source_values: Vec<NotNan<f64>> = vec![
        unsafe { NotNan::unchecked_new(42.25_f64) },
        unsafe { NotNan::unchecked_new(-13.5_f64) },
        unsafe { NotNan::unchecked_new(0.125_f64) },
    ];

    let narrowed: Vec<NotNan<f32>> = source_values
        .into_iter()
        .map(NotNan::<f64>::as_f32)
        .collect();

    let narrowed_inner: Vec<f32> = narrowed.into_iter().map(NotNan::into_inner).collect();

    assert_eq!(narrowed_inner, vec![42.25_f32, -13.5_f32, 0.125_f32]);
    assert!(narrowed_inner.iter().all(|value| !value.is_nan()));
}

#[test]
fn as_f32_uses_f32_rounding_and_handles_overflow_to_infinity() {
    let exactly_representable = unsafe { NotNan::unchecked_new(1.5_f64) }.as_f32();
    assert_eq!(exactly_representable.into_inner(), 1.5_f32);

    let halfway_between_one_and_next_f32 = 1.0_f64 + 2_f64.powi(-24);
    let rounded_ties_to_even = unsafe {
        NotNan::unchecked_new(halfway_between_one_and_next_f32)
    }
    .as_f32();

    assert_eq!(rounded_ties_to_even.into_inner(), 1.0_f32);

    let overflow = unsafe { NotNan::unchecked_new(f64::MAX) }.as_f32();
    let overflow_inner = overflow.into_inner();

    assert!(overflow_inner.is_infinite());
    assert!(overflow_inner.is_sign_positive());

    let negative_overflow = unsafe { NotNan::unchecked_new(-f64::MAX) }.as_f32();
    let negative_overflow_inner = negative_overflow.into_inner();

    assert!(negative_overflow_inner.is_infinite());
    assert!(negative_overflow_inner.is_sign_negative());
}