use indexmap::map::IndexMap;

#[test]
fn iter_mut_into_slice_exposes_only_remaining_entries_and_allows_mutation() {
    let mut map: IndexMap<&str, i32> = IndexMap::new();

    assert!(map.is_empty());
    assert_eq!(map.insert("alpha", 10), None);
    assert_eq!(map.insert("beta", 20), None);
    assert_eq!(map.insert("gamma", 30), None);
    assert_eq!(map.insert("delta", 40), None);
    assert_eq!(map.len(), 4);

    {
        let mut iter = map.iter_mut();

        let first = iter.next().expect("iterator should yield the first entry");
        assert_eq!(*first.0, "alpha");
        assert_eq!(*first.1, 10);
        *first.1 += 1;

        let remaining = iter.into_slice();
        assert_eq!(remaining.len(), 3);
        assert!(!remaining.is_empty());

        let first_remaining = remaining
            .first()
            .expect("remaining slice should still contain beta");
        assert_eq!(*first_remaining.0, "beta");
        assert_eq!(*first_remaining.1, 20);

        let last_remaining = remaining
            .last()
            .expect("remaining slice should still contain delta");
        assert_eq!(*last_remaining.0, "delta");
        assert_eq!(*last_remaining.1, 40);

        let gamma = remaining
            .get_index_mut(1)
            .expect("gamma should be the second remaining entry");
        assert_eq!(*gamma.0, "gamma");
        *gamma.1 += 300;

        for (_, value) in remaining.iter_mut() {
            *value *= 2;
        }
    }

    assert_eq!(map.get("alpha"), Some(&11));
    assert_eq!(map.get("beta"), Some(&40));
    assert_eq!(map.get("gamma"), Some(&660));
    assert_eq!(map.get("delta"), Some(&80));

    let ordered_entries: Vec<(&str, i32)> = map.iter().map(|(key, value)| (*key, *value)).collect();
    assert_eq!(
        ordered_entries,
        vec![("alpha", 11), ("beta", 40), ("gamma", 660), ("delta", 80)]
    );
}

#[test]
fn iter_mut_into_slice_on_exhausted_iterator_returns_empty_mutable_slice() {
    let mut map: IndexMap<&str, usize> = IndexMap::new();
    map.insert("only", 1);

    {
        let mut iter = map.iter_mut();
        let entry = iter.next().expect("single entry should exist");
        assert_eq!(*entry.0, "only");
        *entry.1 = 99;

        let remaining = iter.into_slice();
        assert_eq!(remaining.len(), 0);
        assert!(remaining.is_empty());
        assert_eq!(remaining.first(), None);
        assert_eq!(remaining.last(), None);
        assert_eq!(remaining.get_index(0), None);
    }

    assert_eq!(map.len(), 1);
    assert_eq!(map.get("only"), Some(&99));
}