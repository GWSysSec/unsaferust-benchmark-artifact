#![allow(deprecated)]

use indexmap::IndexMap;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

fn entries(map: &IndexMap<i32, String>) -> Vec<(i32, String)> {
    map.iter().map(|(k, v)| (*k, v.clone())).collect()
}

#[test]
fn capacity_hasher_split_truncate_append_and_clear_workflow() {
    let mut map: IndexMap<i32, String, RandomState> =
        IndexMap::with_capacity_and_hasher(2, RandomState::new());

    let _ = map.hasher().build_hasher();

    map.reserve_exact(8);
    assert!(map.capacity() >= 8);

    map.try_reserve_exact(3)
        .expect("small exact reservation should succeed");
    assert!(map.capacity() >= 3);

    for (key, value) in [
        (10, "ten"),
        (20, "twenty"),
        (30, "thirty"),
        (40, "forty"),
        (50, "fifty"),
    ] {
        assert_eq!(map.insert(key, value.to_string()), None);
    }

    map.truncate(4);
    assert_eq!(map.len(), 4);
    assert_eq!(
        entries(&map),
        vec![
            (10, "ten".to_string()),
            (20, "twenty".to_string()),
            (30, "thirty".to_string()),
            (40, "forty".to_string()),
        ]
    );

    map.truncate(10);
    assert_eq!(map.len(), 4);

    let mut tail = map.split_off(2);
    assert_eq!(
        entries(&map),
        vec![(10, "ten".to_string()), (20, "twenty".to_string())]
    );
    assert_eq!(
        entries(&tail),
        vec![(30, "thirty".to_string()), (40, "forty".to_string())]
    );

    assert_eq!(tail.insert(20, "TWENTY from tail".to_string()), None);
    assert_eq!(tail.insert(60, "sixty".to_string()), None);

    map.append(&mut tail);
    assert!(tail.is_empty());
    assert_eq!(
        entries(&map),
        vec![
            (10, "ten".to_string()),
            (20, "TWENTY from tail".to_string()),
            (30, "thirty".to_string()),
            (40, "forty".to_string()),
            (60, "sixty".to_string()),
        ]
    );

    map.clear();
    assert!(map.is_empty());
    assert!(map.capacity() >= map.len());
}

#[test]
fn mutation_removal_range_and_splice_workflow() {
    let mut map: IndexMap<i32, String> = IndexMap::new();
    for (key, value) in [
        (1, "one"),
        (2, "two"),
        (3, "three"),
        (4, "four"),
        (5, "five"),
        (6, "six"),
    ] {
        assert_eq!(map.insert(key, value.to_string()), None);
    }

    {
        let (index, key, value) = map.get_full_mut(&3).expect("key 3 exists");
        assert_eq!(index, 2);
        assert_eq!(*key, 3);
        value.push_str(" updated");
    }
    assert_eq!(map.get(&3).map(String::as_str), Some("three updated"));

    {
        let first = map.first_mut().expect("non-empty map has a first entry");
        assert_eq!(*first.0, 1);
        first.1.push_str("!");
    }
    {
        let last = map.last_mut().expect("non-empty map has a last entry");
        assert_eq!(*last.0, 6);
        last.1.push_str("!");
    }
    assert_eq!(map.get(&1).map(String::as_str), Some("one!"));
    assert_eq!(map.get(&6).map(String::as_str), Some("six!"));

    {
        let range = map.get_range(1..4).expect("valid immutable range");
        let range_entries: Vec<(i32, String)> =
            range.iter().map(|(k, v)| (*k, v.clone())).collect();
        assert_eq!(
            range_entries,
            vec![
                (2, "two".to_string()),
                (3, "three updated".to_string()),
                (4, "four".to_string()),
            ]
        );
    }

    {
        let range = map.get_range_mut(2..5).expect("valid mutable range");
        for (_, value) in range.iter_mut() {
            value.push_str(" range");
        }
    }
    assert_eq!(map.get(&3).map(String::as_str), Some("three updated range"));
    assert_eq!(map.get(&5).map(String::as_str), Some("five range"));

    let removed_by_splice: Vec<(i32, String)> = map
        .splice(
            1..3,
            vec![
                (20, "twenty".to_string()),
                (30, "thirty".to_string()),
                (40, "forty".to_string()),
            ],
        )
        .collect();
    assert_eq!(
        removed_by_splice,
        vec![
            (2, "two".to_string()),
            (3, "three updated range".to_string()),
        ]
    );
    assert_eq!(
        entries(&map),
        vec![
            (1, "one!".to_string()),
            (20, "twenty".to_string()),
            (30, "thirty".to_string()),
            (40, "forty".to_string()),
            (4, "four range".to_string()),
            (5, "five range".to_string()),
            (6, "six!".to_string()),
        ]
    );

    assert_eq!(map.remove_entry(&30), Some((30, "thirty".to_string())));
    assert!(!map.contains_key(&30));
    assert_eq!(
        entries(&map),
        vec![
            (1, "one!".to_string()),
            (20, "twenty".to_string()),
            (6, "six!".to_string()),
            (40, "forty".to_string()),
            (4, "four range".to_string()),
            (5, "five range".to_string()),
        ]
    );

    assert_eq!(
        map.shift_remove_entry(&20),
        Some((20, "twenty".to_string()))
    );
    assert_eq!(
        entries(&map),
        vec![
            (1, "one!".to_string()),
            (6, "six!".to_string()),
            (40, "forty".to_string()),
            (4, "four range".to_string()),
            (5, "five range".to_string()),
        ]
    );

    assert_eq!(map.shift_remove_index(1), Some((6, "six!".to_string())));
    assert_eq!(
        entries(&map),
        vec![
            (1, "one!".to_string()),
            (40, "forty".to_string()),
            (4, "four range".to_string()),
            (5, "five range".to_string()),
        ]
    );

    assert_eq!(map.shift_remove_index(99), None);

    map.retain(|key, value| {
        value.push_str(" kept");
        key % 2 == 0
    });
    assert_eq!(
        entries(&map),
        vec![
            (40, "forty kept".to_string()),
            (4, "four range kept".to_string()),
        ]
    );
}

#[test]
fn sorting_searching_partition_and_boxed_slice_workflow() {
    let mut map: IndexMap<i32, String> = IndexMap::new();
    for (key, value) in [
        (4, "dddd"),
        (1, "a"),
        (3, "ccc"),
        (2, "bb"),
        (5, "eeeee"),
    ] {
        assert_eq!(map.insert(key, value.to_string()), None);
    }

    map.sort_keys();
    assert_eq!(
        entries(&map),
        vec![
            (1, "a".to_string()),
            (2, "bb".to_string()),
            (3, "ccc".to_string()),
            (4, "dddd".to_string()),
            (5, "eeeee".to_string()),
        ]
    );

    assert_eq!(map.binary_search_by(|key, _| key.cmp(&3)), Ok(2));
    assert_eq!(
        map.binary_search_by_key(&4usize, |_, value| value.len()),
        Ok(3)
    );
    assert_eq!(map.partition_point(|key, _| *key < 4), 3);

    map.sort_by(|left_key, _, right_key, _| right_key.cmp(left_key));
    assert_eq!(
        map.keys().copied().collect::<Vec<_>>(),
        vec![5, 4, 3, 2, 1]
    );

    map.sort_by_cached_key(|_, value| value.len());
    assert_eq!(
        map.iter()
            .map(|(key, value)| (*key, value.len()))
            .collect::<Vec<_>>(),
        vec![(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]
    );

    map.sort_unstable_keys();
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);

    map.sort_unstable_by(|left_key, left_value, right_key, right_value| {
        right_value
            .len()
            .cmp(&left_value.len())
            .then_with(|| left_key.cmp(right_key))
    });
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![5, 4, 3, 2, 1]);

    let sorted_by_pairs: Vec<(i32, String)> = map
        .clone()
        .sorted_by(|left_key, _, right_key, _| left_key.cmp(right_key))
        .collect();
    assert_eq!(
        sorted_by_pairs,
        vec![
            (1, "a".to_string()),
            (2, "bb".to_string()),
            (3, "ccc".to_string()),
            (4, "dddd".to_string()),
            (5, "eeeee".to_string()),
        ]
    );

    let sorted_unstable_by_pairs: Vec<(i32, String)> = map
        .clone()
        .sorted_unstable_by(|_, left_value, _, right_value| {
            left_value.len().cmp(&right_value.len())
        })
        .collect();
    assert_eq!(
        sorted_unstable_by_pairs,
        vec![
            (1, "a".to_string()),
            (2, "bb".to_string()),
            (3, "ccc".to_string()),
            (4, "dddd".to_string()),
            (5, "eeeee".to_string()),
        ]
    );

    let boxed = map.into_boxed_slice();
    assert_eq!(boxed.len(), 5);
    assert_eq!(boxed.first(), Some((&5, &"eeeee".to_string())));
    assert_eq!(boxed.last(), Some((&1, &"a".to_string())));
    assert_eq!(
        boxed
            .get_range(1..4)
            .expect("valid boxed slice range")
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![4, 3, 2]
    );
}

#[test]
fn empty_edge_cases_for_ranges_removal_and_clear() {
    let mut map: IndexMap<i32, String> = IndexMap::new();

    assert!(map.get_range(0..0).expect("empty range is valid").is_empty());
    assert!(map.get_range_mut(0..0).expect("empty range is valid").is_empty());
    assert!(map.get_range(1..1).is_none());
    assert_eq!(map.remove_entry(&1), None);
    assert_eq!(map.shift_remove_entry(&1), None);
    assert_eq!(map.shift_remove_index(0), None);

    map.clear();
    assert!(map.is_empty());

    assert_eq!(
        map.splice(0..0, vec![(7, "seven".to_string())])
            .collect::<Vec<_>>(),
        Vec::<(i32, String)>::new()
    );
    assert_eq!(entries(&map), vec![(7, "seven".to_string())]);

    map.truncate(0);
    assert!(map.is_empty());
}