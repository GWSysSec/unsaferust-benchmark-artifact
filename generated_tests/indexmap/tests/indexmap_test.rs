use indexmap::{IndexMap, IndexSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::BuildHasherDefault;

type DefaultIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<DefaultHasher>>;
type DefaultIndexSet<T> = IndexSet<T, BuildHasherDefault<DefaultHasher>>;

#[test]
fn indexmap_with_default_builds_ordered_map_and_supports_updates() {
    let mut map: DefaultIndexMap<&str, i32> = indexmap::indexmap_with_default!(DefaultHasher;
        "apples" => 3,
        "bananas" => 5,
        "cherries" => 8
    );

    assert_eq!(map.len(), 3);
    assert!(!map.is_empty());
    assert_eq!(map.get("bananas"), Some(&5));
    assert_eq!(map.get_index_of("apples"), Some(0));
    assert_eq!(map.get_index(2), Some((&"cherries", &8)));

    let replaced = map.insert("bananas", 13);
    assert_eq!(replaced, Some(5));
    assert_eq!(map.get("bananas"), Some(&13));
    assert_eq!(map.get_index_of("bananas"), Some(1));

    let inserted = map.insert("dates", 21);
    assert_eq!(inserted, None);
    assert_eq!(map.len(), 4);
    assert_eq!(map.last(), Some((&"dates", &21)));

    let removed = map.shift_remove("apples");
    assert_eq!(removed, Some(3));
    assert_eq!(map.get_index(0), Some((&"bananas", &13)));
    assert!(!map.contains_key("apples"));
}

#[test]
fn indexmap_with_default_handles_empty_and_duplicate_keys() {
    let empty: DefaultIndexMap<String, usize> = indexmap::indexmap_with_default!(DefaultHasher;);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    assert_eq!(empty.first(), None);

    let map: DefaultIndexMap<&str, usize> = indexmap::indexmap_with_default!(DefaultHasher;
        "same" => 1,
        "other" => 2,
        "same" => 3
    );

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("same"), Some(&3));
    assert_eq!(map.get_index_of("same"), Some(0));
    assert_eq!(map.get_index(1), Some((&"other", &2)));
}

#[test]
fn indexset_with_default_builds_ordered_set_and_ignores_duplicates() {
    let mut set: DefaultIndexSet<&str> = indexmap::indexset_with_default!(DefaultHasher;
        "red",
        "green",
        "blue",
        "green"
    );

    assert_eq!(set.len(), 3);
    assert!(!set.is_empty());
    assert!(set.contains("red"));
    assert!(set.contains("green"));
    assert!(set.contains("blue"));
    assert_eq!(set.get_index(0), Some(&"red"));
    assert_eq!(set.get_index(1), Some(&"green"));
    assert_eq!(set.get_index(2), Some(&"blue"));

    let inserted_new = set.insert("yellow");
    assert!(inserted_new);
    assert_eq!(set.len(), 4);
    assert_eq!(set.get_index_of("yellow"), Some(3));

    let inserted_existing = set.insert("red");
    assert!(!inserted_existing);
    assert_eq!(set.len(), 4);

    let removed = set.shift_remove("green");
    assert!(removed);
    assert!(!set.contains("green"));
    assert_eq!(set.get_index(0), Some(&"red"));
    assert_eq!(set.get_index(1), Some(&"blue"));
}

#[test]
fn indexset_with_default_handles_empty_set_and_index_operations() {
    let mut empty: DefaultIndexSet<i32> = indexmap::indexset_with_default!(DefaultHasher;);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    assert_eq!(empty.get_index(0), None);

    assert!(empty.insert(10));
    assert!(empty.insert(20));
    assert!(empty.insert(30));
    assert_eq!(empty.len(), 3);

    empty.swap_indices(0, 2);
    assert_eq!(empty.get_index(0), Some(&30));
    assert_eq!(empty.get_index(2), Some(&10));

    let removed = empty.swap_remove_index(1);
    assert_eq!(removed, Some(20));
    assert_eq!(empty.len(), 2);
    assert!(empty.contains(&10));
    assert!(empty.contains(&30));
}