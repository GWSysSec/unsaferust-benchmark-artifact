use indexmap::map::RawEntryApiV1;
use indexmap::IndexMap;

#[test]
fn raw_entry_mut_chains_modify_existing_and_lazily_inserts_missing() {
    let mut map: IndexMap<String, Vec<i32>> = IndexMap::new();
    map.insert("alpha".to_string(), vec![1, 2]);
    map.insert("beta".to_string(), vec![10]);

    let mut occupied_default_called = false;
    {
        let (key, values) = map
            .raw_entry_mut_v1()
            .from_key("alpha")
            .and_modify(|key, values| {
                assert_eq!(key.as_str(), "alpha");
                values.push(3);
                values[0] *= 10;
            })
            .or_insert_with(|| {
                occupied_default_called = true;
                ("alpha".to_string(), vec![99])
            });

        assert_eq!(key.as_str(), "alpha");
        values.push(4);
    }

    assert!(
        !occupied_default_called,
        "or_insert_with must not run when the raw entry is occupied"
    );
    assert_eq!(map.get("alpha").map(Vec::as_slice), Some(&[10, 2, 3, 4][..]));
    assert_eq!(map.get("beta").map(Vec::as_slice), Some(&[10][..]));
    assert_eq!(map.len(), 2);
    assert_eq!(map.get_index_of("alpha"), Some(0));

    let mut vacant_modify_called = false;
    {
        let (key, values) = map
            .raw_entry_mut_v1()
            .from_key("gamma")
            .and_modify(|_, values| {
                vacant_modify_called = true;
                values.push(-1);
            })
            .or_insert_with(|| ("gamma".to_string(), vec![7, 8]));

        assert_eq!(key.as_str(), "gamma");
        values.push(9);
    }

    assert!(
        !vacant_modify_called,
        "and_modify must not run when the raw entry is vacant"
    );
    assert_eq!(map.get("gamma").map(Vec::as_slice), Some(&[7, 8, 9][..]));
    assert_eq!(map.len(), 3);
    assert_eq!(map.get_index_of("gamma"), Some(2));
}

#[test]
fn raw_entry_mut_updates_inserted_value_on_later_lookup() {
    let mut map: IndexMap<String, i32> = IndexMap::new();

    {
        let (key, value) = map
            .raw_entry_mut_v1()
            .from_key("counter")
            .and_modify(|_, value| *value += 100)
            .or_insert_with(|| ("counter".to_string(), 1));

        assert_eq!(key.as_str(), "counter");
        assert_eq!(*value, 1);
        *value += 4;
    }

    assert_eq!(map.get("counter"), Some(&5));
    assert_eq!(map.len(), 1);

    let mut inserted_again = false;
    {
        let (key, value) = map
            .raw_entry_mut_v1()
            .from_key("counter")
            .and_modify(|key, value| {
                assert_eq!(key.as_str(), "counter");
                *value *= 3;
            })
            .or_insert_with(|| {
                inserted_again = true;
                ("counter".to_string(), 999)
            });

        assert_eq!(key.as_str(), "counter");
        assert_eq!(*value, 15);
        *value -= 2;
    }

    assert!(
        !inserted_again,
        "or_insert_with must remain lazy after the key has been inserted"
    );
    assert_eq!(map.get("counter"), Some(&13));
    assert_eq!(
        map.get_full("counter")
            .map(|(index, key, value)| (index, key.as_str(), *value)),
        Some((0, "counter", 13))
    );
}