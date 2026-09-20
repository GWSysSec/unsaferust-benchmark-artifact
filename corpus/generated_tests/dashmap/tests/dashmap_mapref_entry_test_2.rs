use dashmap::DashMap;

#[test]
fn entry_and_modify_then_or_insert_with_updates_existing_and_lazily_inserts_missing() {
    let map: DashMap<String, Vec<i32>> = DashMap::new();

    let initial = map
        .entry("scores".to_string())
        .or_insert_with(|| vec![10, 20]);
    assert_eq!(&*initial, &vec![10, 20]);
    drop(initial);

    map.entry("scores".to_string())
        .and_modify(|values| {
            values.push(30);
            values.retain(|value| *value >= 20);
        })
        .or_insert_with(|| vec![999]);

    let scores = map.get("scores").expect("existing entry should remain present");
    assert_eq!(&*scores, &vec![20, 30]);
    drop(scores);

    map.entry("fallback".to_string())
        .and_modify(|values| values.push(1))
        .or_insert_with(|| vec![40, 50]);

    let fallback = map
        .get("fallback")
        .expect("missing entry should be inserted by or_insert_with");
    assert_eq!(&*fallback, &vec![40, 50]);
    drop(fallback);

    assert_eq!(map.len(), 2);
    assert!(map.contains_key("scores"));
    assert!(map.contains_key("fallback"));
}

#[test]
fn or_insert_with_is_lazy_for_occupied_entries_after_and_modify_chain() {
    let map: DashMap<&'static str, String> = DashMap::new();

    map.insert("status", "new".to_string());

    let mut factory_calls = 0;
    {
        let status = map
            .entry("status")
            .and_modify(|value| value.push_str("-modified"))
            .or_insert_with(|| {
                factory_calls += 1;
                "created".to_string()
            });

        assert_eq!(&*status, "new-modified");
    }

    assert_eq!(factory_calls, 0);
    assert_eq!(
        map.view("status", |_key, value| value.clone()),
        Some("new-modified".to_string())
    );

    {
        let created = map.entry("created").or_insert_with(|| {
            factory_calls += 1;
            "created".to_string()
        });

        assert_eq!(&*created, "created");
    }

    assert_eq!(factory_calls, 1);
    assert_eq!(map.len(), 2);
}

#[test]
fn or_try_insert_with_inserts_on_success_and_returns_error_without_inserting() {
    let map: DashMap<&'static str, usize> = DashMap::new();

    let inserted = map
        .entry("answer")
        .or_try_insert_with(|| -> Result<usize, &'static str> { Ok(42) })
        .expect("successful factory should insert");
    assert_eq!(*inserted, 42);
    drop(inserted);

    let result = map
        .entry("fallible")
        .or_try_insert_with(|| -> Result<usize, &'static str> { Err("not ready") });

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "not ready");
    assert!(!map.contains_key("fallible"));
    assert_eq!(map.len(), 1);

    let retry = map
        .entry("fallible")
        .and_modify(|value| *value += 100)
        .or_try_insert_with(|| -> Result<usize, &'static str> { Ok(7) })
        .expect("retry should insert after previous error");
    assert_eq!(*retry, 7);
    drop(retry);

    assert_eq!(map.get("fallible").map(|value| *value), Some(7));
    assert_eq!(map.len(), 2);
}

#[test]
fn or_try_insert_with_is_lazy_for_existing_entries_and_composes_with_and_modify() {
    let map: DashMap<String, i32> = DashMap::new();
    map.insert("counter".to_string(), 5);

    let mut fallible_factory_calls = 0;
    let counter = map
        .entry("counter".to_string())
        .and_modify(|value| *value *= 2)
        .or_try_insert_with(|| -> Result<i32, &'static str> {
            fallible_factory_calls += 1;
            Err("factory should not run for occupied entry")
        })
        .expect("occupied entry should not evaluate fallible factory");

    assert_eq!(*counter, 10);
    drop(counter);
    assert_eq!(fallible_factory_calls, 0);

    map.entry("counter".to_string())
        .and_modify(|value| *value += 3)
        .or_try_insert_with(|| -> Result<i32, &'static str> { Ok(100) })
        .expect("occupied entry should still be returned");
    assert_eq!(map.get("counter").map(|value| *value), Some(13));

    let absent_result = map
        .entry("absent".to_string())
        .or_try_insert_with(|| -> Result<i32, &'static str> { Err("missing dependency") });
    assert_eq!(absent_result.unwrap_err(), "missing dependency");
    assert!(!map.contains_key("absent"));

    let absent = map
        .entry("absent".to_string())
        .or_try_insert_with(|| -> Result<i32, &'static str> { Ok(21) })
        .expect("second attempt should insert");
    assert_eq!(*absent, 21);
    drop(absent);

    assert_eq!(map.len(), 2);
    assert_eq!(map.remove("counter"), Some(("counter".to_string(), 13)));
    assert_eq!(map.remove("absent"), Some(("absent".to_string(), 21)));
    assert!(map.is_empty());
}