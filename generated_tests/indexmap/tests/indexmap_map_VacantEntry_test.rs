use indexmap::map::Entry;
use indexmap::IndexMap;

#[test]
fn vacant_entry_into_key_returns_owned_key_without_inserting() {
    let mut map: IndexMap<String, i32> = IndexMap::new();

    assert!(map.is_empty());
    assert_eq!(map.insert("alpha".to_string(), 10), None);
    assert_eq!(map.insert("beta".to_string(), 20), None);

    let original_order: Vec<String> = map.keys().cloned().collect();
    assert_eq!(original_order, vec!["alpha".to_string(), "beta".to_string()]);
    assert_eq!(map.len(), 2);

    let reclaimed_key = match map.entry("gamma".to_string()) {
        Entry::Vacant(vacant) => {
            assert_eq!(vacant.index(), 2);
            assert_eq!(vacant.key(), "gamma");
            vacant.into_key()
        }
        Entry::Occupied(_) => panic!("gamma should not already be present"),
    };

    assert_eq!(reclaimed_key, "gamma".to_string());
    assert_eq!(map.len(), 2);
    assert!(!map.contains_key("gamma"));
    assert_eq!(map.get("alpha"), Some(&10));
    assert_eq!(map.get("beta"), Some(&20));

    let order_after_into_key: Vec<String> = map.keys().cloned().collect();
    assert_eq!(order_after_into_key, original_order);

    let mut derived_key = reclaimed_key;
    derived_key.push_str("-inserted");
    assert_eq!(map.insert(derived_key.clone(), 30), None);

    assert_eq!(map.len(), 3);
    assert!(map.contains_key("gamma-inserted"));
    assert!(!map.contains_key("gamma"));
    assert_eq!(map.get_index_of("gamma-inserted"), Some(2));

    let final_entries: Vec<(String, i32)> = map.iter().map(|(k, v)| (k.clone(), *v)).collect();
    assert_eq!(
        final_entries,
        vec![
            ("alpha".to_string(), 10),
            ("beta".to_string(), 20),
            ("gamma-inserted".to_string(), 30),
        ]
    );
}

#[test]
fn vacant_entry_into_key_after_removal_does_not_restore_removed_slot() {
    let mut map: IndexMap<String, i32> = IndexMap::new();

    map.insert("red".to_string(), 1);
    map.insert("green".to_string(), 2);
    map.insert("blue".to_string(), 3);

    assert_eq!(map.shift_remove("green"), Some(2));
    assert_eq!(map.len(), 2);
    assert_eq!(map.get_index_of("blue"), Some(1));

    let reclaimed_key = match map.entry("green".to_string()) {
        Entry::Vacant(vacant) => {
            assert_eq!(vacant.index(), 2);
            vacant.into_key()
        }
        Entry::Occupied(_) => panic!("green should be vacant after removal"),
    };

    assert_eq!(reclaimed_key, "green");
    assert_eq!(map.len(), 2);
    assert!(!map.contains_key("green"));

    let remaining: Vec<(String, i32)> = map.iter().map(|(k, v)| (k.clone(), *v)).collect();
    assert_eq!(
        remaining,
        vec![("red".to_string(), 1), ("blue".to_string(), 3)]
    );

    match map.entry(reclaimed_key) {
        Entry::Vacant(vacant) => {
            assert_eq!(vacant.index(), 2);
            let value_ref = vacant.insert(22);
            assert_eq!(*value_ref, 22);
        }
        Entry::Occupied(_) => panic!("green should still be vacant before reinsertion"),
    }

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("green"), Some(&22));
    assert_eq!(map.get_index_of("green"), Some(2));
}