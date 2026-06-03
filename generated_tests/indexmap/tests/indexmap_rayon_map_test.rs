#[cfg(feature = "rayon")]
use indexmap::rayon::prelude::*;
#[cfg(feature = "rayon")]
use indexmap::IndexMap;
#[cfg(feature = "rayon")]
use std::cmp::Ordering;

#[cfg(feature = "rayon")]
fn ordered_pairs(map: &IndexMap<String, i32>) -> Vec<(String, i32)> {
    map.iter().map(|(k, v)| (k.clone(), *v)).collect()
}

#[cfg(feature = "rayon")]
#[test]
fn parallel_key_value_iteration_and_mutation_workflow() {
    let mut scores: IndexMap<String, i32> = IndexMap::new();
    scores.insert("delta".to_string(), 4);
    scores.insert("alpha".to_string(), 1);
    scores.insert("charlie".to_string(), 3);
    scores.insert("bravo".to_string(), 2);

    let mut keys: Vec<String> = scores.par_keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, vec!["alpha", "bravo", "charlie", "delta"]);

    let mut values: Vec<i32> = scores.par_values().copied().collect();
    values.sort();
    assert_eq!(values, vec![1, 2, 3, 4]);

    scores.par_values_mut().for_each(|value| *value *= 10);

    assert_eq!(scores.get("alpha"), Some(&10));
    assert_eq!(scores.get("bravo"), Some(&20));
    assert_eq!(scores.get("charlie"), Some(&30));
    assert_eq!(scores.get("delta"), Some(&40));

    let mut equivalent: IndexMap<String, i32> = IndexMap::new();
    equivalent.insert("delta".to_string(), 40);
    equivalent.insert("alpha".to_string(), 10);
    equivalent.insert("charlie".to_string(), 30);
    equivalent.insert("bravo".to_string(), 20);

    assert!(scores.par_eq(&equivalent));

    equivalent.insert("echo".to_string(), 50);
    assert!(!scores.par_eq(&equivalent));
}

#[cfg(feature = "rayon")]
#[test]
fn parallel_stable_sorting_variants_preserve_expected_semantics() {
    let mut by_key: IndexMap<String, i32> = IndexMap::new();
    by_key.insert("delta".to_string(), 4);
    by_key.insert("alpha".to_string(), 1);
    by_key.insert("charlie".to_string(), 3);
    by_key.insert("bravo".to_string(), 2);

    by_key.par_sort_keys();
    assert_eq!(
        ordered_pairs(&by_key),
        vec![
            ("alpha".to_string(), 1),
            ("bravo".to_string(), 2),
            ("charlie".to_string(), 3),
            ("delta".to_string(), 4),
        ]
    );

    by_key.par_sort_by(|left_key, left_value, right_key, right_value| {
        right_value
            .cmp(left_value)
            .then_with(|| left_key.cmp(right_key))
    });
    assert_eq!(
        ordered_pairs(&by_key),
        vec![
            ("delta".to_string(), 4),
            ("charlie".to_string(), 3),
            ("bravo".to_string(), 2),
            ("alpha".to_string(), 1),
        ]
    );

    by_key.par_sort_by_cached_key(|key, value| (value % 2, key.clone()));
    assert_eq!(
        ordered_pairs(&by_key),
        vec![
            ("bravo".to_string(), 2),
            ("delta".to_string(), 4),
            ("alpha".to_string(), 1),
            ("charlie".to_string(), 3),
        ]
    );

    let sorted_by_value_then_key: Vec<(String, i32)> = by_key
        .clone()
        .par_sorted_by(|left_key, left_value, right_key, right_value| {
            left_value
                .cmp(right_value)
                .then_with(|| left_key.cmp(right_key))
        })
        .collect();

    assert_eq!(
        sorted_by_value_then_key,
        vec![
            ("alpha".to_string(), 1),
            ("bravo".to_string(), 2),
            ("charlie".to_string(), 3),
            ("delta".to_string(), 4),
        ]
    );
}

#[cfg(feature = "rayon")]
#[test]
fn parallel_unstable_sorting_variants_produce_correct_ordering() {
    let mut map: IndexMap<String, i32> = IndexMap::new();
    map.insert("pear".to_string(), 4);
    map.insert("fig".to_string(), 2);
    map.insert("banana".to_string(), 6);
    map.insert("kiwi".to_string(), 4);
    map.insert("apple".to_string(), 5);

    map.par_sort_unstable_keys();
    assert_eq!(
        map.par_keys().cloned().collect::<Vec<String>>(),
        vec![
            "apple".to_string(),
            "banana".to_string(),
            "fig".to_string(),
            "kiwi".to_string(),
            "pear".to_string(),
        ]
    );

    map.par_sort_unstable_by(|left_key, left_value, right_key, right_value| {
        left_value
            .cmp(right_value)
            .then_with(|| left_key.cmp(right_key))
    });

    assert_eq!(
        ordered_pairs(&map),
        vec![
            ("fig".to_string(), 2),
            ("kiwi".to_string(), 4),
            ("pear".to_string(), 4),
            ("apple".to_string(), 5),
            ("banana".to_string(), 6),
        ]
    );

    let descending_by_key_length: Vec<(String, i32)> = map
        .clone()
        .par_sorted_unstable_by(|left_key, left_value, right_key, right_value| {
            right_key
                .len()
                .cmp(&left_key.len())
                .then_with(|| {
                    let key_order = left_key.cmp(right_key);
                    if key_order == Ordering::Equal {
                        left_value.cmp(right_value)
                    } else {
                        key_order
                    }
                })
        })
        .collect();

    assert_eq!(
        descending_by_key_length,
        vec![
            ("banana".to_string(), 6),
            ("apple".to_string(), 5),
            ("kiwi".to_string(), 4),
            ("pear".to_string(), 4),
            ("fig".to_string(), 2),
        ]
    );

    assert_eq!(map.len(), 5);
    assert_eq!(map.get("banana"), Some(&6));
}