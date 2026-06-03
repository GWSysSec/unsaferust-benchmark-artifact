use serde_yaml::mapping::Entry;
use serde_yaml::Mapping;
use serde_yaml::Value;

#[test]
fn occupied_entry_get_reads_value() {
    let mut m = Mapping::new();
    m.insert(Value::String("name".to_string()), Value::String("alice".to_string()));
    m.insert(Value::String("age".to_string()), Value::Number(30.into()));

    assert_eq!(m.len(), 2);

    match m.entry(Value::String("name".to_string())) {
        Entry::Occupied(o) => {
            assert_eq!(o.key(), &Value::String("name".to_string()));
            assert_eq!(o.get(), &Value::String("alice".to_string()));
            assert_ne!(o.get(), &Value::String("bob".to_string()));
        }
        Entry::Vacant(_) => panic!("expected Occupied entry for 'name'"),
    }


    assert_eq!(m.len(), 2);
    assert_eq!(m.get(&Value::String("name".to_string())), Some(&Value::String("alice".to_string())));
    assert_eq!(m.get(&Value::String("age".to_string())), Some(&Value::Number(30.into())));
}

#[test]
fn occupied_entry_get_mut_mutates_value() {
    let mut m = Mapping::new();
    m.insert(Value::String("counter".to_string()), Value::Number(1.into()));
    m.insert(Value::String("label".to_string()), Value::String("init".to_string()));


    assert_eq!(m.len(), 2);
    assert_eq!(
        m.get(&Value::String("counter".to_string())),
        Some(&Value::Number(1.into()))
    );

    match m.entry(Value::String("counter".to_string())) {
        Entry::Occupied(mut o) => {
            assert_eq!(o.get(), &Value::Number(1.into()));
            *o.get_mut() = Value::Number(42.into());
            assert_eq!(o.get(), &Value::Number(42.into()));
        }
        Entry::Vacant(_) => panic!("expected Occupied"),
    }


    assert_eq!(m.len(), 2);
    assert_eq!(
        m.get(&Value::String("counter".to_string())),
        Some(&Value::Number(42.into()))
    );
    assert_ne!(
        m.get(&Value::String("counter".to_string())),
        Some(&Value::Number(1.into()))
    );


    match m.entry(Value::String("label".to_string())) {
        Entry::Occupied(mut o) => {
            *o.get_mut() = Value::Bool(true);
            assert_eq!(o.get(), &Value::Bool(true));
        }
        Entry::Vacant(_) => panic!("expected Occupied"),
    }
    assert_eq!(
        m.get(&Value::String("label".to_string())),
        Some(&Value::Bool(true))
    );
}

#[test]
fn occupied_entry_remove_entry_returns_pair_and_shrinks() {
    let mut m = Mapping::new();
    m.insert(Value::String("a".to_string()), Value::Number(1.into()));
    m.insert(Value::String("b".to_string()), Value::Number(2.into()));
    m.insert(Value::String("c".to_string()), Value::Number(3.into()));

    assert_eq!(m.len(), 3);
    assert_eq!(m.get(&Value::String("b".to_string())), Some(&Value::Number(2.into())));

    let (k, v) = match m.entry(Value::String("b".to_string())) {
        Entry::Occupied(o) => {
            assert_eq!(o.key(), &Value::String("b".to_string()));
            assert_eq!(o.get(), &Value::Number(2.into()));
            o.remove_entry()
        }
        Entry::Vacant(_) => panic!("expected Occupied for 'b'"),
    };

    assert_eq!(k, Value::String("b".to_string()));
    assert_eq!(v, Value::Number(2.into()));


    assert_eq!(m.len(), 2);
    assert_eq!(m.get(&Value::String("b".to_string())), None);
    assert_eq!(m.get(&Value::String("a".to_string())), Some(&Value::Number(1.into())));
    assert_eq!(m.get(&Value::String("c".to_string())), Some(&Value::Number(3.into())));


    match m.entry(Value::String("b".to_string())) {
        Entry::Vacant(ve) => {
            assert_eq!(ve.key(), &Value::String("b".to_string()));
            ve.insert(Value::Number(99.into()));
        }
        Entry::Occupied(_) => panic!("expected Vacant after remove_entry"),
    }
    assert_eq!(m.len(), 3);
    assert_eq!(
        m.get(&Value::String("b".to_string())),
        Some(&Value::Number(99.into()))
    );
}

#[test]
fn occupied_entry_workflow_after_yaml_parse() {
    let yaml = "x: 10\ny: 20\nz: 30\n";
    let value: Value = serde_yaml::from_str(yaml).expect("parse");
    let mut m = match value {
        Value::Mapping(m) => m,
        _ => panic!("expected mapping"),
    };

    assert_eq!(m.len(), 3);


    match m.entry(Value::String("y".to_string())) {
        Entry::Occupied(mut o) => {
            assert_eq!(o.get(), &Value::Number(20.into()));
            *o.get_mut() = Value::Number(200.into());
            assert_eq!(o.get(), &Value::Number(200.into()));
        }
        Entry::Vacant(_) => panic!("expected Occupied 'y'"),
    }

    assert_eq!(m.get(&Value::String("y".to_string())), Some(&Value::Number(200.into())));
    assert_eq!(m.len(), 3);

    let removed = match m.entry(Value::String("z".to_string())) {
        Entry::Occupied(o) => o.remove_entry(),
        Entry::Vacant(_) => panic!("expected Occupied 'z'"),
    };
    assert_eq!(removed.0, Value::String("z".to_string()));
    assert_eq!(removed.1, Value::Number(30.into()));

    assert_eq!(m.len(), 2);
    assert_eq!(m.get(&Value::String("z".to_string())), None);
    assert_eq!(m.get(&Value::String("x".to_string())), Some(&Value::Number(10.into())));
}