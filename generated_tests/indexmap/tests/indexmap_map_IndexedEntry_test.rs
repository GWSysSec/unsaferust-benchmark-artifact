use indexmap::IndexMap;

fn ordered_entries(map: &IndexMap<&'static str, i32>) -> Vec<(&'static str, i32)> {
    map.iter().map(|(key, value)| (*key, *value)).collect()
}

#[test]
fn indexed_entry_shift_remove_entry_preserves_following_order() {
    let mut map = IndexMap::new();

    assert_eq!(map.insert("alpha", 10), None);
    assert_eq!(map.insert("bravo", 20), None);
    assert_eq!(map.insert("charlie", 30), None);
    assert_eq!(map.insert("delta", 40), None);
    assert_eq!(map.insert("echo", 50), None);

    assert_eq!(map.len(), 5);
    assert_eq!(map.get_index_of("charlie"), Some(2));
    assert_eq!(map.get_index(3), Some((&"delta", &40)));

    {
        let mut entry = map
            .get_index_entry(2)
            .expect("index 2 should contain the charlie entry");
        assert_eq!(entry.index(), 2);
        assert_eq!(entry.key(), &"charlie");
        assert_eq!(entry.get(), &30);
        assert_eq!(entry.insert(33), 30);
        assert_eq!(entry.get(), &33);
    }

    let removed = map
        .get_index_entry(2)
        .expect("updated charlie entry should still be at index 2")
        .shift_remove_entry();

    assert_eq!(removed, ("charlie", 33));
    assert_eq!(map.len(), 4);
    assert!(!map.contains_key("charlie"));
    assert_eq!(
        ordered_entries(&map),
        vec![("alpha", 10), ("bravo", 20), ("delta", 40), ("echo", 50)]
    );

    assert_eq!(map.get_index_of("delta"), Some(2));
    assert_eq!(map.get_index_of("echo"), Some(3));
    assert_eq!(map.first(), Some((&"alpha", &10)));
    assert_eq!(map.last(), Some((&"echo", &50)));

    let removed_first = map
        .get_index_entry(0)
        .expect("first entry should exist")
        .shift_remove_entry();

    assert_eq!(removed_first, ("alpha", 10));
    assert_eq!(
        ordered_entries(&map),
        vec![("bravo", 20), ("delta", 40), ("echo", 50)]
    );
    assert_eq!(map.get_index(0), Some((&"bravo", &20)));
    assert_eq!(map.get_index(1), Some((&"delta", &40)));
    assert!(map.get_index_entry(map.len()).is_none());
}

#[test]
fn indexed_entry_shift_remove_entry_on_last_entry_matches_pop_semantics() {
    let mut map = IndexMap::new();

    for (key, value) in [("red", 1), ("green", 2), ("blue", 3)] {
        assert_eq!(map.insert(key, value), None);
    }

    let last_index = map.len() - 1;
    let removed = map
        .get_index_entry(last_index)
        .expect("last entry should be available")
        .shift_remove_entry();

    assert_eq!(removed, ("blue", 3));
    assert_eq!(map.len(), 2);
    assert_eq!(ordered_entries(&map), vec![("red", 1), ("green", 2)]);
    assert_eq!(map.get_index_of("red"), Some(0));
    assert_eq!(map.get_index_of("green"), Some(1));
    assert_eq!(map.get_index_of("blue"), None);

    assert_eq!(map.insert("yellow", 4), None);
    assert_eq!(
        ordered_entries(&map),
        vec![("red", 1), ("green", 2), ("yellow", 4)]
    );
}