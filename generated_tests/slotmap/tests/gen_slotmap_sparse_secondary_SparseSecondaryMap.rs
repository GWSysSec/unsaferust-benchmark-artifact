use slotmap::{SlotMap, SparseSecondaryMap, DefaultKey, Key};

#[test]
fn test_sparse_secondary_map_basic_operations() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("first");
    let k2 = primary.insert("second");
    let k3 = primary.insert("third");

    secondary.insert(k1, 10);
    secondary.insert(k2, 20);
    secondary.insert(k3, 30);

    assert_eq!(secondary.len(), 3);
    assert_eq!(secondary.get(k1), Some(&10));
    assert_eq!(secondary.get(k2), Some(&20));
    assert_eq!(secondary.get(k3), Some(&30));


    secondary.insert(k2, 200);
    assert_eq!(secondary.get(k2), Some(&200));


    assert_eq!(secondary.remove(k1), Some(10));
    assert_eq!(secondary.len(), 2);
    assert_eq!(secondary.get(k1), None);
}

#[test]
fn test_sparse_secondary_map_contains_key() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("a");
    let k2 = primary.insert("b");

    secondary.insert(k1, 100);

    assert!(secondary.contains_key(k1));
    assert!(!secondary.contains_key(k2));
}

#[test]
fn test_sparse_secondary_map_iteration() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("a");
    let k2 = primary.insert("b");
    let k3 = primary.insert("c");

    secondary.insert(k1, 1);
    secondary.insert(k2, 2);
    secondary.insert(k3, 3);

    let mut keys: Vec<DefaultKey> = secondary.keys().collect();
    keys.sort_by_key(|k| k.data().as_ffi());
    assert_eq!(keys.len(), 3);

    let mut values: Vec<&i32> = secondary.values().collect();
    values.sort();
    assert_eq!(values, vec![&1, &2, &3]);


    let items: Vec<(DefaultKey, &i32)> = secondary.iter().collect();
    assert_eq!(items.len(), 3);
}

#[test]
fn test_sparse_secondary_map_iter_mut() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("a");
    let k2 = primary.insert("b");

    secondary.insert(k1, 10);
    secondary.insert(k2, 20);

    for (_key, value) in secondary.iter_mut() {
        *value *= 2;
    }

    assert_eq!(secondary.get(k1), Some(&20));
    assert_eq!(secondary.get(k2), Some(&40));
}

#[test]
fn test_sparse_secondary_map_drain() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("a");
    let k2 = primary.insert("b");
    let k3 = primary.insert("c");

    secondary.insert(k1, 1);
    secondary.insert(k2, 2);
    secondary.insert(k3, 3);

    let drained: Vec<(DefaultKey, i32)> = secondary.drain().collect();
    assert_eq!(drained.len(), 3);
    assert!(secondary.is_empty());
}

#[test]
fn test_sparse_secondary_map_entry_api() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("a");


    secondary.entry(k1).unwrap().or_insert(42);
    assert_eq!(secondary.get(k1), Some(&42));


    secondary.entry(k1).unwrap().or_insert(99);
    assert_eq!(secondary.get(k1), Some(&42));
}

#[test]
fn test_sparse_secondary_map_get_mut() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("hello");
    secondary.insert(k1, 5);

    if let Some(val) = secondary.get_mut(k1) {
        *val = 50;
    }
    assert_eq!(secondary.get(k1), Some(&50));
}

#[test]
fn test_sparse_secondary_map_large_scale() {
    let mut primary: SlotMap<DefaultKey, usize> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, usize> = SparseSecondaryMap::new();

    let mut keys = Vec::new();
    for i in 0..10000 {
        let k = primary.insert(i);
        secondary.insert(k, i * 2);
        keys.push(k);
    }

    assert_eq!(secondary.len(), 10000);

    for (i, k) in keys.iter().enumerate() {
        assert_eq!(secondary.get(*k), Some(&(i * 2)));
    }


    for k in keys.iter().step_by(2) {
        secondary.remove(*k);
    }

    assert_eq!(secondary.len(), 5000);
}

#[test]
fn test_sparse_secondary_map_into_iter() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("x");
    let k2 = primary.insert("y");

    secondary.insert(k1, 100);
    secondary.insert(k2, 200);

    let items: Vec<(DefaultKey, i32)> = secondary.into_iter().collect();
    assert_eq!(items.len(), 2);
}

#[test]
fn test_sparse_secondary_map_values_mut() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("a");
    let k2 = primary.insert("b");
    let k3 = primary.insert("c");

    secondary.insert(k1, 1);
    secondary.insert(k2, 2);
    secondary.insert(k3, 3);

    for v in secondary.values_mut() {
        *v += 10;
    }

    let mut values: Vec<&i32> = secondary.values().collect();
    values.sort();
    assert_eq!(values, vec![&11, &12, &13]);
}

#[test]
fn test_sparse_secondary_map_removed_primary_key() {
    let mut primary: SlotMap<DefaultKey, &str> = SlotMap::new();
    let mut secondary: SparseSecondaryMap<DefaultKey, i32> = SparseSecondaryMap::new();

    let k1 = primary.insert("temp");
    secondary.insert(k1, 999);


    primary.remove(k1);




    assert_eq!(secondary.len(), 1);
}