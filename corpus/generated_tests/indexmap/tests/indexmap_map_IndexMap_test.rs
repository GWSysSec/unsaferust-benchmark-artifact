#![allow(deprecated)]

use indexmap::IndexMap;
use std::cmp::Ordering;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

fn entries(map: &IndexMap<i32, String>) -> Vec<(i32, String)> {
    map.iter().map(|(k, v)| (*k, v.clone())).collect()
}

#[test]
fn capacity_hasher_split_truncate_clear_and_append_workflow() {
    let mut map: IndexMap<i32, String, RandomState> =
        IndexMap::with_capacity_and_hasher(2, RandomState::new());

    let _ = map.hasher().build_hasher();

    map.reserve_exact(6);
    assert!(map.capacity() >= 6);

    map.try_reserve_exact(4)
        .expect("small exact reservation should succeed");
    assert!(map.capacity() >= 4);

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

    map.truncate(99);
    assert_eq!(map.len(), 4);

    let old_capacity = map.capacity();
    let mut tail = map.split_off(2);
    assert_eq!(map.capacity(), old_capacity);
    assert_eq!(entries(&map), vec![(10, "ten".to_string()), (20, "twenty".to_string())]);
    assert_eq!(
        entries(&tail),
        vec![(30, "thirty".to_string()), (40, "forty".to_string())]
    );

    tail.insert(20, "TWENTY".to_string());
    tail.insert(60, "sixty".to_string());

    map.append(&mut tail);
    assert!(tail.is_empty());
    assert_eq!(entries(&map), vec![
        (10, "ten".to_string()),
        (20, "TWENTY".to_string()),
        (30, "thirty".to_string()),
        (40, "forty".to_string()),
        (60, "sixty".to_string()),
    ]);

    let capacity_before_clear = map.capacity();
    map.clear();
    assert!(map.is_empty());
    assert_eq!(map.capacity(), capacity_before_clear);
}

#[test]
fn mutation_splice_remove_retain_and_range_workflow() {
    let mut map: IndexMap<i32, String> = IndexMap::from([
        (0, "zero".to_string()),
        (1, "one".to_string()),
        (2, "two".to_string()),
        (3, "three".to_string()),
        (4, "four".to_string()),
    ]);

    {
        let (index, key, value) = map.get_full_mut(&2).expect("key 2 should exist");
        assert_eq!(index, 2);
        assert_eq!(*key, 2);
        value.push_str("-mutated");
    }

    {
        let (key, value) = map.first_mut().expect("map has a first entry");
        assert_eq!(*key, 0);
        value.push_str("-first");
    }

    {
        let (key, value) = map.last_mut().expect("map has a last entry");
        assert_eq!(*key, 4);
        value.push_str("-last");
    }

    {
        let range = map.get_range(1..4).expect("valid immutable range");
        assert_eq!(range.len(), 3);
        assert_eq!(range.first().map(|(k, v)| (*k, v.as_str())), Some((1, "one")));
        assert_eq!(
            range.last().map(|(k, v)| (*k, v.as_str())),
            Some((3, "three"))
        );
    }

    {
        let range = map.get_range_mut(1..=2).expect("valid mutable range");
        for (_, value) in range.iter_mut() {
            value.push_str("-range");
        }
    }

    assert_eq!(map.get(&0).map(String::as_str), Some("zero-first"));
    assert_eq!(map.get(&2).map(String::as_str), Some("two-mutated-range"));
    assert_eq!(map.get(&4).map(String::as_str), Some("four-last"));
    assert!(map.get_range(4..2).is_none());
    assert!(map.get_range_mut(0..99).is_none());

    let removed_by_swap = map.remove_entry(&1);
    assert_eq!(removed_by_swap, Some((1, "one-range".to_string())));
    assert_eq!(map.len(), 4);

    map.sort_keys();
    let removed_by_shift = map.shift_remove_entry(&3);
    assert_eq!(removed_by_shift, Some((3, "three".to_string())));
    assert_eq!(
        entries(&map),
        vec![
            (0, "zero-first".to_string()),
            (2, "two-mutated-range".to_string()),
            (4, "four-last".to_string()),
        ]
    );

    let shifted_index = map.shift_remove_index(1);
    assert_eq!(shifted_index, Some((2, "two-mutated-range".to_string())));
    assert_eq!(
        entries(&map),
        vec![(0, "zero-first".to_string()), (4, "four-last".to_string())]
    );

    map.insert(6, "six".to_string());
    map.insert(8, "eight".to_string());
    map.insert(10, "ten".to_string());

    map.retain(|key, value| {
        if *key >= 6 {
            value.push_str("-kept");
        }
        *key == 0 || *key >= 8
    });

    assert_eq!(
        entries(&map),
        vec![
            (0, "zero-first".to_string()),
            (8, "eight-kept".to_string()),
            (10, "ten-kept".to_string()),
        ]
    );

    let replacements = [
        (12, "twelve".to_string()),
        (10, "TEN-updated".to_string()),
        (14, "fourteen".to_string()),
    ];
    let removed: Vec<_> = map.splice(1..2, replacements).collect();

    assert_eq!(removed, vec![(8, "eight-kept".to_string())]);
    assert_eq!(
        entries(&map),
        vec![
            (0, "zero-first".to_string()),
            (12, "twelve".to_string()),
            (14, "fourteen".to_string()),
            (10, "TEN-updated".to_string()),
        ]
    );
}

#[test]
fn sorting_searching_and_boxed_slice_workflow() {
    let mut map: IndexMap<i32, i32> = IndexMap::from([(3, 30), (1, 10), (4, 40), (2, 20)]);

    map.sort_keys();
    assert_eq!(
        map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        vec![(1, 10), (2, 20), (3, 30), (4, 40)]
    );

    assert_eq!(
        map.binary_search_by(|key, _| key.cmp(&3)),
        Ok(2)
    );
    assert_eq!(
        map.binary_search_by(|key, _| key.cmp(&5)),
        Err(4)
    );
    assert_eq!(
        map.binary_search_by_key(&20, |_, value| *value),
        Ok(1)
    );
    assert_eq!(
        map.partition_point(|key, _| *key < 3),
        2
    );

    map.sort_by(|left_key, left_value, right_key, right_value| {
        right_value
            .cmp(left_value)
            .then_with(|| left_key.cmp(right_key))
    });
    assert_eq!(
        map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        vec![(4, 40), (3, 30), (2, 20), (1, 10)]
    );

    let sorted_by_value_then_key: Vec<_> = map
        .clone()
        .sorted_by(|left_key, left_value, right_key, right_value| {
            left_value
                .cmp(right_value)
                .then_with(|| left_key.cmp(right_key))
        })
        .collect();
    assert_eq!(
        sorted_by_value_then_key,
        vec![(1, 10), (2, 20), (3, 30), (4, 40)]
    );

    map.sort_unstable_keys();
    assert_eq!(
        map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        vec![(1, 10), (2, 20), (3, 30), (4, 40)]
    );

    map.sort_unstable_by(|left_key, left_value, right_key, right_value| {
        match (left_value % 30).cmp(&(right_value % 30)) {
            Ordering::Equal => left_key.cmp(right_key),
            ordering => ordering,
        }
    });
    assert_eq!(
        map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        vec![(3, 30), (1, 10), (4, 40), (2, 20)]
    );

    let sorted_unstable_desc: Vec<_> = map
        .clone()
        .sorted_unstable_by(|left_key, _, right_key, _| right_key.cmp(left_key))
        .collect();
    assert_eq!(
        sorted_unstable_desc,
        vec![(4, 40), (3, 30), (2, 20), (1, 10)]
    );

    map.sort_by_cached_key(|key, value| (value % 25, *key));
    assert_eq!(
        map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        vec![(3, 30), (1, 10), (4, 40), (2, 20)]
    );

    let boxed = map.into_boxed_slice();
    assert_eq!(boxed.len(), 4);
    assert_eq!(boxed.first().map(|(k, v)| (*k, *v)), Some((3, 30)));
    assert_eq!(boxed.get_index(2).map(|(k, v)| (*k, *v)), Some((4, 40)));
}