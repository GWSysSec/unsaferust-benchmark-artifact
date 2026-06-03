use indexmap::map::Entry;
use indexmap::IndexMap;

#[test]
fn entry_chaining_modifies_existing_and_lazily_inserts_missing_values() {
    let mut map: IndexMap<String, Vec<i32>> = IndexMap::new();
    map.insert("alpha".to_string(), vec![1, 2]);
    map.insert("beta".to_string(), vec![10]);

    let mut fallback_called = false;
    map.entry("alpha".to_string())
        .and_modify(|values| {
            values.push(3);
            values[0] *= 10;
        })
        .or_insert_with(|| {
            fallback_called = true;
            vec![99]
        })
        .push(4);

    assert!(
        !fallback_called,
        "or_insert_with must not evaluate its closure for occupied entries"
    );
    assert_eq!(map.get("alpha").map(Vec::as_slice), Some(&[10, 2, 3, 4][..]));
    assert_eq!(map.get("beta").map(Vec::as_slice), Some(&[10][..]));
    assert_eq!(map.len(), 2);
    assert_eq!(map.get_index_of("alpha"), Some(0));

    let gamma_values = map
        .entry("gamma".to_string())
        .and_modify(|values| values.push(-1))
        .or_insert_with(|| vec![7, 8]);

    gamma_values.push(9);

    assert_eq!(map.get("gamma").map(Vec::as_slice), Some(&[7, 8, 9][..]));
    assert_eq!(map.len(), 3);
    assert_eq!(map.get_index_of("gamma"), Some(2));
}

#[test]
fn or_insert_with_key_can_derive_value_from_the_vacant_key_without_cloning() {
    let mut map: IndexMap<String, usize> = IndexMap::new();
    map.insert("red".to_string(), 3);

    let red = map
        .entry("red".to_string())
        .and_modify(|value| *value += 10)
        .or_insert_with_key(|key| key.len() * 100);

    assert_eq!(*red, 13);
    assert_eq!(map.get("red"), Some(&13));
    assert_eq!(map.len(), 1);

    let inserted = map
        .entry("ultraviolet".to_string())
        .and_modify(|value| *value += 1)
        .or_insert_with_key(|key| key.len());

    assert_eq!(*inserted, "ultraviolet".len());
    assert_eq!(map.get("ultraviolet"), Some(&11));
    assert_eq!(
        map.iter().map(|(key, value)| (key.as_str(), *value)).collect::<Vec<_>>(),
        vec![("red", 13), ("ultraviolet", 11)]
    );
}

#[test]
fn occupied_entry_remove_entry_removes_the_matching_pair() {
    let mut map: IndexMap<String, i32> = IndexMap::new();
    map.insert("first".to_string(), 1);
    map.insert("middle".to_string(), 2);
    map.insert("last".to_string(), 3);

    let removed = match map.entry("middle".to_string()) {
        Entry::Occupied(entry) => entry.remove_entry(),
        Entry::Vacant(_) => panic!("expected middle to be occupied"),
    };

    assert_eq!(removed, ("middle".to_string(), 2));
    assert_eq!(map.len(), 2);
    assert!(!map.contains_key("middle"));
    assert_eq!(map.get("first"), Some(&1));
    assert_eq!(map.get("last"), Some(&3));

    let missing_key = match map.entry("missing".to_string()) {
        Entry::Vacant(entry) => entry.into_key(),
        Entry::Occupied(_) => panic!("missing key should not be occupied"),
    };

    assert_eq!(missing_key, "missing");
    assert_eq!(map.len(), 2);
    assert!(!map.contains_key("missing"));
}

#[test]
fn occupied_entry_shift_remove_entry_preserves_relative_order_of_remaining_items() {
    let mut map: IndexMap<String, i32> = IndexMap::new();
    map.insert("a".to_string(), 10);
    map.insert("b".to_string(), 20);
    map.insert("c".to_string(), 30);
    map.insert("d".to_string(), 40);

    let removed = match map.entry("b".to_string()) {
        Entry::Occupied(entry) => entry.shift_remove_entry(),
        Entry::Vacant(_) => panic!("expected b to be occupied"),
    };

    assert_eq!(removed, ("b".to_string(), 20));
    assert_eq!(map.len(), 3);
    assert_eq!(
        map.iter().map(|(key, value)| (key.as_str(), *value)).collect::<Vec<_>>(),
        vec![("a", 10), ("c", 30), ("d", 40)]
    );
    assert_eq!(map.get_index_of("a"), Some(0));
    assert_eq!(map.get_index_of("c"), Some(1));
    assert_eq!(map.get_index_of("d"), Some(2));
    assert_eq!(map.get_index_of("b"), None);
}