use indexmap::map::IndexMap;

#[test]
fn iter_mut_into_slice_returns_unvisited_tail_and_mutations_are_preserved() {
    let mut map: IndexMap<&str, i32> = IndexMap::new();

    assert_eq!(map.insert("alpha", 10), None);
    assert_eq!(map.insert("beta", 20), None);
    assert_eq!(map.insert("gamma", 30), None);
    assert_eq!(map.insert("delta", 40), None);
    assert_eq!(map.len(), 4);

    {
        let mut iter = map.iter_mut();

        let (first_key, first_value) = iter.next().expect("first entry should exist");
        assert_eq!(*first_key, "alpha");
        assert_eq!(*first_value, 10);
        *first_value += 5;

        let remaining = iter.into_slice();
        assert_eq!(remaining.len(), 3);
        assert!(!remaining.is_empty());

        let (remaining_first_key, remaining_first_value) =
            remaining.first_mut().expect("remaining slice should have a first entry");
        assert_eq!(*remaining_first_key, "beta");
        assert_eq!(*remaining_first_value, 20);
        *remaining_first_value += 2;

        let (gamma_key, gamma_value) = remaining
            .get_index_mut(1)
            .expect("gamma should be the second remaining entry");
        assert_eq!(*gamma_key, "gamma");
        assert_eq!(*gamma_value, 30);
        *gamma_value *= 2;

        let (remaining_last_key, remaining_last_value) =
            remaining.last().expect("remaining slice should have a last entry");
        assert_eq!(*remaining_last_key, "delta");
        assert_eq!(*remaining_last_value, 40);
    }

    assert_eq!(map.get("alpha"), Some(&15));
    assert_eq!(map.get("beta"), Some(&22));
    assert_eq!(map.get("gamma"), Some(&60));
    assert_eq!(map.get("delta"), Some(&40));

    let ordered: Vec<(&str, i32)> = map.iter().map(|(key, value)| (*key, *value)).collect();
    assert_eq!(
        ordered,
        vec![("alpha", 15), ("beta", 22), ("gamma", 60), ("delta", 40)]
    );
}

#[test]
fn iter_mut_into_slice_after_consuming_all_entries_is_empty() {
    let mut map: IndexMap<&str, usize> = IndexMap::new();
    map.insert("one", 1);
    map.insert("two", 2);

    {
        let mut iter = map.iter_mut();

        let (_, first_value) = iter.next().expect("first entry should exist");
        *first_value = 10;

        let (_, second_value) = iter.next().expect("second entry should exist");
        *second_value = 20;

        assert!(iter.next().is_none());

        let empty_tail = iter.into_slice();
        assert_eq!(empty_tail.len(), 0);
        assert!(empty_tail.is_empty());
        assert!(empty_tail.first().is_none());
        assert!(empty_tail.last().is_none());
    }

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("one"), Some(&10));
    assert_eq!(map.get("two"), Some(&20));
}

#[test]
fn iter_mut_into_slice_can_be_split_and_mutated_in_sections() {
    let mut map: IndexMap<&str, i32> = IndexMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);
    map.insert("d", 4);
    map.insert("e", 5);

    {
        let mut iter = map.iter_mut();

        let (skipped_key, skipped_value) = iter.next().expect("first entry should exist");
        assert_eq!(*skipped_key, "a");
        *skipped_value = 100;

        let tail = iter.into_slice();
        assert_eq!(tail.len(), 4);

        let (left, right) = tail.split_at_mut(2);
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 2);

        let (_, b_value) = left
            .get_index_mut(0)
            .expect("left section should contain b at index 0");
        *b_value += 10;

        let (_, c_value) = left
            .get_index_mut(1)
            .expect("left section should contain c at index 1");
        *c_value += 20;

        let (_, d_value) = right
            .first_mut()
            .expect("right section should contain d first");
        *d_value += 30;

        let (_, e_value) = right.last_mut().expect("right section should contain e last");
        *e_value += 40;
    }

    assert_eq!(map.get_index(0), Some((&"a", &100)));
    assert_eq!(map.get_index(1), Some((&"b", &12)));
    assert_eq!(map.get_index(2), Some((&"c", &23)));
    assert_eq!(map.get_index(3), Some((&"d", &34)));
    assert_eq!(map.get_index(4), Some((&"e", &45)));
}