use indexmap::set::Slice;
use indexmap::IndexSet;
use std::cmp::Ordering;

fn values(slice: &Slice<i32>) -> Vec<i32> {
    slice.iter().copied().collect()
}

#[test]
fn set_slice_ranges_splits_and_searches_after_ordered_workflow() {
    let mut set: IndexSet<i32> = IndexSet::with_capacity(10);
    for value in [5, 10, 15, 20, 25, 30, 35, 40] {
        assert!(set.insert(value), "new value {value} should be inserted");
    }
    assert!(!set.insert(20), "duplicate insertion should not change the set");
    assert_eq!(set.len(), 8);

    let slice = set.as_slice();
    assert_eq!(values(slice), vec![5, 10, 15, 20, 25, 30, 35, 40]);

    let middle = slice.get_range(2..6).expect("2..6 is in bounds");
    assert_eq!(values(middle), vec![15, 20, 25, 30]);

    let full = slice.get_range(..).expect("full range is valid");
    assert_eq!(values(full), vec![5, 10, 15, 20, 25, 30, 35, 40]);

    let empty_at_end = slice.get_range(8..8).expect("empty range at len is valid");
    assert!(empty_at_end.is_empty());

    assert!(slice.get_range(9..9).is_none());
    assert!(slice.get_range(6..2).is_none());

    let (front, back) = slice.split_at(3);
    assert_eq!(values(front), vec![5, 10, 15]);
    assert_eq!(values(back), vec![20, 25, 30, 35, 40]);

    let (empty_left, all_right) = slice.split_at(0);
    assert!(empty_left.is_empty());
    assert_eq!(values(all_right), values(slice));

    let (all_left, empty_right) = slice.split_at(slice.len());
    assert_eq!(values(all_left), values(slice));
    assert!(empty_right.is_empty());

    let (first, rest_after_first) = slice.split_first().expect("non-empty slice has a first item");
    assert_eq!(*first, 5);
    assert_eq!(values(rest_after_first), vec![10, 15, 20, 25, 30, 35, 40]);

    let (last, rest_before_last) = slice.split_last().expect("non-empty slice has a last item");
    assert_eq!(*last, 40);
    assert_eq!(values(rest_before_last), vec![5, 10, 15, 20, 25, 30, 35]);

    assert_eq!(slice.binary_search(&5), Ok(0));
    assert_eq!(slice.binary_search(&25), Ok(4));
    assert_eq!(slice.binary_search(&40), Ok(7));
    assert_eq!(slice.binary_search(&22), Err(4));
    assert_eq!(slice.binary_search(&50), Err(8));

    assert_eq!(slice.binary_search_by(|probe| probe.cmp(&30)), Ok(5));
    assert_eq!(slice.binary_search_by(|probe| probe.cmp(&12)), Err(2));

    assert_eq!(slice.binary_search_by_key(&0, |probe| probe / 10), Ok(0));
    assert_eq!(slice.binary_search_by_key(&4, |probe| probe / 10), Ok(7));
    assert_eq!(slice.binary_search_by_key(&9, |probe| probe / 10), Err(8));

    assert_eq!(slice.partition_point(|value| *value < 25), 4);
    assert_eq!(slice.partition_point(|value| *value <= 40), 8);
    assert_eq!(slice.partition_point(|value| *value < 0), 0);
}

#[test]
fn empty_set_slice_edge_cases_are_consistent() {
    let empty: &Slice<i32> = Slice::new();

    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());

    let empty_range = empty.get_range(0..0).expect("0..0 is valid for an empty slice");
    assert!(empty_range.is_empty());
    assert!(empty.get_range(1..1).is_none());

    let (left, right) = empty.split_at(0);
    assert!(left.is_empty());
    assert!(right.is_empty());

    assert!(empty.split_first().is_none());
    assert!(empty.split_last().is_none());

    assert_eq!(empty.binary_search(&10), Err(0));
    assert_eq!(
        empty.binary_search_by(|probe| {
            if *probe < 10 {
                Ordering::Less
            } else if *probe > 10 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }),
        Err(0)
    );
    assert_eq!(empty.binary_search_by_key(&10, |probe| *probe), Err(0));
    assert_eq!(empty.partition_point(|value| *value < 10), 0);
}