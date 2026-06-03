
use indexmap::{IndexMap, indexmap};
use std::hash::{BuildHasher, Hash, Hasher};

#[test]
fn test_hasher_returns_valid_build_hasher() {
    let map: IndexMap<String, i32> = IndexMap::new();
    let hasher = map.hasher();

    let mut state = hasher.build_hasher();
    "test_key".hash(&mut state);
    let hash_value = state.finish();

    let mut state2 = hasher.build_hasher();
    "test_key".hash(&mut state2);
    let hash_value2 = state2.finish();
    assert_eq!(hash_value, hash_value2);


    let mut state3 = hasher.build_hasher();
    "different_key".hash(&mut state3);
    let hash_value3 = state3.finish();
    assert_ne!(hash_value, hash_value3);


    let map2 = indexmap! { "a" => 1, "b" => 2 };
    let hasher2 = map2.hasher();
    let mut s = hasher2.build_hasher();
    "a".hash(&mut s);
    let h = s.finish();
    assert_ne!(h, 0_u64.wrapping_add(1).wrapping_sub(1));

    assert_eq!(map2.len(), 2);
    assert_eq!(map2["a"], 1);
    assert_eq!(map2["b"], 2);
}

#[test]
fn test_clear_empties_map_preserves_capacity() {
    let mut map = indexmap! {
        "alpha" => 10,
        "beta" => 20,
        "gamma" => 30,
        "delta" => 40,
    };
    assert_eq!(map.len(), 4);
    assert!(map.contains_key("alpha"));
    assert!(map.contains_key("delta"));

    let cap_before = map.capacity();
    map.clear();

    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
    assert!(!map.contains_key("alpha"));
    assert!(!map.contains_key("beta"));
    assert!(map.capacity() >= cap_before);

    map.insert("new_key", 99);
    assert_eq!(map.len(), 1);
    assert_eq!(map["new_key"], 99);
}

#[test]
fn test_truncate_shortens_map() {
    let mut map = indexmap! {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        "e" => 5,
    };
    assert_eq!(map.len(), 5);

    map.truncate(3);
    assert_eq!(map.len(), 3);
    assert!(map.contains_key("a"));
    assert!(map.contains_key("b"));
    assert!(map.contains_key("c"));
    assert!(!map.contains_key("d"));
    assert!(!map.contains_key("e"));


    map.truncate(0);
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());


    let mut map2 = indexmap! { "x" => 100 };
    map2.truncate(50);
    assert_eq!(map2.len(), 1);
    assert_eq!(map2["x"], 100);
}

#[test]
fn test_split_off_divides_map() {
    let mut map = indexmap! {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        "e" => 5,
    };
    assert_eq!(map.len(), 5);

    let split = map.split_off(2);
    assert_eq!(map.len(), 2);
    assert_eq!(split.len(), 3);

    assert!(map.contains_key("a"));
    assert!(map.contains_key("b"));
    assert!(!map.contains_key("c"));

    assert!(split.contains_key("c"));
    assert!(split.contains_key("d"));
    assert!(split.contains_key("e"));
    assert_eq!(split["c"], 3);
    assert_eq!(split["d"], 4);
    assert_eq!(split["e"], 5);


    let mut map3 = indexmap! { "x" => 10, "y" => 20 };
    let all = map3.split_off(0);
    assert_eq!(map3.len(), 0);
    assert_eq!(all.len(), 2);
}

#[test]
fn test_reserve_exact_increases_capacity() {
    let mut map: IndexMap<i32, i32> = IndexMap::new();
    assert_eq!(map.len(), 0);

    map.reserve_exact(100);
    let cap = map.capacity();
    assert!(cap >= 100);


    for i in 0..100 {
        map.insert(i, i * 2);
    }
    assert_eq!(map.len(), 100);
    assert_eq!(map[&0], 0);
    assert_eq!(map[&50], 100);
    assert_eq!(map[&99], 198);


    map.reserve_exact(50);
    assert!(map.capacity() >= 150);
    assert_eq!(map.len(), 100);
}

#[test]
fn test_try_reserve_exact_success_and_error() {
    let mut map: IndexMap<u64, u64> = IndexMap::new();

    let result = map.try_reserve_exact(100);
    assert!(result.is_ok());
    assert!(map.capacity() >= 100);


    for i in 0..50 {
        map.insert(i, i + 1);
    }
    assert_eq!(map.len(), 50);


    let result2 = map.try_reserve_exact(200);
    assert!(result2.is_ok());
    assert!(map.capacity() >= 250);


    let result3 = map.try_reserve_exact(usize::MAX / 2);
    assert!(result3.is_err());


    let err = result3.unwrap_err();
    let err_clone = err.clone();

    assert_eq!(format!("{}", err), format!("{}", err_clone));

    assert_eq!(map.len(), 50);
    assert_eq!(map[&0], 1);
}

#[test]
fn test_splice_replaces_range() {
    let mut map = indexmap! {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        "e" => 5,
    };
    assert_eq!(map.len(), 5);

    let replacement = vec![("x", 10), ("y", 20), ("z", 30)];
    let removed: Vec<(_, _)> = map.splice(1..3, replacement).collect();

    assert_eq!(removed.len(), 2);
    assert_eq!(removed[0], ("b", 2));
    assert_eq!(removed[1], ("c", 3));


    assert_eq!(map.len(), 6);
    assert_eq!(map.get_index(0), Some((&"a", &1)));
    assert_eq!(map.get_index(1), Some((&"x", &10)));
    assert_eq!(map.get_index(2), Some((&"y", &20)));
    assert_eq!(map.get_index(3), Some((&"z", &30)));
    assert_eq!(map.get_index(4), Some((&"d", &4)));
    assert_eq!(map.get_index(5), Some((&"e", &5)));
}

#[test]
fn test_append_merges_maps() {
    let mut map1 = indexmap! {
        "a" => 1,
        "b" => 2,
    };
    let mut map2 = indexmap! {
        "c" => 3,
        "d" => 4,
        "b" => 99,
    };

    assert_eq!(map1.len(), 2);
    assert_eq!(map2.len(), 3);

    map1.append(&mut map2);


    assert_eq!(map2.len(), 0);
    assert!(map2.is_empty());


    assert!(map1.contains_key("a"));
    assert!(map1.contains_key("b"));
    assert!(map1.contains_key("c"));
    assert!(map1.contains_key("d"));
    assert_eq!(map1["a"], 1);
    assert_eq!(map1["b"], 99);
    assert_eq!(map1["c"], 3);
    assert_eq!(map1["d"], 4);
}

#[test]
fn test_get_full_mut_returns_index_key_value() {
    let mut map = indexmap! {
        "first" => 100,
        "second" => 200,
        "third" => 300,
    };

    let result = map.get_full_mut("second");
    assert!(result.is_some());
    let (idx, key, val) = result.unwrap();
    assert_eq!(idx, 1);
    assert_eq!(*key, "second");
    assert_eq!(*val, 200);


    *val = 999;
    assert_eq!(map["second"], 999);


    let missing = map.get_full_mut("nonexistent");
    assert!(missing.is_none());


    let (idx0, key0, val0) = map.get_full_mut("first").unwrap();
    assert_eq!(idx0, 0);
    assert_eq!(*key0, "first");
    assert_eq!(*val0, 100);

    let (idx2, key2, val2) = map.get_full_mut("third").unwrap();
    assert_eq!(idx2, 2);
    assert_eq!(*key2, "third");
    assert_eq!(*val2, 300);
}

#[test]
fn test_remove_entry_swap_removes() {
    let mut map = indexmap! {
        "a" => 10,
        "b" => 20,
        "c" => 30,
        "d" => 40,
    };
    assert_eq!(map.len(), 4);


    let removed = map.remove_entry("b");
    assert_eq!(removed, Some(("b", 20)));
    assert_eq!(map.len(), 3);
    assert!(!map.contains_key("b"));


    assert_eq!(map.get_index(0), Some((&"a", &10)));
    assert_eq!(map.get_index(1), Some((&"d", &40)));
    assert_eq!(map.get_index(2), Some((&"c", &30)));


    let missing = map.remove_entry("zzz");
    assert!(missing.is_none());
    assert_eq!(map.len(), 3);
}

#[test]
fn test_shift_remove_entry_preserves_order() {
    let mut map = indexmap! {
        "a" => 10,
        "b" => 20,
        "c" => 30,
        "d" => 40,
    };
    assert_eq!(map.len(), 4);

    let removed = map.shift_remove_entry("b");
    assert_eq!(removed, Some(("b", 20)));
    assert_eq!(map.len(), 3);


    assert_eq!(map.get_index(0), Some((&"a", &10)));
    assert_eq!(map.get_index(1), Some((&"c", &30)));
    assert_eq!(map.get_index(2), Some((&"d", &40)));


    let removed_first = map.shift_remove_entry("a");
    assert_eq!(removed_first, Some(("a", 10)));
    assert_eq!(map.get_index(0), Some((&"c", &30)));
    assert_eq!(map.get_index(1), Some((&"d", &40)));
    assert_eq!(map.len(), 2);


    let missing = map.shift_remove_entry("nonexistent");
    assert!(missing.is_none());
}

#[test]
fn test_retain_filters_entries() {
    let mut map = indexmap! {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
    };
    assert_eq!(map.len(), 6);


    map.retain(|k, _v| k % 2 == 0);

    assert_eq!(map.len(), 3);
    assert!(!map.contains_key(&1));
    assert!(map.contains_key(&2));
    assert!(!map.contains_key(&3));
    assert!(map.contains_key(&4));
    assert!(!map.contains_key(&5));
    assert!(map.contains_key(&6));


    let keys: Vec<_> = map.keys().copied().collect();
    assert_eq!(keys, vec![2, 4, 6]);


    map.retain(|_k, v| v.len() > 3);
    assert_eq!(map.len(), 1);
    assert!(map.contains_key(&4));
    assert_eq!(map[&4], "four");
}

#[test]
fn test_sort_keys_orders_by_key() {
    let mut map = indexmap! {
        5 => "five",
        1 => "one",
        3 => "three",
        2 => "two",
        4 => "four",
    };


    let keys_before: Vec<_> = map.keys().copied().collect();
    assert_eq!(keys_before, vec![5, 1, 3, 2, 4]);

    map.sort_keys();

    let keys_after: Vec<_> = map.keys().copied().collect();
    assert_eq!(keys_after, vec![1, 2, 3, 4, 5]);


    assert_eq!(map.get_index(0), Some((&1, &"one")));
    assert_eq!(map.get_index(1), Some((&2, &"two")));
    assert_eq!(map.get_index(2), Some((&3, &"three")));
    assert_eq!(map.get_index(3), Some((&4, &"four")));
    assert_eq!(map.get_index(4), Some((&5, &"five")));
}

#[test]
fn test_sort_by_custom_comparator() {
    let mut map = indexmap! {
        "apple" => 3,
        "banana" => 1,
        "cherry" => 4,
        "date" => 1,
        "elderberry" => 5,
    };


    map.sort_by(|k1, v1, k2, v2| v1.cmp(v2).then_with(|| k1.cmp(k2)));

    let entries: Vec<_> = map.iter().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(entries[0], ("banana", 1));
    assert_eq!(entries[1], ("date", 1));
    assert_eq!(entries[2], ("apple", 3));
    assert_eq!(entries[3], ("cherry", 4));
    assert_eq!(entries[4], ("elderberry", 5));
    assert_eq!(map.len(), 5);


    map.sort_by(|k1, _v1, k2, _v2| k2.len().cmp(&k1.len()));
    let keys: Vec<_> = map.keys().copied().collect();
    assert_eq!(keys[0], "elderberry");
    assert_eq!(keys[1], "banana");
    assert_eq!(keys[2], "cherry");
}

#[test]
fn test_sorted_by_consumes_and_returns_sorted_iter() {
    let map = indexmap! {
        "z" => 26,
        "a" => 1,
        "m" => 13,
        "f" => 6,
    };

    let sorted_iter = map.sorted_by(|k1, _v1, k2, _v2| k1.cmp(k2));


    let slice = sorted_iter.as_slice();
    assert_eq!(slice.len(), 4);

    let sorted_entries: Vec<_> = sorted_iter.collect();
    assert_eq!(sorted_entries.len(), 4);
    assert_eq!(sorted_entries[0], ("a", 1));
    assert_eq!(sorted_entries[1], ("f", 6));
    assert_eq!(sorted_entries[2], ("m", 13));
    assert_eq!(sorted_entries[3], ("z", 26));
}

#[test]
fn test_combined_workflow_clear_reserve_insert_truncate_split() {
    let mut map: IndexMap<i32, String> = IndexMap::new();


    map.reserve_exact(20);
    assert!(map.capacity() >= 20);


    for i in 0..10 {
        map.insert(i, format!("val_{}", i));
    }
    assert_eq!(map.len(), 10);


    map.truncate(7);
    assert_eq!(map.len(), 7);
    assert!(!map.contains_key(&7));
    assert!(map.contains_key(&6));


    let tail = map.split_off(3);
    assert_eq!(map.len(), 3);
    assert_eq!(tail.len(), 4);
    assert_eq!(map[&0], "val_0");
    assert_eq!(tail[&3], "val_3");


    map.clear();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);


    assert_eq!(tail.len(), 4);
    assert_eq!(tail[&6], "val_6");
}

#[test]
fn test_splice_empty_range_inserts() {
    let mut map = indexmap! {
        "a" => 1,
        "b" => 2,
        "c" => 3,
    };


    let replacement = vec![("x", 10), ("y", 20)];
    let removed: Vec<_> = map.splice(1..1, replacement).collect();
    assert_eq!(removed.len(), 0);
    assert_eq!(map.len(), 5);
    assert_eq!(map.get_index(0), Some((&"a", &1)));
    assert_eq!(map.get_index(1), Some((&"x", &10)));
    assert_eq!(map.get_index(2), Some((&"y", &20)));
    assert_eq!(map.get_index(3), Some((&"b", &2)));
    assert_eq!(map.get_index(4), Some((&"c", &3)));
}

#[test]
fn test_retain_mutates_values() {
    let mut map = indexmap! {
        1 => 10,
        2 => 20,
        3 => 30,
        4 => 40,
        5 => 50,
    };


    map.retain(|k, v| {
        *v += 1;
        *k <= 3
    });

    assert_eq!(map.len(), 3);

    assert_eq!(map[&1], 11);
    assert_eq!(map[&2], 21);
    assert_eq!(map[&3], 31);
    assert!(!map.contains_key(&4));
    assert!(!map.contains_key(&5));

    let keys: Vec<_> = map.keys().copied().collect();
    assert_eq!(keys, vec![1, 2, 3]);
}