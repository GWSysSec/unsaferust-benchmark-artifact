#[cfg(feature = "rayon")]
use indexmap::rayon::prelude::*;
#[cfg(feature = "rayon")]
use indexmap::IndexSet;
#[cfg(feature = "rayon")]
use std::cmp::Ordering;

#[cfg(feature = "rayon")]
fn sorted_vec<I>(iter: I) -> Vec<i32>
where
    I: IntoIterator<Item = i32>,
{
    let mut values: Vec<i32> = iter.into_iter().collect();
    values.sort();
    values
}

#[cfg(feature = "rayon")]
fn set_values(set: &IndexSet<i32>) -> Vec<i32> {
    set.iter().copied().collect()
}

#[cfg(feature = "rayon")]
#[test]
fn parallel_set_relationships_and_combinations_workflow() {
    let left: IndexSet<i32> = [1, 2, 3, 5, 8, 13].into_iter().collect();
    let right: IndexSet<i32> = [3, 5, 8, 21, 34].into_iter().collect();
    let left_clone: IndexSet<i32> = [13, 8, 5, 3, 2, 1].into_iter().collect();
    let subset: IndexSet<i32> = [2, 5, 13].into_iter().collect();
    let disjoint: IndexSet<i32> = [55, 89].into_iter().collect();

    assert!(indexmap::rayon::set::par_eq(&left, &left_clone));
    assert!(!indexmap::rayon::set::par_eq(&left, &right));

    assert!(indexmap::rayon::set::par_is_subset(&subset, &left));
    assert!(indexmap::rayon::set::par_is_superset(&left, &subset));
    assert!(!indexmap::rayon::set::par_is_subset(&right, &left));

    assert!(indexmap::rayon::set::par_is_disjoint(&left, &disjoint));
    assert!(!indexmap::rayon::set::par_is_disjoint(&left, &right));

    let difference = sorted_vec(
        indexmap::rayon::set::par_difference(&left, &right)
            .copied()
            .collect::<Vec<_>>(),
    );
    assert_eq!(difference, vec![1, 2, 13]);

    let intersection = sorted_vec(
        indexmap::rayon::set::par_intersection(&left, &right)
            .copied()
            .collect::<Vec<_>>(),
    );
    assert_eq!(intersection, vec![3, 5, 8]);

    let symmetric_difference = sorted_vec(
        indexmap::rayon::set::par_symmetric_difference(&left, &right)
            .copied()
            .collect::<Vec<_>>(),
    );
    assert_eq!(symmetric_difference, vec![1, 2, 13, 21, 34]);

    let union = sorted_vec(
        indexmap::rayon::set::par_union(&left, &right)
            .copied()
            .collect::<Vec<_>>(),
    );
    assert_eq!(union, vec![1, 2, 3, 5, 8, 13, 21, 34]);

    let empty: IndexSet<i32> = IndexSet::new();
    assert!(indexmap::rayon::set::par_is_subset(&empty, &left));
    assert!(indexmap::rayon::set::par_is_disjoint(&empty, &left));
    assert_eq!(
        indexmap::rayon::set::par_union(&empty, &left)
            .copied()
            .collect::<Vec<_>>()
            .len(),
        left.len()
    );
}

#[cfg(feature = "rayon")]
#[test]
fn parallel_set_sorting_workflow() {
    let mut numbers: IndexSet<i32> = [40, -1, 7, 7, 3, 12, 0].into_iter().collect();
    assert_eq!(numbers.len(), 6);

    indexmap::rayon::set::par_sort(&mut numbers);
    assert_eq!(set_values(&numbers), vec![-1, 0, 3, 7, 12, 40]);
    assert_eq!(numbers.get_index_of(&7), Some(3));

    indexmap::rayon::set::par_sort_by(&mut numbers, |a, b| b.cmp(a));
    assert_eq!(set_values(&numbers), vec![40, 12, 7, 3, 0, -1]);

    let sorted_by_absolute_value = indexmap::rayon::set::par_sorted_by(numbers.clone(), |a, b| {
        a.abs().cmp(&b.abs()).then_with(|| a.cmp(b))
    })
    .collect::<Vec<_>>();
    assert_eq!(sorted_by_absolute_value, vec![0, -1, 3, 7, 12, 40]);

    indexmap::rayon::set::par_sort_unstable(&mut numbers);
    assert_eq!(set_values(&numbers), vec![-1, 0, 3, 7, 12, 40]);

    indexmap::rayon::set::par_sort_unstable_by(&mut numbers, |a, b| {
        let parity_order = (a % 2).abs().cmp(&(b % 2).abs());
        if parity_order == Ordering::Equal {
            a.cmp(b)
        } else {
            parity_order
        }
    });
    assert_eq!(set_values(&numbers), vec![0, 12, -1, 3, 7, 40]);

    let sorted_unstable_desc = indexmap::rayon::set::par_sorted_unstable_by(
        numbers.clone(),
        |a, b| b.cmp(a),
    )
    .collect::<Vec<_>>();
    assert_eq!(sorted_unstable_desc, vec![40, 12, 7, 3, 0, -1]);

    indexmap::rayon::set::par_sort_by_cached_key(&mut numbers, |value| {
        (value.abs() % 10, *value)
    });
    assert_eq!(set_values(&numbers), vec![0, -1, 12, 3, 7, 40]);

    assert!(numbers.contains(&12));
    assert!(!numbers.contains(&99));
}