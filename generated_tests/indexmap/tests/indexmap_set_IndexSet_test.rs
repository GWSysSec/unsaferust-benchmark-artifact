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
fn capacity_hasher_truncate_split_append_and_clear_workflow() {
    let mut set: IndexSet<i32, RandomState> =
        IndexSet::with_capacity_and_hasher(2, RandomState::new());

    let _state = set.hasher().build_hasher();

    set.reserve_exact(10);
    assert!(set.capacity() >= 10);

    set.try_reserve_exact(5)
        .expect("small exact reservation should succeed");
    assert!(set.capacity() >= 5);

    for value in [10, 20, 30, 40, 50, 60] {
        assert!(set.insert(value));
    }

    set.truncate(4);
    assert_eq!(set.len(), 4);
    assert_eq!(values(&IndexSet::from_iter(set.iter().copied())), vec![10, 20, 30, 40]);

    set.truncate(10);
    assert_eq!(set.len(), 4);

    let original_capacity = set.capacity();
    let mut tail = set.split_off(2);
    assert_eq!(values(&IndexSet::from_iter(set.iter().copied())), vec![10, 20]);
    assert_eq!(values(&IndexSet::from_iter(tail.iter().copied())), vec![30, 40]);
    assert_eq!(set.capacity(), original_capacity);

    let tail_capacity = tail.capacity();
    set.append(&mut tail);
    assert_eq!(values(&IndexSet::from_iter(set.iter().copied())), vec![10, 20, 30, 40]);
    assert!(tail.is_empty());
    assert_eq!(tail.capacity(), tail_capacity);

    let mut other = IndexSet::from([20, 70, 80]);
    set.append(&mut other);
    assert_eq!(values(&IndexSet::from_iter(set.iter().copied())), vec![10, 20, 30, 40, 70, 80]);
    assert!(other.is_empty());

    set.clear();
    assert!(set.is_empty());
    assert_eq!(set.capacity(), original_capacity.max(set.capacity()));
}

#[test]
fn splice_range_and_index_removal_preserve_expected_order() {
    let mut set = IndexSet::from([0, 1, 2, 3, 4]);

    let removed: Vec<_> = set.splice(2..4, [5, 4, 3, 2, 1]).collect();
    assert_eq!(removed, vec![2, 3]);
    assert_eq!(values(&set), vec![0, 1, 5, 3, 2, 4]);

    let middle = set.get_range(1..4).expect("valid range should produce a slice");
    assert_eq!(slice_values(middle), vec![1, 5, 3]);

    assert!(set.get_range(7..8).is_none());

    let shifted = set.shift_remove_index(2);
    assert_eq!(shifted, Some(5));
    assert_eq!(values(&set), vec![0, 1, 3, 2, 4]);

    assert_eq!(set.shift_remove_index(set.len()), None);

    let boxed = set.into_boxed_slice();
    assert_eq!(boxed.len(), 5);
    assert_eq!(boxed.iter().copied().collect::<Vec<_>>(), vec![0, 1, 3, 2, 4]);
}

#[test]
fn take_swap_take_shift_take_and_retain_have_distinct_order_effects() {
    let mut set = IndexSet::from([1, 2, 3, 4, 5]);

    assert_eq!(set.shift_take(&2), Some(2));
    assert_eq!(values(&set), vec![1, 3, 4, 5]);

    assert_eq!(set.swap_take(&3), Some(3));
    assert_eq!(set.len(), 3);
    assert!(set.contains(&1));
    assert!(set.contains(&4));
    assert!(set.contains(&5));
    assert!(!set.contains(&3));

    assert_eq!(set.take(&1), Some(1));
    assert_eq!(set.take(&99), None);
    assert!(!set.contains(&1));

    set.extend([6, 7, 8, 9]);
    set.retain(|value| value % 2 == 0);
    assert_eq!(values(&set), vec![4, 6, 8]);
}

#[test]
fn sorting_and_binary_search_workflows() {
    let mut set = IndexSet::from([5, 1, 4, 2, 3]);

    set.sort();
    assert_eq!(values(&set), vec![1, 2, 3, 4, 5]);
    assert_eq!(set.binary_search(&3), Ok(2));
    assert_eq!(set.binary_search(&6), Err(5));
    assert_eq!(
        set.binary_search_by(|probe| probe.cmp(&4)),
        Ok(3)
    );
    assert_eq!(
        set.binary_search_by_key(&2, |probe| *probe),
        Ok(1)
    );
    assert_eq!(set.partition_point(|probe| *probe < 4), 3);

    set.sort_by(|a, b| b.cmp(a));
    assert_eq!(values(&set), vec![5, 4, 3, 2, 1]);

    let descending_search = set.binary_search_by(|probe| {
        if *probe == 4 {
            Ordering::Equal
        } else if *probe > 4 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    });
    assert_eq!(descending_search, Ok(1));

    let sorted_by_abs_distance: Vec<_> = IndexSet::<i32>::from([-3, 2, -1, 4])
        .sorted_by(|a, b| (*a).abs().cmp(&(*b).abs()).then_with(|| a.cmp(b)))
        .collect();
    assert_eq!(sorted_by_abs_distance, vec![-1, 2, -3, 4]);

    let sorted_unstable_desc: Vec<_> = IndexSet::from([10, 7, 9, 8])
        .sorted_unstable_by(|a, b| b.cmp(a))
        .collect();
    assert_eq!(sorted_unstable_desc, vec![10, 9, 8, 7]);
}

#[test]
fn unstable_and_cached_key_sorting_cover_reordering_cases() {
    let mut set = IndexSet::from(["pear", "fig", "banana", "apple"]);

    set.sort_by_cached_key(|value| (value.len(), *value));
    assert_eq!(
        set.iter().copied().collect::<Vec<_>>(),
        vec!["fig", "pear", "apple", "banana"]
    );

    set.sort_unstable();
    assert_eq!(
        set.iter().copied().collect::<Vec<_>>(),
        vec!["apple", "banana", "fig", "pear"]
    );

    set.sort_unstable_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    assert_eq!(
        set.iter().copied().collect::<Vec<_>>(),
        vec!["banana", "apple", "pear", "fig"]
    );

    assert_eq!(set.partition_point(|value| value.len() >= 5), 2);
}