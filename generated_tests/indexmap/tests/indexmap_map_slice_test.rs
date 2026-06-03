use indexmap::map::Slice;
use indexmap::IndexMap;
use std::cmp::Ordering;

fn entries(slice: &Slice<i32, String>) -> Vec<(i32, String)> {
    slice.iter().map(|(k, v)| (*k, v.clone())).collect()
}

#[test]
fn empty_map_slice_new_mut_supports_ranges_splits_and_searches() {
    let empty: &mut Slice<i32, String> = Slice::new_mut();

    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert!(empty.first_mut().is_none());
    assert!(empty.last_mut().is_none());
    assert!(empty.split_first().is_none());
    assert!(empty.split_first_mut().is_none());
    assert!(empty.split_last().is_none());
    assert!(empty.split_last_mut().is_none());

    let zero_range = empty.get_range(0..0).expect("0..0 is valid for an empty slice");
    assert!(zero_range.is_empty());

    let zero_range_mut = empty
        .get_range_mut(..)
        .expect("full range is valid for an empty mutable slice");
    assert!(zero_range_mut.is_empty());

    let (left, right) = empty.split_at(0);
    assert!(left.is_empty());
    assert!(right.is_empty());

    let (left_mut, right_mut) = empty.split_at_mut(0);
    assert!(left_mut.is_empty());
    assert!(right_mut.is_empty());

    assert_eq!(
        empty.binary_search_by(|key, _value| key.cmp(&10)),
        Err(0)
    );
    assert_eq!(
        empty.binary_search_by_key(&"missing", |_key, value| value.as_str()),
        Err(0)
    );
    assert_eq!(empty.partition_point(|key, _value| *key < 10), 0);
    assert!(empty.get_range(1..1).is_none());
    assert!(empty.get_range_mut(1..1).is_none());
}

#[test]
fn map_slice_ranges_splits_and_mutations_form_a_workflow() {
    let mut map: IndexMap<i32, String> = IndexMap::new();
    for (key, value) in [
        (10, "ten"),
        (20, "twenty"),
        (30, "thirty"),
        (40, "forty"),
        (50, "fifty"),
        (60, "sixty"),
    ] {
        assert_eq!(map.insert(key, value.to_string()), None);
    }

    {
        let slice = map.as_slice();

        assert_eq!(
            entries(slice),
            vec![
                (10, "ten".to_string()),
                (20, "twenty".to_string()),
                (30, "thirty".to_string()),
                (40, "forty".to_string()),
                (50, "fifty".to_string()),
                (60, "sixty".to_string()),
            ]
        );

        let middle = slice.get_range(1..5).expect("1..5 is a valid range");
        assert_eq!(
            entries(middle),
            vec![
                (20, "twenty".to_string()),
                (30, "thirty".to_string()),
                (40, "forty".to_string()),
                (50, "fifty".to_string()),
            ]
        );

        let prefix = slice.get_range(..2).expect("..2 is a valid range");
        let suffix = slice.get_range(4..).expect("4.. is a valid range");
        assert_eq!(entries(prefix), vec![(10, "ten".to_string()), (20, "twenty".to_string())]);
        assert_eq!(entries(suffix), vec![(50, "fifty".to_string()), (60, "sixty".to_string())]);

        assert!(slice.get_range(7..8).is_none());
        assert!(slice.get_range(4..2).is_none());

        let (left, right) = slice.split_at(3);
        assert_eq!(
            entries(left),
            vec![
                (10, "ten".to_string()),
                (20, "twenty".to_string()),
                (30, "thirty".to_string()),
            ]
        );
        assert_eq!(
            entries(right),
            vec![
                (40, "forty".to_string()),
                (50, "fifty".to_string()),
                (60, "sixty".to_string()),
            ]
        );

        let ((first_key, first_value), rest) =
            slice.split_first().expect("non-empty slice has a first element");
        assert_eq!((*first_key, first_value.as_str()), (10, "ten"));
        assert_eq!(
            entries(rest),
            vec![
                (20, "twenty".to_string()),
                (30, "thirty".to_string()),
                (40, "forty".to_string()),
                (50, "fifty".to_string()),
                (60, "sixty".to_string()),
            ]
        );

        let ((last_key, last_value), before_last) =
            slice.split_last().expect("non-empty slice has a last element");
        assert_eq!((*last_key, last_value.as_str()), (60, "sixty"));
        assert_eq!(
            entries(before_last),
            vec![
                (10, "ten".to_string()),
                (20, "twenty".to_string()),
                (30, "thirty".to_string()),
                (40, "forty".to_string()),
                (50, "fifty".to_string()),
            ]
        );
    }

    {
        let slice = map.as_mut_slice();

        let first = slice.first_mut().expect("first element is present");
        assert_eq!(*first.0, 10);
        first.1.push_str("-first");

        let last = slice.last_mut().expect("last element is present");
        assert_eq!(*last.0, 60);
        last.1.push_str("-last");
    }

    {
        let slice = map.as_mut_slice();

        let middle = slice
            .get_range_mut(2..4)
            .expect("2..4 is a valid mutable range");
        for (_key, value) in middle.iter_mut() {
            value.push_str("-middle");
        }

        assert!(slice.get_range_mut(8..9).is_none());
    }

    {
        let slice = map.as_mut_slice();

        let (left, right) = slice.split_at_mut(3);
        left.last_mut()
            .expect("left side has a last element")
            .1
            .push_str("-left");
        right
            .first_mut()
            .expect("right side has a first element")
            .1
            .push_str("-right");
    }

    {
        let slice = map.as_mut_slice();

        let ((first_key, first_value), rest) = slice
            .split_first_mut()
            .expect("mutable non-empty slice has a first element");
        assert_eq!(*first_key, 10);
        first_value.push_str("-split-first");
        rest.first_mut()
            .expect("rest after first has an element")
            .1
            .push_str("-rest-first");
    }

    {
        let slice = map.as_mut_slice();

        let ((last_key, last_value), before_last) = slice
            .split_last_mut()
            .expect("mutable non-empty slice has a last element");
        assert_eq!(*last_key, 60);
        last_value.push_str("-split-last");
        before_last
            .last_mut()
            .expect("prefix before last has an element")
            .1
            .push_str("-before-last");
    }

    assert_eq!(map.get(&10).map(String::as_str), Some("ten-first-split-first"));
    assert_eq!(map.get(&20).map(String::as_str), Some("twenty-rest-first"));
    assert_eq!(map.get(&30).map(String::as_str), Some("thirty-middle-left"));
    assert_eq!(map.get(&40).map(String::as_str), Some("forty-middle-right"));
    assert_eq!(map.get(&50).map(String::as_str), Some("fifty-before-last"));
    assert_eq!(map.get(&60).map(String::as_str), Some("sixty-last-split-last"));
}

#[test]
fn sorted_map_slice_binary_search_and_partition_point_use_keys_and_values() {
    let mut map: IndexMap<i32, String> = IndexMap::new();
    for (key, value) in [
        (5, "ant"),
        (10, "bee"),
        (15, "cat"),
        (20, "dog"),
        (25, "eel"),
        (30, "fox"),
    ] {
        map.insert(key, value.to_string());
    }

    let slice = map.as_slice();

    assert_eq!(
        slice.binary_search_by(|key, _value| key.cmp(&20)),
        Ok(3)
    );
    assert_eq!(
        slice.binary_search_by(|key, _value| {
            if *key < 18 {
                Ordering::Less
            } else if *key > 18 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }),
        Err(3)
    );

    assert_eq!(
        slice.binary_search_by_key(&"dog", |_key, value| value.as_str()),
        Ok(3)
    );
    assert_eq!(
        slice.binary_search_by_key(&"cow", |_key, value| value.as_str()),
        Err(3)
    );

    let first_key_ge_20 = slice.partition_point(|key, _value| *key < 20);
    assert_eq!(first_key_ge_20, 3);
    assert_eq!(slice.get_index(first_key_ge_20).map(|(key, _)| *key), Some(20));

    let short_value_boundary = slice.partition_point(|key, value| *key <= 25 && value.len() == 3);
    assert_eq!(short_value_boundary, 5);
    assert_eq!(slice.get_index(short_value_boundary).map(|(key, _)| *key), Some(30));
}