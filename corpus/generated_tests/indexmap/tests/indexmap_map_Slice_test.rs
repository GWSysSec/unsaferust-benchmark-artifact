use indexmap::map::Slice;
use indexmap::IndexMap;
use std::cmp::Ordering;

fn slice_entries(slice: &Slice<i32, String>) -> Vec<(i32, String)> {
    slice.iter().map(|(k, v)| (*k, v.clone())).collect()
}

#[test]
fn empty_mutable_slice_has_expected_edge_case_behavior() {
    let empty: &mut Slice<i32, String> = Slice::new_mut();

    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    assert!(empty.first_mut().is_none());
    assert!(empty.last_mut().is_none());
    assert!(empty.split_first().is_none());
    assert!(empty.split_first_mut().is_none());
    assert!(empty.split_last().is_none());
    assert!(empty.split_last_mut().is_none());

    let (left, right) = empty.split_at(0);
    assert!(left.is_empty());
    assert!(right.is_empty());

    let (left_mut, right_mut) = empty.split_at_mut(0);
    assert!(left_mut.is_empty());
    assert!(right_mut.is_empty());

    assert!(empty.get_range(0..0).expect("empty range is valid").is_empty());
    assert!(empty
        .get_range_mut(0..0)
        .expect("empty mutable range is valid")
        .is_empty());
    assert!(empty.get_range(1..1).is_none());
}

#[test]
fn slice_ranges_and_split_mutations_preserve_order_and_update_values() {
    let mut map: IndexMap<i32, String> = IndexMap::new();
    for (key, value) in [
        (10, "ten"),
        (20, "twenty"),
        (30, "thirty"),
        (40, "forty"),
        (50, "fifty"),
    ] {
        assert_eq!(map.insert(key, value.to_string()), None);
    }

    {
        let slice = map.as_mut_slice();

        let (_, first_value) = slice.first_mut().expect("first item should exist");
        first_value.push_str("-first");

        let (_, last_value) = slice.last_mut().expect("last item should exist");
        last_value.push_str("-last");

        let middle = slice
            .get_range(1..4)
            .expect("middle range should be in bounds");
        assert_eq!(
            slice_entries(middle),
            vec![
                (20, "twenty".to_string()),
                (30, "thirty".to_string()),
                (40, "forty".to_string()),
            ]
        );

        let middle_mut = slice
            .get_range_mut(1..4)
            .expect("middle mutable range should be in bounds");
        for (_, value) in middle_mut.iter_mut() {
            value.push_str("-middle");
        }

        assert!(slice.get_range(4..2).is_none());
        assert!(slice.get_range_mut(0..6).is_none());

        let (left, right) = slice.split_at(2);
        assert_eq!(
            slice_entries(left),
            vec![
                (10, "ten-first".to_string()),
                (20, "twenty-middle".to_string()),
            ]
        );
        assert_eq!(
            slice_entries(right),
            vec![
                (30, "thirty-middle".to_string()),
                (40, "forty-middle".to_string()),
                (50, "fifty-last".to_string()),
            ]
        );

        let (left_mut, right_mut) = slice.split_at_mut(3);
        let (_, left_last_value) = left_mut
            .last_mut()
            .expect("left side should have a last value");
        left_last_value.push_str("-left");

        let (_, right_first_value) = right_mut
            .first_mut()
            .expect("right side should have a first value");
        right_first_value.push_str("-right");
    }

    assert_eq!(
        map.iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (10, "ten-first"),
            (20, "twenty-middle"),
            (30, "thirty-middle-left"),
            (40, "forty-middle-right"),
            (50, "fifty-last"),
        ]
    );
}

#[test]
fn split_first_and_split_last_variants_expose_remainder_slices() {
    let mut map: IndexMap<i32, String> = IndexMap::new();
    for (key, value) in [(1, "one"), (2, "two"), (3, "three"), (4, "four")] {
        map.insert(key, value.to_string());
    }

    {
        let slice = map.as_slice();

        let ((first_key, first_value), rest) =
            slice.split_first().expect("nonempty slice has first item");
        assert_eq!((*first_key, first_value.as_str()), (1, "one"));
        assert_eq!(
            slice_entries(rest),
            vec![
                (2, "two".to_string()),
                (3, "three".to_string()),
                (4, "four".to_string()),
            ]
        );

        let ((last_key, last_value), rest) =
            slice.split_last().expect("nonempty slice has last item");
        assert_eq!((*last_key, last_value.as_str()), (4, "four"));
        assert_eq!(
            slice_entries(rest),
            vec![
                (1, "one".to_string()),
                (2, "two".to_string()),
                (3, "three".to_string()),
            ]
        );
    }

    {
        let slice = map.as_mut_slice();

        let ((first_key, first_value), rest) = slice
            .split_first_mut()
            .expect("nonempty mutable slice has first item");
        assert_eq!(*first_key, 1);
        first_value.push_str("-updated");
        assert_eq!(rest.len(), 3);
        let (_, rest_first_value) = rest
            .first_mut()
            .expect("remainder after first should have first item");
        rest_first_value.push_str("-after-first");
    }

    {
        let slice = map.as_mut_slice();

        let ((last_key, last_value), rest) = slice
            .split_last_mut()
            .expect("nonempty mutable slice has last item");
        assert_eq!(*last_key, 4);
        last_value.push_str("-updated");
        assert_eq!(rest.len(), 3);
        let (_, rest_last_value) = rest
            .last_mut()
            .expect("remainder before last should have last item");
        rest_last_value.push_str("-before-last");
    }

    assert_eq!(
        map.iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (1, "one-updated"),
            (2, "two-after-first"),
            (3, "three-before-last"),
            (4, "four-updated"),
        ]
    );
}

#[test]
fn sorted_slice_search_and_partition_point_match_expected_indices() {
    let mut map: IndexMap<i32, String> = IndexMap::new();
    for key in [30, 10, 50, 20, 40] {
        map.insert(key, format!("value-{key}"));
    }
    map.sort_keys();

    let slice = map.as_slice();
    assert_eq!(
        slice.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
        vec![10, 20, 30, 40, 50]
    );

    assert_eq!(
        slice.binary_search_by(|key, _| key.cmp(&30)),
        Ok(2),
        "existing key should be found at its sorted index"
    );
    assert_eq!(
        slice.binary_search_by(|key, _| {
            if *key < 35 {
                Ordering::Less
            } else if *key > 35 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }),
        Err(3),
        "missing key should report the insertion index"
    );

    assert_eq!(
        slice.binary_search_by_key(&"value-40".to_string(), |_, value| value.clone()),
        Ok(3)
    );
    assert_eq!(
        slice.binary_search_by_key(&25, |key, _| *key),
        Err(2),
        "25 belongs between 20 and 30"
    );

    assert_eq!(slice.partition_point(|key, _| *key < 35), 3);
    assert_eq!(slice.partition_point(|_, value| value.as_str() <= "value-30"), 3);
    assert_eq!(slice.partition_point(|key, _| *key < 5), 0);
    assert_eq!(slice.partition_point(|key, _| *key <= 50), slice.len());
}