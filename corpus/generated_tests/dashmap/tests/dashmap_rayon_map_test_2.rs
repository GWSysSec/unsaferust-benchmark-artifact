use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn par_iter_mut_updates_all_entries_in_parallel_workflow() {
    let map: DashMap<usize, usize> = DashMap::with_capacity(128);

    for key in 0..128 {
        assert_eq!(map.insert(key, key + 1), None);
    }

    assert_eq!(map.len(), 128);
    assert!(!map.is_empty());
    assert_eq!(map.get(&0).map(|value| *value), Some(1));
    assert_eq!(map.get(&127).map(|value| *value), Some(128));

    for mut entry in map.iter_mut() {
        let key = *entry.key();
        *entry.value_mut() = key * key;
    }

    assert_eq!(map.len(), 128);
    assert_eq!(map.get(&0).map(|value| *value), Some(0));
    assert_eq!(map.get(&1).map(|value| *value), Some(1));
    assert_eq!(map.get(&12).map(|value| *value), Some(144));
    assert_eq!(map.get(&127).map(|value| *value), Some(16_129));

    for key in 0..128 {
        assert_eq!(map.get(&key).map(|value| *value), Some(key * key));
    }

    map.retain(|key, _value| key % 2 == 0);

    assert_eq!(map.len(), 64);
    assert!(map.contains_key(&0));
    assert!(!map.contains_key(&1));
    assert!(map.contains_key(&126));
    assert!(!map.contains_key(&127));

    for mut entry in map.iter_mut() {
        *entry.value_mut() += 10;
    }

    assert_eq!(map.get(&0).map(|value| *value), Some(10));
    assert_eq!(map.get(&2).map(|value| *value), Some(14));
    assert_eq!(map.get(&10).map(|value| *value), Some(110));
    assert_eq!(map.get(&126).map(|value| *value), Some(15_886));

    for key in (0..128).step_by(2) {
        assert_eq!(map.get(&key).map(|value| *value), Some(key * key + 10));
    }
}

#[test]
fn par_iter_mut_handles_empty_and_single_entry_maps() {
    let empty: DashMap<&'static str, usize> = DashMap::new();
    let visited_empty = AtomicUsize::new(0);

    for _entry in empty.iter_mut() {
        visited_empty.fetch_add(1, Ordering::SeqCst);
    }

    assert_eq!(visited_empty.load(Ordering::SeqCst), 0);
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);

    let single: DashMap<&'static str, usize> = DashMap::new();
    assert_eq!(single.insert("count", 41), None);

    let visited_single = AtomicUsize::new(0);

    for mut entry in single.iter_mut() {
        assert_eq!(*entry.key(), "count");
        *entry.value_mut() += 1;
        visited_single.fetch_add(1, Ordering::SeqCst);
    }

    assert_eq!(visited_single.load(Ordering::SeqCst), 1);
    assert_eq!(single.len(), 1);
    assert_eq!(single.get("count").map(|value| *value), Some(42));

    assert_eq!(single.remove("count"), Some(("count", 42)));
    assert!(single.is_empty());
}