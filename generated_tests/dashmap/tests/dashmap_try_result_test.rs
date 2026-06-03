use dashmap::DashMap;

#[test]
fn try_result_reports_present_and_unwraps_value_after_map_workflow() {
    let map = DashMap::new();

    assert!(map.is_empty());
    assert_eq!(map.insert("alpha", 10), None);
    assert_eq!(map.insert("beta", 20), None);
    assert_eq!(map.len(), 2);

    let present = map.try_get("alpha");
    assert!(present.is_present());
    assert!(!present.is_absent());

    let value_ref = present.try_unwrap();
    assert!(value_ref.is_some());
    assert_eq!(*value_ref.unwrap(), 10);

    map.alter("alpha", |_key, value| value + 5);

    let updated = map.try_get("alpha");
    assert!(updated.is_present());
    assert_eq!(*updated.try_unwrap().unwrap(), 15);

    assert!(map.contains_key("beta"));
    assert_eq!(map.remove("beta"), Some(("beta", 20)));
    assert!(!map.contains_key("beta"));
}

#[test]
fn try_result_reports_absent_and_try_unwrap_returns_none_for_missing_key() {
    let map = DashMap::with_capacity(4);

    map.insert("existing", 42);
    assert_eq!(map.len(), 1);

    let absent = map.try_get("missing");
    assert!(absent.is_absent());
    assert!(!absent.is_present());
    assert!(absent.try_unwrap().is_none());

    let removed = map.remove("existing");
    assert_eq!(removed, Some(("existing", 42)));
    assert!(map.is_empty());

    let removed_result = map.try_get("existing");
    assert!(removed_result.is_absent());
    assert!(removed_result.try_unwrap().is_none());
}

#[test]
fn try_result_try_unwrap_returns_none_while_target_shard_is_locked() {
    let map = DashMap::new();

    map.insert("locked-key", 7);
    map.insert("other-key", 11);

    let mut write_guard = map.get_mut("locked-key").expect("key should exist");
    *write_guard += 1;

    let locked_result = map.try_get("locked-key");
    assert!(locked_result.is_locked());
    assert!(!locked_result.is_present());
    assert!(!locked_result.is_absent());
    assert!(locked_result.try_unwrap().is_none());

    drop(write_guard);

    let available_again = map.try_get("locked-key");
    assert!(available_again.is_present());
    assert_eq!(*available_again.try_unwrap().unwrap(), 8);

    let other_value = map.try_get("other-key");
    assert!(other_value.is_present());
    assert_eq!(*other_value.try_unwrap().unwrap(), 11);
}