#![allow(deprecated)]

use indexmap::IndexSet;
use std::cmp::Ordering;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

fn values(set: &IndexSet<i32>) -> Vec<i32> {
    set.iter().copied().collect()
}

fn slice_values(slice: &indexmap::set::Slice<i32>) -> Vec<i32> {
    slice.iter().copied().collect()
}

#[test]
fn capacity_hasher_truncate_split_append_splice_and_clear_workflow() {
    let mut set: IndexSet<i32, RandomState> =
        IndexSet::with_capacity_and_hasher(2, RandomState::new());

    let _built_hasher = set.hasher().build_hasher();

    set.reserve_exact(8);
    assert!(set.capacity() >= 8);

    set.try_reserve_exact(4)
        .expect("small exact reservation should succeed");
    assert!(set.capacity() >= 4);

    for value in [10, 20, 30, 40, 50, 60] {
        assert!(set.insert(value));
    }
    assert_eq!(values(&set), vec![10, 20, 30, 40, 50, 60]);

    set.truncate(4);
    assert_eq!(set.len(), 4);
    assert_eq!(values(&set), vec![10, 20, 30, 40]);

    set.truncate(10);
    assert_eq!(values(&set), vec![10, 20, 30, 40]);

    let mut tail = set.split_off(2);
    assert_eq!(values(&set), vec![10, 20]);
    assert_eq!(values(&tail), vec![30, 40]);

    assert!(tail.insert(50));
    assert!(tail.insert(60));
    set.append(&mut tail);
    assert!(tail.is_empty());
    assert_eq!(values(&set), vec![10, 20, 30, 40, 50, 60]);

    let replaced: Vec<i32> = set.splice(1..4, [21, 31, 41]).collect();
    assert_eq!(replaced, vec![20, 30, 40]);
    assert_eq!(values(&set), vec![10, 21, 31, 41, 50, 60]);

    let range = set.get_range(1..4).expect("valid range should produce a slice");
    assert_eq!(slice_values(range), vec![21, 31, 41]);
    assert!(set.get_range(10..12).is_none());

    assert_eq!(set.shift_remove_index(2), Some(31));
    assert_eq!(values(&set), vec![10, 21, 41, 50, 60]);
    assert_eq!(set.shift_remove_index(99), None);

    set.clear();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn take_swap_take_shift_take_and_retain_preserve_expected_membership_and_order() {
    let mut set: IndexSet<i32> = [1, 2, 3, 4, 5, 6].into_iter().collect();

    assert_eq!(set.take(&3), Some(3));
    assert!(!set.contains(&3));
    assert_eq!(values(&set), vec![1, 2, 6, 4, 5]);

    assert_eq!(set.swap_take(&2), Some(2));
    assert!(!set.contains(&2));
    assert_eq!(set.len(), 4);
    assert_eq!(values(&set), vec![1, 5, 6, 4]);
    assert!(set.contains(&1));
    assert!(set.contains(&4));
    assert!(set.contains(&5));
    assert!(set.contains(&6));

    set.sort();
    assert_eq!(values(&set), vec![1, 4, 5, 6]);

    assert_eq!(set.shift_take(&4), Some(4));
    assert_eq!(values(&set), vec![1, 5, 6]);
    assert_eq!(set.shift_take(&99), None);

    set.retain(|value| value % 2 == 1);
    assert_eq!(values(&set), vec![1, 5]);

    assert!(set.insert(7));
    assert!(set.insert(9));
    set.retain(|value| *value <= 7);
    assert_eq!(values(&set), vec![1, 5, 7]);
}

#[test]
fn sorting_and_searching_workflow_on_ordered_sets() {
    let mut set: IndexSet<i32> = [40, 10, 30, 20, 50].into_iter().collect();

    set.sort();
    assert_eq!(values(&set), vec![10, 20, 30, 40, 50]);
    assert_eq!(set.binary_search(&30), Ok(2));
    assert_eq!(set.binary_search(&35), Err(3));

    let less_than_35 = set.partition_point(|value| *value < 35);
    assert_eq!(less_than_35, 3);

    assert_eq!(set.binary_search_by(|value| value.cmp(&40)), Ok(3));
    assert_eq!(set.binary_search_by_key(&5, |value| value / 10), Ok(4));

    set.sort_by(|left, right| right.cmp(left));
    assert_eq!(values(&set), vec![50, 40, 30, 20, 10]);

    let sorted_by_distance: Vec<i32> = set
        .clone()
        .sorted_by(|left, right| {
            let left_distance = (left - 32).abs();
            let right_distance = (right - 32).abs();
            left_distance
                .cmp(&right_distance)
                .then_with(|| left.cmp(right))
        })
        .collect();
    assert_eq!(sorted_by_distance, vec![30, 40, 20, 50, 10]);

    set.sort_by_cached_key(|value| (value - 25).abs());
    assert_eq!(values(&set), vec![30, 20, 40, 10, 50]);

    set.sort_unstable();
    assert_eq!(values(&set), vec![10, 20, 30, 40, 50]);

    set.sort_unstable_by(|left, right| {
        let left_parity = left % 2;
        let right_parity = right % 2;
        left_parity
            .cmp(&right_parity)
            .then_with(|| right.cmp(left))
    });
    assert_eq!(values(&set), vec![50, 40, 30, 20, 10]);

    let unstable_sorted: Vec<i32> = set
        .clone()
        .sorted_unstable_by(|left, right| {
            let by_last_digit = (left % 10).cmp(&(right % 10));
            if by_last_digit == Ordering::Equal {
                left.cmp(right)
            } else {
                by_last_digit
            }
        })
        .collect();
    assert_eq!(unstable_sorted, vec![10, 20, 30, 40, 50]);
}

#[test]
fn boxed_slice_and_slice_range_workflow() {
    let mut set: IndexSet<i32> = [3, 1, 4, 1, 5, 9, 2].into_iter().collect();
    assert_eq!(values(&set), vec![3, 1, 4, 5, 9, 2]);

    set.sort();
    assert_eq!(values(&set), vec![1, 2, 3, 4, 5, 9]);

    let middle = set.get_range(2..5).expect("middle range exists");
    assert_eq!(slice_values(middle), vec![3, 4, 5]);
    assert_eq!(middle.binary_search(&4), Ok(1));
    assert_eq!(middle.partition_point(|value| *value < 5), 2);

    let boxed = set.into_boxed_slice();
    assert_eq!(boxed.len(), 6);
    assert_eq!(boxed.first(), Some(&1));
    assert_eq!(boxed.last(), Some(&9));

    let boxed_values: Vec<i32> = boxed.iter().copied().collect();
    assert_eq!(boxed_values, vec![1, 2, 3, 4, 5, 9]);
}