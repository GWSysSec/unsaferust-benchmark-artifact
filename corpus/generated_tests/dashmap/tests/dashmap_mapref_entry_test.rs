use dashmap::DashMap;

#[test]
fn entry_insert_modify_and_default_workflow() {
    let map: DashMap<String, Vec<i32>> = DashMap::new();

    let mut defaulted = map.entry("numbers".to_string()).or_default();
    assert!(defaulted.is_empty());
    defaulted.push(1);
    defaulted.push(2);
    drop(defaulted);

    map.entry("numbers".to_string())
        .and_modify(|values| values.push(3))
        .or_insert(vec![99]);

    let observed = map.get("numbers").expect("numbers should be present");
    assert_eq!(&*observed, &vec![1, 2, 3]);
    drop(observed);

    map.entry("letters".to_string())
        .and_modify(|values| values.push(100))
        .or_insert(vec![4, 5]);

    let inserted = map.get("letters").expect("letters should be inserted");
    assert_eq!(&*inserted, &vec![4, 5]);
    drop(inserted);

    let missing_key = map.entry("not_inserted".to_string()).into_key();
    assert_eq!(missing_key, "not_inserted");
    assert!(!map.contains_key("not_inserted"));
}

#[test]
fn entry_or_insert_with_and_try_insert_workflow() {
    let map: DashMap<&'static str, i32> = DashMap::new();

    let value = map.entry("eager").or_insert(10);
    assert_eq!(*value, 10);
    drop(value);

    let existing = map.entry("eager").or_insert(99);
    assert_eq!(*existing, 10);
    drop(existing);

    let mut factory_calls = 0;
    let lazy = map.entry("lazy").or_insert_with(|| {
        factory_calls += 1;
        20
    });
    assert_eq!(*lazy, 20);
    drop(lazy);
    assert_eq!(factory_calls, 1);

    let existing_lazy = map.entry("lazy").or_insert_with(|| {
        factory_calls += 1;
        30
    });
    assert_eq!(*existing_lazy, 20);
    drop(existing_lazy);
    assert_eq!(factory_calls, 1);

    let fallible = map
        .entry("fallible")
        .or_try_insert_with(|| -> Result<i32, &'static str> { Ok(40) })
        .expect("successful fallible insertion should return a reference");
    assert_eq!(*fallible, 40);
    drop(fallible);

    let failed = map
        .entry("failed")
        .or_try_insert_with(|| -> Result<i32, &'static str> { Err("construction failed") });
    assert!(failed.is_err());
    assert!(!map.contains_key("failed"));

    let existing_fallible = map
        .entry("fallible")
        .or_try_insert_with(|| -> Result<i32, &'static str> {
            panic!("constructor must not run for an occupied entry")
        })
        .expect("occupied entry should return the existing value");
    assert_eq!(*existing_fallible, 40);
}

#[test]
fn occupied_entry_into_ref_replace_and_remove_workflow() {
    let map: DashMap<String, i32> = DashMap::new();

    let occupied = map.entry("counter".to_string()).insert_entry(5);
    let mut counter_ref = occupied.into_ref();
    assert_eq!(*counter_ref, 5);
    *counter_ref += 7;
    assert_eq!(*counter_ref, 12);
    drop(counter_ref);

    assert_eq!(*map.get("counter").expect("counter should remain present"), 12);

    let replace_occupied = map.entry("replace_me".to_string()).insert_entry(100);
    let (replaced_key, replaced_value) = replace_occupied.replace_entry(200);
    assert_eq!(replaced_key, "replace_me");
    assert_eq!(replaced_value, 100);
    assert_eq!(
        *map.get("replace_me")
            .expect("replace_me should still be present after replacement"),
        200
    );

    let remove_occupied = map.entry("remove_me".to_string()).insert_entry(300);
    let (removed_key, removed_value) = remove_occupied.remove_entry();
    assert_eq!(removed_key, "remove_me");
    assert_eq!(removed_value, 300);
    assert!(!map.contains_key("remove_me"));

    assert_eq!(map.len(), 2);
    assert!(map.contains_key("counter"));
    assert!(map.contains_key("replace_me"));
}