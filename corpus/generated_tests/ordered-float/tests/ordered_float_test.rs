use ordered_float::{NotNan, OrderedFloat};

#[test]
fn not_nan_as_f32_is_callable_from_integration_tests() {
    let finite = NotNan::<f64>::new(12.5).expect("finite values should construct NotNan");
    let narrowed = NotNan::as_f32(finite);

    assert_eq!(narrowed.into_inner(), 12.5_f32);
    assert!(NotNan::<f64>::new(f64::NAN).is_err());
}

#[test]
fn as_f32_supports_sorting_and_unwrapping_workflow() {
    let source_values = vec![
        NotNan::<f64>::new(8.25).unwrap(),
        NotNan::<f64>::new(-3.5).unwrap(),
        NotNan::<f64>::new(0.0).unwrap(),
        NotNan::<f64>::new(f64::INFINITY).unwrap(),
        NotNan::<f64>::new(-0.0).unwrap(),
    ];

    let mut narrowed_values: Vec<NotNan<f32>> =
        source_values.into_iter().map(NotNan::as_f32).collect();

    narrowed_values.sort();

    let unwrapped: Vec<f32> = narrowed_values
        .into_iter()
        .map(NotNan::<f32>::into_inner)
        .collect();

    assert_eq!(unwrapped.len(), 5);
    assert_eq!(unwrapped[0], -3.5_f32);
    assert_eq!(unwrapped[1], 0.0_f32);
    assert_eq!(unwrapped[2].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(unwrapped[3], 8.25_f32);
    assert_eq!(unwrapped[4], f32::INFINITY);
}

#[test]
fn as_f32_preserves_expected_edge_value_semantics() {
    let positive_infinity = NotNan::<f64>::new(f64::INFINITY).unwrap();
    let negative_infinity = NotNan::<f64>::new(f64::NEG_INFINITY).unwrap();
    let negative_zero = NotNan::<f64>::new(-0.0).unwrap();

    let narrowed_positive_infinity = NotNan::as_f32(positive_infinity);
    let narrowed_negative_infinity = NotNan::as_f32(negative_infinity);
    let narrowed_negative_zero = NotNan::as_f32(negative_zero);

    assert_eq!(narrowed_positive_infinity.into_inner(), f32::INFINITY);
    assert_eq!(narrowed_negative_infinity.into_inner(), f32::NEG_INFINITY);
    assert!(narrowed_negative_infinity < NotNan::<f32>::new(-1.0).unwrap());
    assert_eq!(
        narrowed_negative_zero.into_inner().to_bits(),
        (-0.0_f32).to_bits()
    );
}

#[test]
fn ordered_float_and_not_nan_can_be_used_together_in_a_realistic_pipeline() {
    let raw_readings = [3.0_f64, -1.25, 2.5, 3.0, f64::NAN, 0.5];

    let mut valid_readings: Vec<NotNan<f32>> = raw_readings
        .iter()
        .copied()
        .filter_map(|value| NotNan::<f64>::new(value).ok())
        .map(NotNan::as_f32)
        .collect();

    valid_readings.sort();

    let sorted_valid: Vec<f32> = valid_readings
        .iter()
        .copied()
        .map(NotNan::<f32>::into_inner)
        .collect();

    assert_eq!(sorted_valid, vec![-1.25_f32, 0.5, 2.5, 3.0, 3.0]);

    let ordered_extremes = [
        OrderedFloat(f64::NAN),
        OrderedFloat(f64::NEG_INFINITY),
        OrderedFloat(42.0),
    ];

    assert!(ordered_extremes[0].into_inner().is_nan());
    assert_eq!(ordered_extremes[1].into_inner(), f64::NEG_INFINITY);
    assert_eq!(ordered_extremes[2].into_inner(), 42.0);
}