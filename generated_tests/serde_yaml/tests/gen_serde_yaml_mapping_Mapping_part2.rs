use serde_yaml::Mapping;
use serde_yaml::Value;

#[test]
fn mapping_keys_and_values_views() {
    let mut m = Mapping::new();
    m.insert(Value::String("alpha".to_string()), Value::Number(1.into()));
    m.insert(Value::String("beta".to_string()), Value::Number(2.into()));
    m.insert(Value::String("gamma".to_string()), Value::Number(3.into()));


    let keys = m.keys();
    assert_eq!(keys.len(), 3);
    let collected_keys: Vec<&Value> = m.keys().collect();
    assert_eq!(collected_keys.len(), 3);
    assert_eq!(collected_keys[0], &Value::String("alpha".to_string()));
    assert_eq!(collected_keys[1], &Value::String("beta".to_string()));
    assert_eq!(collected_keys[2], &Value::String("gamma".to_string()));


    let values = m.values();
    assert_eq!(values.len(), 3);
    let collected_values: Vec<&Value> = m.values().collect();
    assert_eq!(collected_values.len(), 3);
    assert_eq!(collected_values[0], &Value::Number(1.into()));
    assert_eq!(collected_values[1], &Value::Number(2.into()));
    assert_eq!(collected_values[2], &Value::Number(3.into()));


    assert_eq!(m.keys().len(), 3);
    assert_eq!(m.values().len(), 3);
}

#[test]
fn mapping_keys_values_empty_and_single() {
    let empty = Mapping::new();
    assert_eq!(empty.keys().len(), 0);
    assert_eq!(empty.values().len(), 0);
    assert_eq!(empty.keys().count(), 0);
    assert_eq!(empty.values().count(), 0);

    let mut single = Mapping::new();
    single.insert(Value::Bool(true), Value::String("yes".to_string()));
    assert_eq!(single.keys().len(), 1);
    assert_eq!(single.values().len(), 1);

    let only_key: Vec<&Value> = single.keys().collect();
    let only_val: Vec<&Value> = single.values().collect();
    assert_eq!(only_key.len(), 1);
    assert_eq!(only_val.len(), 1);
    assert_eq!(only_key[0], &Value::Bool(true));
    assert_eq!(only_val[0], &Value::String("yes".to_string()));
}

#[test]
fn mapping_into_keys_consumes_and_yields_owned() {
    let mut m = Mapping::new();
    m.insert(Value::String("one".to_string()), Value::Number(11.into()));
    m.insert(Value::String("two".to_string()), Value::Number(22.into()));
    m.insert(Value::String("three".to_string()), Value::Number(33.into()));
    m.insert(Value::String("four".to_string()), Value::Number(44.into()));


    assert_eq!(m.keys().len(), 4);
    assert_eq!(m.values().len(), 4);

    let into_keys = m.into_keys();
    assert_eq!(into_keys.len(), 4);

    let owned_keys: Vec<Value> = into_keys.collect();
    assert_eq!(owned_keys.len(), 4);
    assert_eq!(owned_keys[0], Value::String("one".to_string()));
    assert_eq!(owned_keys[1], Value::String("two".to_string()));
    assert_eq!(owned_keys[2], Value::String("three".to_string()));
    assert_eq!(owned_keys[3], Value::String("four".to_string()));
    assert_ne!(owned_keys[0], owned_keys[1]);
}

#[test]
fn mapping_into_values_consumes_and_yields_owned() {
    let mut m = Mapping::new();
    m.insert(Value::Number(1.into()), Value::String("a".to_string()));
    m.insert(Value::Number(2.into()), Value::String("b".to_string()));
    m.insert(Value::Number(3.into()), Value::String("c".to_string()));

    assert_eq!(m.values().len(), 3);
    assert_eq!(m.keys().len(), 3);

    let into_values = m.into_values();
    assert_eq!(into_values.len(), 3);

    let owned_values: Vec<Value> = into_values.collect();
    assert_eq!(owned_values.len(), 3);
    assert_eq!(owned_values[0], Value::String("a".to_string()));
    assert_eq!(owned_values[1], Value::String("b".to_string()));
    assert_eq!(owned_values[2], Value::String("c".to_string()));
    assert_ne!(owned_values[0], owned_values[2]);
}

#[test]
fn mapping_views_after_parsing_yaml() {
    let yaml = "host: localhost\nport: 8080\nenabled: true\n";
    let value: Value = serde_yaml::from_str(yaml).expect("parse YAML");

    let m = match value {
        Value::Mapping(m) => m,
        other => panic!("expected mapping, got {:?}", other),
    };

    assert_eq!(m.keys().len(), 3);
    assert_eq!(m.values().len(), 3);

    let keys: Vec<&Value> = m.keys().collect();
    assert_eq!(keys[0], &Value::String("host".to_string()));
    assert_eq!(keys[1], &Value::String("port".to_string()));
    assert_eq!(keys[2], &Value::String("enabled".to_string()));

    let values: Vec<&Value> = m.values().collect();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0], &Value::String("localhost".to_string()));
    assert_eq!(values[2], &Value::Bool(true));


    let owned_keys: Vec<Value> = m.clone().into_keys().collect();
    let owned_values: Vec<Value> = m.into_values().collect();
    assert_eq!(owned_keys.len(), 3);
    assert_eq!(owned_values.len(), 3);
    assert_eq!(owned_keys[1], Value::String("port".to_string()));
    assert_eq!(owned_values[1], Value::Number(8080.into()));
}