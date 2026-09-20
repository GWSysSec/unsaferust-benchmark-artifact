use indexmap::set::Slice;
use indexmap::IndexSet;
use std::cmp::Ordering;

fn slice_values(slice: &Slice<i32>) -> Vec<i32> {
    slice.iter().copied().collect()
}

#[test]
fn set_slice_range_split_and_search_workflow() {
    let mut set: IndexSet<i32> = IndexSet::new();
    for value in [10, 20, 30, 40, 50, 60, 70] {
        assert!(set.insert(value));
    }
    assert!(!set.insert(30), "duplicate insertion should not change the set");

    let slice = set.as_slice();
    assert_eq!(slice_values(slice), vec![10, 20, 30, 40, 50, 60, 70]);

    let middle = slice.get_range(2..5).expect("2..5 is a valid range");
    assert_eq!(slice_values(middle), vec![30, 40, 50]);

    let prefix = slice.get_range(..3).expect("..3 is a valid range");
    assert_eq!(slice_values(prefix), vec![10, 20, 30]);

    let suffix = slice.get_range(4..).expect("4.. is a valid range");
    assert_eq!(slice_values(suffix), vec![50, 60, 70]);

    assert!(slice.get_range(8..9).is_none());
    assert!(slice.get_range(5..2).is_none());

    let (left, right) = slice.split_at(4);
    assert_eq!(slice_values(left), vec![10, 20, 30, 40]);
    assert_eq!(slice_values(right), vec![50, 60, 70]);

    let (first, rest) = slice.split_first().expect("non-empty slice has first value");
    assert_eq!(*first, 10);
    assert_eq!(slice_values(rest), vec![20, 30, 40, 50, 60, 70]);

    let (last, rest) = slice.split_last().expect("non-empty slice has last value");
    assert_eq!(*last, 70);
    assert_eq!(slice_values(rest), vec![10, 20, 30, 40, 50, 60]);

    assert_eq!(slice.binary_search(&40), Ok(3));
    assert_eq!(slice.binary_search(&35), Err(3));
    assert_eq!(slice.binary_search(&5), Err(0));
    assert_eq!(slice.binary_search(&90), Err(7));

    assert_eq!(
        slice.binary_search_by(|value| {
            if *value < 50 {
                Ordering::Less
            } else if *value > 50 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }),
        Ok(4)
    );

    assert_eq!(
        slice.binary_search_by(|value| {
            if *value < 55 {
                Ordering::Less
            } else if *value > 55 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }),
        Err(5)
    );

    assert_eq!(slice.binary_search_by_key(&6, |value| value / 10), Ok(5));
    assert_eq!(slice.binary_search_by_key(&8, |value| value / 10), Err(7));

    assert_eq!(slice.partition_point(|value| *value < 45), 4);
    assert_eq!(slice.partition_point(|value| *value <= 70), 7);
    assert_eq!(slice.partition_point(|value| *value < 10), 0);
}

#[test]
fn empty_and_singleton_set_slices_have_expected_boundaries() {
    let empty: &Slice<i32> = Slice::new();

    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert!(empty.split_first().is_none());
    assert!(empty.split_last().is_none());

    let (left, right) = empty.split_at(0);
    assert!(left.is_empty());
    assert!(right.is_empty());

    let empty_range = empty.get_range(0..0).expect("empty range at zero is valid");
    assert!(empty_range.is_empty());
    assert!(empty.get_range(1..1).is_none());

    assert_eq!(empty.binary_search(&10), Err(0));
    assert_eq!(empty.binary_search_by(|_| Ordering::Equal), Err(0));
    assert_eq!(empty.binary_search_by_key(&1, |value| *value), Err(0));
    assert_eq!(empty.partition_point(|_| true), 0);

    let mut set: IndexSet<i32> = IndexSet::new();
    assert!(set.insert(42));
    let slice = set.as_slice();

    let only = slice.get_range(..).expect("full range is valid");
    assert_eq!(slice_values(only), vec![42]);

    let (left, right) = slice.split_at(1);
    assert_eq!(slice_values(left), vec![42]);
    assert!(right.is_empty());

    let (first, rest) = slice.split_first().expect("singleton has first value");
    assert_eq!(*first, 42);
    assert!(rest.is_empty());

    let (last, rest) = slice.split_last().expect("singleton has last value");
    assert_eq!(*last, 42);
    assert!(rest.is_empty());

    assert_eq!(slice.binary_search(&42), Ok(0));
    assert_eq!(slice.binary_search(&41), Err(0));
    assert_eq!(slice.binary_search(&43), Err(1));
    assert_eq!(slice.partition_point(|value| *value < 42), 0);
    assert_eq!(slice.partition_point(|value| *value <= 42), 1);
}