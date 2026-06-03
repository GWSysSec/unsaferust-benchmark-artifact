use serde_yaml::{Number, Value};

#[test]
fn test_number_integer_classification() {
    let v: Value = serde_yaml::from_str("42").unwrap();
    let n = match &v {
        Value::Number(n) => n.clone(),
        _ => panic!("expected number"),
    };
    assert_eq!(n.is_i64(), true);
    assert_eq!(n.is_u64(), true);
    assert_eq!(n.as_i64(), Some(42));
    assert_eq!(n.as_u64(), Some(42));
    assert_eq!(n.is_nan(), false);
    assert_eq!(n.is_infinite(), false);
    assert_eq!(n.is_finite(), true);
}

#[test]
fn test_number_negative_integer() {
    let v: Value = serde_yaml::from_str("-7").unwrap();
    let n = match &v {
        Value::Number(n) => n.clone(),
        _ => panic!("expected number"),
    };
    assert_eq!(n.is_i64(), true);
    assert_eq!(n.is_u64(), false);
    assert_eq!(n.as_i64(), Some(-7));
    assert_eq!(n.as_u64(), None);
    assert_eq!(n.is_nan(), false);
    assert_eq!(n.is_infinite(), false);
    assert_eq!(n.is_finite(), true);
}

#[test]
fn test_number_large_unsigned() {
    let big: u64 = u64::MAX;
    let yaml = format!("{}", big);
    let v: Value = serde_yaml::from_str(&yaml).unwrap();
    let n = match &v {
        Value::Number(n) => n.clone(),
        _ => panic!("expected number"),
    };
    assert_eq!(n.is_u64(), true);
    assert_eq!(n.is_i64(), false);
    assert_eq!(n.as_u64(), Some(u64::MAX));
    assert_eq!(n.as_i64(), None);
    assert_eq!(n.is_finite(), true);
    assert_eq!(n.is_nan(), false);
    assert_eq!(n.is_infinite(), false);
}

#[test]
fn test_number_float_finite() {
    let v: Value = serde_yaml::from_str("3.14").unwrap();
    let n = match &v {
        Value::Number(n) => n.clone(),
        _ => panic!("expected number"),
    };
    assert_eq!(n.is_i64(), false);
    assert_eq!(n.is_u64(), false);
    assert_eq!(n.as_i64(), None);
    assert_eq!(n.as_u64(), None);
    assert_eq!(n.is_finite(), true);
    assert_eq!(n.is_nan(), false);
    assert_eq!(n.is_infinite(), false);
}

#[test]
fn test_number_nan_and_infinity() {
    let v: Value = serde_yaml::from_str(".nan").unwrap();
    let n = match &v {
        Value::Number(n) => n.clone(),
        _ => panic!("expected number, got {:?}", v),
    };
    assert_eq!(n.is_nan(), true);
    assert_eq!(n.is_finite(), false);
    assert_eq!(n.is_infinite(), false);
    assert_eq!(n.is_i64(), false);
    assert_eq!(n.is_u64(), false);

    let v2: Value = serde_yaml::from_str(".inf").unwrap();
    let n2 = match &v2 {
        Value::Number(n) => n.clone(),
        _ => panic!("expected number"),
    };
    assert_eq!(n2.is_infinite(), true);
    assert_eq!(n2.is_finite(), false);
    assert_eq!(n2.is_nan(), false);

    let v3: Value = serde_yaml::from_str("-.inf").unwrap();
    let n3 = match &v3 {
        Value::Number(n) => n.clone(),
        _ => panic!("expected number"),
    };
    assert_eq!(n3.is_infinite(), true);
    assert_eq!(n3.is_finite(), false);
    assert_eq!(n3.as_i64(), None);
}

#[test]
fn test_number_via_to_value_roundtrip() {
    let n_i: Number = serde_yaml::from_value(serde_yaml::to_value(-100i64).unwrap()).unwrap();
    assert_eq!(n_i.is_i64(), true);
    assert_eq!(n_i.as_i64(), Some(-100));
    assert_eq!(n_i.is_u64(), false);

    let n_u: Number = serde_yaml::from_value(serde_yaml::to_value(200u64).unwrap()).unwrap();
    assert_eq!(n_u.is_u64(), true);
    assert_eq!(n_u.as_u64(), Some(200));
    assert_eq!(n_u.is_i64(), true);

    let n_f: Number = serde_yaml::from_value(serde_yaml::to_value(2.5f64).unwrap()).unwrap();
    assert_eq!(n_f.is_finite(), true);
    assert_eq!(n_f.is_nan(), false);
    assert_eq!(n_f.as_i64(), None);
}