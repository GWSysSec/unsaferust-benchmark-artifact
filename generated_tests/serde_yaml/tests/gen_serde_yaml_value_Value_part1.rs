use serde_yaml::Value;

#[test]
fn test_value_type_checks() {
    let yaml = r#"
name: Alice
age: 30
height: 5.5
active: true
nothing: null
tags:
  - admin
  - user
meta:
  level: 9
  score: -3
"#;
    let v: Value = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(v.is_mapping(), true);
    assert_eq!(v.is_sequence(), false);
    assert_eq!(v.is_null(), false);
    assert_eq!(v.is_bool(), false);
    assert_eq!(v.is_number(), false);

    let name = v.get("name").unwrap();
    assert_eq!(name.as_str(), Some("Alice"));
    assert_eq!(name.is_bool(), false);

    let age = v.get("age").unwrap();
    assert_eq!(age.is_number(), true);
    assert_eq!(age.is_u64(), true);
    assert_eq!(age.is_i64(), true);
    assert_eq!(age.as_u64(), Some(30));
    assert_eq!(age.as_i64(), Some(30));

    let height = v.get("height").unwrap();
    assert_eq!(height.is_number(), true);
    assert_eq!(height.is_i64(), false);
    assert_eq!(height.is_u64(), false);

    let active = v.get("active").unwrap();
    assert_eq!(active.is_bool(), true);
    assert_eq!(active.as_bool(), Some(true));

    let nothing = v.get("nothing").unwrap();
    assert_eq!(nothing.is_null(), true);
    assert_eq!(nothing.as_null(), Some(()));
    assert_eq!(nothing.as_bool(), None);

    let tags = v.get("tags").unwrap();
    assert_eq!(tags.is_sequence(), true);
    let seq = tags.as_sequence().unwrap();
    assert_eq!(seq.len(), 2);
    assert_eq!(seq[0].as_str(), Some("admin"));
    assert_eq!(seq[1].as_str(), Some("user"));

    let missing = v.get("missing_key");
    assert!(missing.is_none());

    let nested = v.get("meta").unwrap().get("level").unwrap();
    assert_eq!(nested.as_u64(), Some(9));

    let neg = v.get("meta").unwrap().get("score").unwrap();
    assert_eq!(neg.is_i64(), true);
    assert_eq!(neg.is_u64(), false);
    assert_eq!(neg.as_i64(), Some(-3));
    assert_eq!(neg.as_u64(), None);
}

#[test]
fn test_value_get_mut_and_sequence_mut() {
    let yaml = r#"
items:
  - 1
  - 2
  - 3
flags:
  enabled: false
"#;
    let mut v: Value = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(v.is_mapping(), true);

    {
        let items = v.get_mut("items").unwrap();
        assert_eq!(items.is_sequence(), true);
        let seq = items.as_sequence_mut().unwrap();
        assert_eq!(seq.len(), 3);
        seq.push(Value::from(4i64));
        seq.push(Value::from(5i64));
        assert_eq!(seq.len(), 5);
    }

    let items_again = v.get("items").unwrap().as_sequence().unwrap();
    assert_eq!(items_again.len(), 5);
    assert_eq!(items_again[3].as_i64(), Some(4));
    assert_eq!(items_again[4].as_i64(), Some(5));

    {
        let enabled = v.get_mut("flags").unwrap().get_mut("enabled").unwrap();
        assert_eq!(enabled.as_bool(), Some(false));
        *enabled = Value::Bool(true);
    }

    let enabled_now = v.get("flags").unwrap().get("enabled").unwrap();
    assert_eq!(enabled_now.as_bool(), Some(true));
    assert_eq!(enabled_now.is_bool(), true);


    assert!(v.get("nope").is_none());
    assert!(v.get_mut("nope").is_none());


    let items_val = v.get("items").unwrap();
    let first = items_val.get(0).unwrap();
    assert_eq!(first.as_i64(), Some(1));
    let oob = items_val.get(99);
    assert!(oob.is_none());
}

#[test]
fn test_value_null_and_bool_roundtrip() {
    let null_val: Value = serde_yaml::from_str("null").unwrap();
    assert_eq!(null_val.is_null(), true);
    assert_eq!(null_val.as_null(), Some(()));
    assert_eq!(null_val.is_bool(), false);
    assert_eq!(null_val.is_number(), false);
    assert_eq!(null_val.is_sequence(), false);
    assert_eq!(null_val.is_mapping(), false);

    let true_val: Value = serde_yaml::from_str("true").unwrap();
    assert_eq!(true_val.is_bool(), true);
    assert_eq!(true_val.as_bool(), Some(true));
    assert_eq!(true_val.as_null(), None);

    let false_val: Value = serde_yaml::from_str("false").unwrap();
    assert_eq!(false_val.as_bool(), Some(false));
    assert_eq!(false_val.is_null(), false);

    let big: Value = serde_yaml::from_str("18446744073709551610").unwrap();
    assert_eq!(big.is_u64(), true);
    assert_eq!(big.is_i64(), false);
    assert_eq!(big.as_u64(), Some(18446744073709551610));
    assert_eq!(big.as_i64(), None);
    assert_eq!(big.is_number(), true);
}