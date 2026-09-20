#![cfg(feature = "unstable__schema")]

use borsh::schema::{BorshSchemaContainer, Declaration, Definition};
use std::collections::BTreeMap;

#[test]
fn test_new_empty_container() {
    let decl: Declaration = "MyType".to_string();
    let defs: BTreeMap<Declaration, Definition> = BTreeMap::new();
    let container = BorshSchemaContainer::new(decl.clone(), defs);

    assert_eq!(container.declaration(), &decl);
    assert_eq!(container.declaration().as_str(), "MyType");
    assert_eq!(container.definitions().count(), 0);
    assert!(container.get_definition("MyType").is_none());
    assert!(container.get_definition("NonExistent").is_none());
    assert_ne!(container.declaration().as_str(), "OtherType");
    assert!(container.definitions().next().is_none());
    let collected: Vec<(&Declaration, &Definition)> = container.definitions().collect();
    assert_eq!(collected.len(), 0);
    assert_eq!(container.declaration().len(), 6);
}

#[test]
fn test_new_with_prepopulated_definitions() {
    let decl = "Root".to_string();
    let mut defs = BTreeMap::new();
    defs.insert("i32".to_string(), Definition::Primitive(4));
    defs.insert("u8".to_string(), Definition::Primitive(1));
    defs.insert("u64".to_string(), Definition::Primitive(8));

    let container = BorshSchemaContainer::new(decl.clone(), defs);

    assert_eq!(container.declaration(), &decl);
    assert_eq!(container.declaration().as_str(), "Root");
    assert_eq!(container.definitions().count(), 3);

    let i32_def = container.get_definition("i32");
    assert!(i32_def.is_some());
    match i32_def.unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 4),
        _ => panic!("expected Primitive for i32"),
    }

    let u8_def = container.get_definition("u8");
    assert!(u8_def.is_some());
    match u8_def.unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 1),
        _ => panic!("expected Primitive for u8"),
    }

    let u64_def = container.get_definition("u64");
    assert!(u64_def.is_some());
    match u64_def.unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 8),
        _ => panic!("expected Primitive for u64"),
    }

    assert!(container.get_definition("missing").is_none());
    assert_ne!(i32_def, u8_def);
    assert_ne!(u8_def, u64_def);
}

#[test]
fn test_for_type_u32_primitive() {
    let container = BorshSchemaContainer::for_type::<u32>();

    assert_eq!(container.declaration().as_str(), "u32");
    assert_ne!(container.declaration().as_str(), "u64");
    assert_eq!(container.declaration().len(), 3);

    let def = container.get_definition("u32");
    assert!(def.is_some());
    match def.unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 4),
        other => panic!("expected Primitive(4) for u32, got {:?}", other),
    }

    assert_eq!(container.definitions().count(), 1);
    assert!(container.get_definition("u64").is_none());
    assert!(container.get_definition("i32").is_none());

    let all: Vec<(&Declaration, &Definition)> = container.definitions().collect();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0.as_str(), "u32");
}

#[test]
fn test_for_type_multiple_primitives_differ() {
    let c_u32 = BorshSchemaContainer::for_type::<u32>();
    let c_u64 = BorshSchemaContainer::for_type::<u64>();
    let c_i8 = BorshSchemaContainer::for_type::<i8>();
    let c_bool = BorshSchemaContainer::for_type::<bool>();

    assert_eq!(c_u32.declaration().as_str(), "u32");
    assert_eq!(c_u64.declaration().as_str(), "u64");
    assert_eq!(c_i8.declaration().as_str(), "i8");
    assert_eq!(c_bool.declaration().as_str(), "bool");

    assert_ne!(c_u32.declaration(), c_u64.declaration());
    assert_ne!(c_u64.declaration(), c_i8.declaration());
    assert_ne!(c_i8.declaration(), c_bool.declaration());

    match c_u32.get_definition("u32").unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 4),
        _ => panic!(),
    }
    match c_u64.get_definition("u64").unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 8),
        _ => panic!(),
    }
    match c_i8.get_definition("i8").unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 1),
        _ => panic!(),
    }

    assert!(c_u32.get_definition("u64").is_none());
    assert!(c_u64.get_definition("u32").is_none());
}

#[test]
fn test_insert_definition_new_and_replace() {
    let mut container =
        BorshSchemaContainer::new("Root".to_string(), BTreeMap::new());

    assert_eq!(container.definitions().count(), 0);

    let prev = container.insert_definition("A".to_string(), Definition::Primitive(1));
    assert!(prev.is_none());
    assert_eq!(container.definitions().count(), 1);
    match container.get_definition("A").unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 1),
        _ => panic!(),
    }

    let prev2 = container.insert_definition("A".to_string(), Definition::Primitive(2));
    assert!(prev2.is_some());
    match prev2.unwrap() {
        Definition::Primitive(n) => assert_eq!(n, 1),
        _ => panic!("expected previous Primitive(1)"),
    }
    assert_eq!(container.definitions().count(), 1);
    match container.get_definition("A").unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 2),
        _ => panic!(),
    }

    let prev3 = container.insert_definition("B".to_string(), Definition::Primitive(8));
    assert!(prev3.is_none());
    assert_eq!(container.definitions().count(), 2);
    assert!(container.get_definition("A").is_some());
    assert!(container.get_definition("B").is_some());
    assert_eq!(container.declaration().as_str(), "Root");
}

#[test]
fn test_remove_definition_present_and_absent() {
    let mut defs = BTreeMap::new();
    defs.insert("X".to_string(), Definition::Primitive(4));
    defs.insert("Y".to_string(), Definition::Primitive(8));
    defs.insert("Z".to_string(), Definition::Primitive(2));
    let mut container = BorshSchemaContainer::new("Root".to_string(), defs);

    assert_eq!(container.definitions().count(), 3);
    assert!(container.get_definition("X").is_some());
    assert!(container.get_definition("Y").is_some());
    assert!(container.get_definition("Z").is_some());

    let removed = container.remove_definition("X");
    assert!(removed.is_some());
    match removed.unwrap() {
        Definition::Primitive(n) => assert_eq!(n, 4),
        _ => panic!(),
    }

    assert_eq!(container.definitions().count(), 2);
    assert!(container.get_definition("X").is_none());
    assert!(container.get_definition("Y").is_some());
    assert!(container.get_definition("Z").is_some());

    let absent = container.remove_definition("NotHere");
    assert!(absent.is_none());
    assert_eq!(container.definitions().count(), 2);

    let absent_again = container.remove_definition("X");
    assert!(absent_again.is_none());
    assert_eq!(container.definitions().count(), 2);

    let removed_y = container.remove_definition("Y");
    assert!(removed_y.is_some());
    assert_eq!(container.definitions().count(), 1);
}

#[test]
fn test_get_mut_definition_mutates_in_place() {
    let mut defs = BTreeMap::new();
    defs.insert("A".to_string(), Definition::Primitive(1));
    defs.insert("B".to_string(), Definition::Primitive(2));
    let mut container = BorshSchemaContainer::new("Root".to_string(), defs);

    let before_a = container.get_definition("A").cloned();
    assert!(before_a.is_some());
    match before_a.as_ref().unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 1),
        _ => panic!(),
    }

    {
        let mut_ref = container.get_mut_definition("A");
        assert!(mut_ref.is_some());
        match mut_ref.unwrap() {
            Definition::Primitive(n) => {
                assert_eq!(*n, 1);
                *n = 16;
            }
            _ => panic!("expected Primitive"),
        }
    }

    let after_a = container.get_definition("A");
    assert!(after_a.is_some());
    match after_a.unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 16),
        _ => panic!(),
    }
    assert_ne!(before_a.as_ref(), after_a);

    // B untouched
    match container.get_definition("B").unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 2),
        _ => panic!(),
    }

    assert!(container.get_mut_definition("missing").is_none());
    assert_eq!(container.definitions().count(), 2);
}

#[test]
fn test_definitions_iterator_matches_get_definition() {
    let mut container = BorshSchemaContainer::new("Top".to_string(), BTreeMap::new());
    container.insert_definition("a".to_string(), Definition::Primitive(1));
    container.insert_definition("b".to_string(), Definition::Primitive(2));
    container.insert_definition("c".to_string(), Definition::Primitive(4));
    container.insert_definition("d".to_string(), Definition::Primitive(8));

    let count = container.definitions().count();
    assert_eq!(count, 4);

    let mut keys: Vec<String> = container
        .definitions()
        .map(|(k, _)| k.clone())
        .collect();
    keys.sort();
    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0].as_str(), "a");
    assert_eq!(keys[1].as_str(), "b");
    assert_eq!(keys[2].as_str(), "c");
    assert_eq!(keys[3].as_str(), "d");

    // Collect iteration pairs then verify each via get_definition.
    let pairs: Vec<(Declaration, Definition)> = container
        .definitions()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    assert_eq!(pairs.len(), 4);
    for (k, v) in &pairs {
        let via_get = container.get_definition(k.as_str()).unwrap();
        assert_eq!(via_get, v);
    }

    assert_eq!(container.declaration().as_str(), "Top");
}

#[test]
fn test_full_lifecycle_insert_mutate_remove() {
    let mut container = BorshSchemaContainer::for_type::<u32>();
    let initial_count = container.definitions().count();
    assert_eq!(initial_count, 1);
    assert_eq!(container.declaration().as_str(), "u32");

    // add a new alias definition
    let prev = container.insert_definition("MyAlias".to_string(), Definition::Primitive(4));
    assert!(prev.is_none());
    assert_eq!(container.definitions().count(), 2);

    // mutate through get_mut
    {
        let r = container.get_mut_definition("MyAlias");
        assert!(r.is_some());
        if let Some(Definition::Primitive(n)) = r {
            assert_eq!(*n, 4);
            *n = 7;
        } else {
            panic!("expected Primitive for MyAlias");
        }
    }

    match container.get_definition("MyAlias").unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 7),
        _ => panic!(),
    }

    // remove u32 and put it back
    let removed = container.remove_definition("u32");
    assert!(removed.is_some());
    assert_eq!(container.definitions().count(), 1);
    assert!(container.get_definition("u32").is_none());

    let reinserted = container
        .insert_definition("u32".to_string(), removed.unwrap());
    assert!(reinserted.is_none());
    assert_eq!(container.definitions().count(), 2);

    match container.get_definition("u32").unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 4),
        _ => panic!(),
    }

    // declaration is unaffected by all mutations
    assert_eq!(container.declaration().as_str(), "u32");

    // finally remove MyAlias and confirm
    let removed_alias = container.remove_definition("MyAlias");
    assert!(removed_alias.is_some());
    match removed_alias.unwrap() {
        Definition::Primitive(n) => assert_eq!(n, 7),
        _ => panic!(),
    }
    assert_eq!(container.definitions().count(), 1);
}

#[test]
fn test_declaration_is_independent_of_definitions() {
    let mut container = BorshSchemaContainer::new(
        "TopLevel".to_string(),
        BTreeMap::new(),
    );
    assert_eq!(container.declaration().as_str(), "TopLevel");
    assert_eq!(container.definitions().count(), 0);

    // Inserting a definition whose key equals the declaration does not
    // change the declaration string itself.
    let prev = container.insert_definition(
        "TopLevel".to_string(),
        Definition::Primitive(1),
    );
    assert!(prev.is_none());
    assert_eq!(container.declaration().as_str(), "TopLevel");
    assert_eq!(container.definitions().count(), 1);

    let self_def = container.get_definition("TopLevel");
    assert!(self_def.is_some());
    match self_def.unwrap() {
        Definition::Primitive(n) => assert_eq!(*n, 1),
        _ => panic!(),
    }

    // Remove that self-definition; declaration survives.
    let removed = container.remove_definition("TopLevel");
    assert!(removed.is_some());
    assert_eq!(container.declaration().as_str(), "TopLevel");
    assert_eq!(container.definitions().count(), 0);
    assert!(container.get_definition("TopLevel").is_none());
}