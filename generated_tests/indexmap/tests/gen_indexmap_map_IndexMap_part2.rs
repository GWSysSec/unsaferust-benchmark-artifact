
use indexmap::{indexmap, IndexMap};

#[test]
fn test_sort_unstable_keys_basic() {
    let mut map: IndexMap<&str, i32> = IndexMap::new();
    map.insert("cherry", 3);
    map.insert("apple", 1);
    map.insert("banana", 2);
    map.insert("date", 4);
    map.insert("elderberry", 5);

    assert_eq!(map.get_index(0), Some((&"cherry", &3)));
    assert_eq!(map.get_index(1), Some((&"apple", &1)));
    assert_eq!(map.get_index(2), Some((&"banana", &2)));

    map.sort_unstable_keys();

    let keys: Vec<&&str> = map.keys().collect();
    assert_eq!(keys, vec![&"apple", &"banana", &"cherry", &"date", &"elderberry"]);
    assert_eq!(map.get_index(0), Some((&"apple", &1)));
    assert_eq!(map.get_index(1), Some((&"banana", &2)));
    assert_eq!(map.get_index(2), Some((&"cherry", &3)));
    assert_eq!(map.get_index(3), Some((&"date", &4)));
    assert_eq!(map.get_index(4), Some((&"elderberry", &5)));
}

#[test]
fn test_sort_unstable_by_reverse_order() {
    let mut map: IndexMap<i32, &str> = IndexMap::new();
    map.insert(3, "three");
    map.insert(1, "one");
    map.insert(4, "four");
    map.insert(1, "one_dup");
    map.insert(5, "five");
    map.insert(2, "two");

    assert_eq!(map.len(), 5);
    assert_eq!(map[&1], "one_dup");

    map.sort_unstable_by(|k1, _v1, k2, _v2| k2.cmp(k1));

    let keys: Vec<&i32> = map.keys().collect();
    assert_eq!(keys, vec![&5, &4, &3, &2, &1]);
    assert_eq!(map.get_index(0), Some((&5, &"five")));
    assert_eq!(map.get_index(1), Some((&4, &"four")));
    assert_eq!(map.get_index(4), Some((&1, &"one_dup")));
    assert_eq!(map.len(), 5);
}

#[test]
fn test_sorted_unstable_by_consumes_map() {
    let mut map: IndexMap<i32, i32> = IndexMap::new();
    for i in (0..10).rev() {
        map.insert(i, i * 10);
    }

    assert_eq!(map.len(), 10);
    assert_eq!(map.get_index(0), Some((&9, &90)));

    let into_iter = map.sorted_unstable_by(|k1, _v1, k2, _v2| k1.cmp(k2));
    let slice = into_iter.as_slice();

    assert_eq!(slice.len(), 10);
    assert_eq!(slice.get_index(0), Some((&0, &0)));
    assert_eq!(slice.get_index(5), Some((&5, &50)));
    assert_eq!(slice.get_index(9), Some((&9, &90)));

    let collected: Vec<(i32, i32)> = into_iter.collect();
    assert_eq!(collected.len(), 10);
    assert_eq!(collected[0], (0, 0));
    assert_eq!(collected[9], (9, 90));
}

#[test]
fn test_sort_by_cached_key_string_length() {
    let mut map: IndexMap<String, u32> = IndexMap::new();
    map.insert("elephant".to_string(), 8);
    map.insert("cat".to_string(), 3);
    map.insert("dog".to_string(), 3);
    map.insert("hippopotamus".to_string(), 12);
    map.insert("ant".to_string(), 3);

    assert_eq!(map.len(), 5);

    map.sort_by_cached_key(|k, _v| k.len());

    let keys: Vec<&String> = map.keys().collect();
    assert_eq!(keys[0].len(), 3);
    assert_eq!(keys[1].len(), 3);
    assert_eq!(keys[2].len(), 3);
    assert_eq!(keys[3], "elephant");
    assert_eq!(keys[4], "hippopotamus");
    assert_eq!(map.get("elephant"), Some(&8));
    assert_eq!(map.get("hippopotamus"), Some(&12));
    assert_eq!(map.len(), 5);
}

#[test]
fn test_binary_search_by_sorted_map() {
    let mut map: IndexMap<i32, &str> = IndexMap::new();
    map.insert(2, "two");
    map.insert(5, "five");
    map.insert(8, "eight");
    map.insert(12, "twelve");
    map.insert(20, "twenty");

    map.sort_unstable_keys();

    let result = map.binary_search_by(|k, _v| k.cmp(&8));
    assert_eq!(result, Ok(2));

    let result = map.binary_search_by(|k, _v| k.cmp(&5));
    assert_eq!(result, Ok(1));

    let result = map.binary_search_by(|k, _v| k.cmp(&20));
    assert_eq!(result, Ok(4));

    let result = map.binary_search_by(|k, _v| k.cmp(&6));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), 2);

    let result = map.binary_search_by(|k, _v| k.cmp(&1));
    assert_eq!(result.unwrap_err(), 0);

    let result = map.binary_search_by(|k, _v| k.cmp(&25));
    assert_eq!(result.unwrap_err(), 5);

    let result = map.binary_search_by(|k, _v| k.cmp(&2));
    assert_eq!(result, Ok(0));

    let result = map.binary_search_by(|k, _v| k.cmp(&12));
    assert_eq!(result, Ok(3));
}

#[test]
fn test_binary_search_by_key_with_values() {
    let mut map: IndexMap<&str, i32> = IndexMap::new();
    map.insert("alpha", 10);
    map.insert("beta", 20);
    map.insert("gamma", 30);
    map.insert("delta", 40);
    map.insert("epsilon", 50);

    map.sort_unstable_by(|_k1, v1, _k2, v2| v1.cmp(v2));

    let result = map.binary_search_by_key(&30, |_k, v| *v);
    assert_eq!(result, Ok(2));

    let result = map.binary_search_by_key(&10, |_k, v| *v);
    assert_eq!(result, Ok(0));

    let result = map.binary_search_by_key(&50, |_k, v| *v);
    assert_eq!(result, Ok(4));

    let result = map.binary_search_by_key(&15, |_k, v| *v);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), 1);

    let result = map.binary_search_by_key(&5, |_k, v| *v);
    assert_eq!(result.unwrap_err(), 0);

    let result = map.binary_search_by_key(&55, |_k, v| *v);
    assert_eq!(result.unwrap_err(), 5);

    let result = map.binary_search_by_key(&40, |_k, v| *v);
    assert_eq!(result, Ok(3));

    let result = map.binary_search_by_key(&20, |_k, v| *v);
    assert_eq!(result, Ok(1));
}

#[test]
fn test_partition_point_sorted_map() {
    let mut map: IndexMap<i32, i32> = IndexMap::new();
    for i in &[10, 20, 30, 40, 50, 60, 70, 80] {
        map.insert(*i, i * 2);
    }

    map.sort_unstable_keys();

    let pp = map.partition_point(|k, _v| *k < 45);
    assert_eq!(pp, 4);

    let pp = map.partition_point(|k, _v| *k < 10);
    assert_eq!(pp, 0);

    let pp = map.partition_point(|k, _v| *k < 100);
    assert_eq!(pp, 8);

    let pp = map.partition_point(|k, _v| *k <= 30);
    assert_eq!(pp, 3);

    let pp = map.partition_point(|_k, v| *v < 100);
    assert_eq!(pp, 4);

    let pp = map.partition_point(|k, _v| *k < 1);
    assert_eq!(pp, 0);

    let pp = map.partition_point(|k, _v| *k <= 80);
    assert_eq!(pp, 8);

    let pp = map.partition_point(|k, _v| *k < 55);
    assert_eq!(pp, 5);
}

#[test]
fn test_into_boxed_slice() {
    let map = indexmap! {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        "e" => 5,
    };

    assert_eq!(map.len(), 5);

    let boxed = map.into_boxed_slice();

    assert_eq!(boxed.len(), 5);
    assert_eq!(boxed.get_index(0), Some((&"a", &1)));
    assert_eq!(boxed.get_index(1), Some((&"b", &2)));
    assert_eq!(boxed.get_index(2), Some((&"c", &3)));
    assert_eq!(boxed.get_index(3), Some((&"d", &4)));
    assert_eq!(boxed.get_index(4), Some((&"e", &5)));

    let keys: Vec<&&str> = boxed.keys().collect();
    assert_eq!(keys.len(), 5);
    assert_eq!(keys[0], &"a");
}

#[test]
fn test_get_range_and_get_range_mut() {
    let mut map: IndexMap<i32, i32> = IndexMap::new();
    for i in 0..10 {
        map.insert(i, i * 100);
    }

    let range = map.get_range(2..5);
    assert!(range.is_some());
    let slice = range.unwrap();
    assert_eq!(slice.len(), 3);
    assert_eq!(slice.get_index(0), Some((&2, &200)));
    assert_eq!(slice.get_index(1), Some((&3, &300)));
    assert_eq!(slice.get_index(2), Some((&4, &400)));

    let range_full = map.get_range(..);
    assert!(range_full.is_some());
    assert_eq!(range_full.unwrap().len(), 10);

    let range_empty = map.get_range(5..5);
    assert!(range_empty.is_some());
    assert_eq!(range_empty.unwrap().len(), 0);

    let range_oob = map.get_range(8..12);
    assert!(range_oob.is_none());

    let range_mut = map.get_range_mut(0..3);
    assert!(range_mut.is_some());
    let slice_mut = range_mut.unwrap();
    assert_eq!(slice_mut.len(), 3);

    let range_mut_oob = map.get_range_mut(9..11);
    assert!(range_mut_oob.is_none());
}

#[test]
fn test_first_mut_and_last_mut() {
    let mut map: IndexMap<&str, i32> = IndexMap::new();

    assert!(map.first_mut().is_none());
    assert!(map.last_mut().is_none());

    map.insert("first", 1);
    map.insert("second", 2);
    map.insert("third", 3);

    let first = map.first_mut().unwrap();
    assert_eq!(*first.0, "first");
    assert_eq!(*first.1, 1);
    *first.1 = 100;

    let last = map.last_mut().unwrap();
    assert_eq!(*last.0, "third");
    assert_eq!(*last.1, 3);
    *last.1 = 300;

    assert_eq!(map[&"first"], 100);
    assert_eq!(map[&"third"], 300);
    assert_eq!(map[&"second"], 2);
    assert_eq!(map.len(), 3);
}

#[test]
fn test_shift_remove_index() {
    let mut map: IndexMap<&str, i32> = IndexMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);
    map.insert("d", 4);
    map.insert("e", 5);

    assert_eq!(map.len(), 5);

    let removed = map.shift_remove_index(2);
    assert_eq!(removed, Some(("c", 3)));
    assert_eq!(map.len(), 4);

    assert_eq!(map.get_index(0), Some((&"a", &1)));
    assert_eq!(map.get_index(1), Some((&"b", &2)));
    assert_eq!(map.get_index(2), Some((&"d", &4)));
    assert_eq!(map.get_index(3), Some((&"e", &5)));

    let removed_first = map.shift_remove_index(0);
    assert_eq!(removed_first, Some(("a", 1)));
    assert_eq!(map.len(), 3);
    assert_eq!(map.get_index(0), Some((&"b", &2)));

    let removed_last = map.shift_remove_index(2);
    assert_eq!(removed_last, Some(("e", 5)));
    assert_eq!(map.len(), 2);

    let removed_oob = map.shift_remove_index(10);
    assert_eq!(removed_oob, None);
    assert_eq!(map.len(), 2);
}

#[test]
fn test_sort_unstable_keys_with_integers() {
    let mut map: IndexMap<i32, String> = IndexMap::new();
    let values = [50, 30, 80, 10, 60, 20, 70, 40, 90, 100];
    for v in &values {
        map.insert(*v, format!("val_{}", v));
    }

    assert_eq!(map.get_index(0), Some((&50, &"val_50".to_string())));
    assert_eq!(map.len(), 10);

    map.sort_unstable_keys();

    assert_eq!(map.get_index(0), Some((&10, &"val_10".to_string())));
    assert_eq!(map.get_index(1), Some((&20, &"val_20".to_string())));
    assert_eq!(map.get_index(9), Some((&100, &"val_100".to_string())));

    for i in 0..9 {
        let (k1, _) = map.get_index(i).unwrap();
        let (k2, _) = map.get_index(i + 1).unwrap();
        assert!(k1 < k2);
    }

    assert_eq!(map.get(&50), Some(&"val_50".to_string()));
    assert_eq!(map.get(&10), Some(&"val_10".to_string()));
    assert_eq!(map.get(&100), Some(&"val_100".to_string()));
}

#[test]
fn test_sort_unstable_by_value_then_key() {
    let mut map: IndexMap<&str, i32> = IndexMap::new();
    map.insert("banana", 2);
    map.insert("apple", 2);
    map.insert("cherry", 1);
    map.insert("date", 3);
    map.insert("elderberry", 1);

    map.sort_unstable_by(|k1, v1, k2, v2| {
        v1.cmp(v2).then_with(|| k1.cmp(k2))
    });

    assert_eq!(map.get_index(0).unwrap().0, &"cherry");
    assert_eq!(map.get_index(1).unwrap().0, &"elderberry");
    assert_eq!(map.get_index(2).unwrap().0, &"apple");
    assert_eq!(map.get_index(3).unwrap().0, &"banana");
    assert_eq!(map.get_index(4).unwrap().0, &"date");
    assert_eq!(*map.get_index(0).unwrap().1, 1);
    assert_eq!(*map.get_index(2).unwrap().1, 2);
    assert_eq!(*map.get_index(4).unwrap().1, 3);
}

#[test]
fn test_combined_sort_search_remove_workflow() {
    let mut map: IndexMap<u32, u32> = IndexMap::new();
    for i in (1..=20).rev() {
        map.insert(i, i * i);
    }

    assert_eq!(map.len(), 20);
    assert_eq!(map.get_index(0), Some((&20, &400)));

    map.sort_unstable_keys();

    assert_eq!(map.get_index(0), Some((&1, &1)));
    assert_eq!(map.get_index(19), Some((&20, &400)));

    let found = map.binary_search_by_key(&10, |k, _v| *k);
    assert_eq!(found, Ok(9));

    let pp = map.partition_point(|k, _v| *k <= 15);
    assert_eq!(pp, 15);

    let removed = map.shift_remove_index(9);
    assert_eq!(removed, Some((10, 100)));
    assert_eq!(map.len(), 19);

    assert_eq!(map.get_index(9), Some((&11, &121)));

    let first = map.first_mut().unwrap();
    assert_eq!(*first.0, 1);
    *first.1 = 999;
    assert_eq!(map[&1], 999);

    let last = map.last_mut().unwrap();
    assert_eq!(*last.0, 20);
    *last.1 = 888;
    assert_eq!(map[&20], 888);
}

#[test]
fn test_into_boxed_slice_empty_map() {
    let map: IndexMap<String, String> = IndexMap::new();
    assert_eq!(map.len(), 0);

    let boxed = map.into_boxed_slice();
    assert_eq!(boxed.len(), 0);
    assert!(boxed.first().is_none());
    assert!(boxed.last().is_none());
    assert_eq!(boxed.keys().count(), 0);
    assert_eq!(boxed.values().count(), 0);
    assert!(boxed.get_index(0).is_none());
    assert!(boxed.is_empty());

    let map2: IndexMap<i32, i32> = indexmap! { 42 => 84 };
    let boxed2 = map2.into_boxed_slice();
    assert_eq!(boxed2.len(), 1);
    assert_eq!(boxed2.get_index(0), Some((&42, &84)));
}

#[test]
fn test_sort_by_cached_key_expensive_computation() {
    let mut map: IndexMap<i32, Vec<i32>> = IndexMap::new();
    map.insert(5, vec![1, 2, 3, 4, 5]);
    map.insert(3, vec![1, 2, 3]);
    map.insert(1, vec![1]);
    map.insert(4, vec![1, 2, 3, 4]);
    map.insert(2, vec![1, 2]);

    map.sort_by_cached_key(|_k, v| v.iter().sum::<i32>());

    assert_eq!(*map.get_index(0).unwrap().0, 1);
    assert_eq!(*map.get_index(1).unwrap().0, 2);
    assert_eq!(*map.get_index(2).unwrap().0, 3);
    assert_eq!(*map.get_index(3).unwrap().0, 4);
    assert_eq!(*map.get_index(4).unwrap().0, 5);

    assert_eq!(map.get_index(0).unwrap().1.len(), 1);
    assert_eq!(map.get_index(4).unwrap().1.len(), 5);
    assert_eq!(map.len(), 5);
    assert_eq!(map.get(&3).unwrap(), &vec![1, 2, 3]);
}