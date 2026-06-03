use slotmap::{SlotMap, DefaultKey};

#[test]
fn test_with_key_is_empty_capacity_reserve() {
    let mut sm: SlotMap<DefaultKey, String> = SlotMap::with_key();
    assert_eq!(sm.is_empty(), true);
    assert_eq!(sm.len(), 0);
    let cap_before = sm.capacity();

    sm.reserve(100);
    let cap_after = sm.capacity();
    assert!(cap_after >= 100, "capacity {} should be >= 100 after reserve", cap_after);
    assert!(cap_after >= cap_before);

    let mut keys = Vec::new();
    for i in 0..50 {
        keys.push(sm.insert(format!("item-{}", i)));
    }
    assert_eq!(sm.is_empty(), false);
    assert_eq!(sm.len(), 50);
    assert!(sm.capacity() >= 50);


    let cap_mid = sm.capacity();
    sm.reserve(10);
    assert!(sm.capacity() >= cap_mid);


    for (i, k) in keys.iter().enumerate() {
        assert_eq!(&sm[*k], &format!("item-{}", i));
    }

    sm.clear();
    assert!(sm.is_empty());
    assert_eq!(sm.len(), 0);

    assert_eq!(sm.contains_key(keys[0]), false);
    assert_eq!(sm.contains_key(keys[49]), false);

    assert!(sm.capacity() >= 50);
}

#[test]
fn test_insert_with_key_self_reference() {
    let mut sm: SlotMap<DefaultKey, (DefaultKey, i32)> = SlotMap::new();
    assert!(sm.is_empty());

    let k1 = sm.insert_with_key(|k| (k, 42));
    assert_eq!(sm[k1].0, k1);
    assert_eq!(sm[k1].1, 42);
    assert_eq!(sm.len(), 1);

    let k2 = sm.insert_with_key(|k| (k, 100));
    assert_eq!(sm[k2].0, k2);
    assert_eq!(sm[k2].1, 100);
    assert_ne!(k1, k2);
    assert_eq!(sm.len(), 2);

    let k3 = sm.insert_with_key(|k| (k, -7));
    assert_eq!(sm[k3].0, k3);
    assert_eq!(sm[k3].1, -7);
    assert_ne!(k3, k1);
    assert_ne!(k3, k2);


    assert_eq!(sm[k1].0, k1);
    assert_eq!(sm[k2].0, k2);
    assert_eq!(sm[k3].0, k3);
    assert_eq!(sm.len(), 3);
}

#[test]
fn test_try_insert_with_key_ok_and_err() {
    let mut sm: SlotMap<DefaultKey, i32> = SlotMap::new();
    assert_eq!(sm.is_empty(), true);

    let r1: Result<DefaultKey, &'static str> = sm.try_insert_with_key(|_k| Ok(7));
    assert!(r1.is_ok());
    let k1 = r1.unwrap();
    assert_eq!(sm[k1], 7);
    assert_eq!(sm.len(), 1);

    let r_err: Result<DefaultKey, &'static str> = sm.try_insert_with_key(|_k| Err("nope"));
    assert!(r_err.is_err());
    assert_eq!(r_err.unwrap_err(), "nope");

    assert_eq!(sm.len(), 1);
    assert_eq!(sm[k1], 7);


    let r2: Result<DefaultKey, &'static str> = sm.try_insert_with_key(|k| {

        let _ = k;
        Ok(99)
    });
    assert!(r2.is_ok());
    let k2 = r2.unwrap();
    assert_eq!(sm[k2], 99);
    assert_ne!(k1, k2);
    assert_eq!(sm.len(), 2);


    let r_err2: Result<DefaultKey, i32> = sm.try_insert_with_key(|_k| Err(-1));
    assert_eq!(r_err2.is_err(), true);
    assert_eq!(sm.len(), 2);
    assert_eq!(sm[k1], 7);
    assert_eq!(sm[k2], 99);
}

#[test]
fn test_retain_even_values_then_clear() {
    let mut sm: SlotMap<DefaultKey, i32> = SlotMap::new();
    let mut keys = Vec::new();
    for i in 0..20 {
        keys.push(sm.insert(i));
    }
    assert_eq!(sm.len(), 20);
    assert_eq!(sm.is_empty(), false);


    sm.retain(|_k, v| *v % 2 == 0);
    assert_eq!(sm.len(), 10);
    assert_eq!(sm.is_empty(), false);


    for (i, k) in keys.iter().enumerate() {
        if i % 2 == 0 {
            assert_eq!(sm.contains_key(*k), true);
            assert_eq!(sm[*k], i as i32);
        } else {
            assert_eq!(sm.contains_key(*k), false);
        }
    }


    sm.retain(|_k, _v| true);
    assert_eq!(sm.len(), 10);


    sm.retain(|_k, _v| false);
    assert_eq!(sm.len(), 0);
    assert!(sm.is_empty());


    for i in 0..5 {
        sm.insert(i * 10);
    }
    assert_eq!(sm.len(), 5);
    sm.clear();
    assert_eq!(sm.len(), 0);
    assert!(sm.is_empty());


    let knew = sm.insert(999);
    assert_eq!(sm[knew], 999);
    assert_eq!(sm.len(), 1);
}

#[test]
fn test_get_unchecked_mut_valid_keys() {
    let mut sm: SlotMap<DefaultKey, i64> = SlotMap::new();
    let k1 = sm.insert(100);
    let k2 = sm.insert(200);
    let k3 = sm.insert(300);

    assert_eq!(sm[k1], 100);
    assert_eq!(sm[k2], 200);
    assert_eq!(sm[k3], 300);
    assert_eq!(sm.len(), 3);

    unsafe {
        let v = sm.get_unchecked_mut(k2);
        assert_eq!(*v, 200);
        *v = 2000;
    }
    assert_eq!(sm[k2], 2000);
    assert_eq!(sm[k1], 100);
    assert_eq!(sm[k3], 300);

    unsafe {
        let v = sm.get_unchecked_mut(k1);
        *v += 1;
        let v3 = sm.get_unchecked_mut(k3);
        *v3 -= 50;
    }
    assert_eq!(sm[k1], 101);
    assert_eq!(sm[k3], 250);
    assert_eq!(sm.len(), 3);
}

#[test]
fn test_get_disjoint_mut_ok_and_overlap_and_invalid() {
    let mut sm: SlotMap<DefaultKey, i32> = SlotMap::new();
    let k1 = sm.insert(1);
    let k2 = sm.insert(2);
    let k3 = sm.insert(3);
    let k4 = sm.insert(4);
    assert_eq!(sm.len(), 4);


    let disj = sm.get_disjoint_mut([k1, k2, k3]);
    assert!(disj.is_some());
    let arr = disj.unwrap();
    assert_eq!(*arr[0], 1);
    assert_eq!(*arr[1], 2);
    assert_eq!(*arr[2], 3);
    *arr[0] = 10;
    *arr[1] = 20;
    *arr[2] = 30;

    assert_eq!(sm[k1], 10);
    assert_eq!(sm[k2], 20);
    assert_eq!(sm[k3], 30);
    assert_eq!(sm[k4], 4);


    let overlap = sm.get_disjoint_mut([k1, k1]);
    assert!(overlap.is_none());
    let overlap3 = sm.get_disjoint_mut([k2, k3, k2]);
    assert!(overlap3.is_none());


    sm.remove(k4);
    assert_eq!(sm.contains_key(k4), false);
    let invalid = sm.get_disjoint_mut([k1, k4]);
    assert!(invalid.is_none());


    assert_eq!(sm[k1], 10);
    assert_eq!(sm[k2], 20);
    assert_eq!(sm[k3], 30);
    assert_eq!(sm.len(), 3);


    let one = sm.get_disjoint_mut([k2]);
    assert!(one.is_some());
    let [only] = one.unwrap();
    *only = 200;
    assert_eq!(sm[k2], 200);
}

#[test]
fn test_get_disjoint_unchecked_mut_distinct_keys() {
    let mut sm: SlotMap<DefaultKey, String> = SlotMap::new();
    let k1 = sm.insert("alpha".to_string());
    let k2 = sm.insert("beta".to_string());
    let k3 = sm.insert("gamma".to_string());
    let k4 = sm.insert("delta".to_string());
    assert_eq!(sm.len(), 4);

    unsafe {
        let refs = sm.get_disjoint_unchecked_mut([k1, k2, k3]);
        assert_eq!(refs[0], "alpha");
        assert_eq!(refs[1], "beta");
        assert_eq!(refs[2], "gamma");
        refs[0].push_str("-X");
        refs[1].push_str("-Y");
        refs[2].push_str("-Z");
    }

    assert_eq!(sm[k1], "alpha-X");
    assert_eq!(sm[k2], "beta-Y");
    assert_eq!(sm[k3], "gamma-Z");
    assert_eq!(sm[k4], "delta");
    assert_eq!(sm.len(), 4);


    unsafe {
        let [d] = sm.get_disjoint_unchecked_mut([k4]);
        *d = "delta-W".to_string();
    }
    assert_eq!(sm[k4], "delta-W");


    unsafe {
        let [a, b] = sm.get_disjoint_unchecked_mut([k1, k4]);
        a.push('!');
        b.push('?');
    }
    assert_eq!(sm[k1], "alpha-X!");
    assert_eq!(sm[k4], "delta-W?");
}

#[test]
fn test_iter_mut_and_values_mut_mutation() {
    let mut sm: SlotMap<DefaultKey, i32> = SlotMap::new();
    let k1 = sm.insert(1);
    let k2 = sm.insert(2);
    let k3 = sm.insert(3);
    let k4 = sm.insert(4);
    let k5 = sm.insert(5);
    assert_eq!(sm.len(), 5);

    let mut count = 0;
    for (_k, v) in sm.iter_mut() {
        *v *= 2;
        count += 1;
    }
    assert_eq!(count, 5);
    assert_eq!(sm[k1], 2);
    assert_eq!(sm[k2], 4);
    assert_eq!(sm[k3], 6);
    assert_eq!(sm[k4], 8);
    assert_eq!(sm[k5], 10);

    let mut vcount = 0;
    let mut sum_before: i32 = 0;
    for v in sm.values_mut() {
        sum_before += *v;
        *v += 100;
        vcount += 1;
    }
    assert_eq!(vcount, 5);
    assert_eq!(sum_before, 2 + 4 + 6 + 8 + 10);
    assert_eq!(sm[k1], 102);
    assert_eq!(sm[k2], 104);
    assert_eq!(sm[k3], 106);
    assert_eq!(sm[k4], 108);
    assert_eq!(sm[k5], 110);

    sm.remove(k3);
    let mut after_count = 0;
    for (k, v) in sm.iter_mut() {
        assert!(k != k3);
        *v = 0;
        after_count += 1;
    }
    assert_eq!(after_count, 4);
    assert_eq!(sm.len(), 4);
    assert_eq!(sm[k1], 0);
    assert_eq!(sm[k2], 0);
    assert_eq!(sm[k4], 0);
    assert_eq!(sm[k5], 0);
    assert_eq!(sm.contains_key(k3), false);
}

#[test]
fn test_workflow_reserve_insert_with_key_retain_iter_mut() {
    let mut graph: SlotMap<DefaultKey, (DefaultKey, u32, bool)> = SlotMap::with_key();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);

    graph.reserve(1000);
    let cap = graph.capacity();
    assert!(cap >= 1000);

    let mut node_keys = Vec::with_capacity(500);
    for i in 0..500u32 {
        let k = graph.insert_with_key(|key| (key, i, i % 3 == 0));
        node_keys.push(k);
    }
    assert_eq!(graph.len(), 500);
    assert_eq!(graph.is_empty(), false);
    assert!(graph.capacity() >= cap);

    for k in &node_keys {
        let entry = &graph[*k];
        assert_eq!(entry.0, *k);
    }

    graph.retain(|_k, v| v.2);
    let expected_remaining = (0..500u32).filter(|i| i % 3 == 0).count();
    assert_eq!(graph.len(), expected_remaining);
    assert!(expected_remaining >= 160 && expected_remaining <= 170);

    for (k, v) in graph.iter_mut() {
        assert_eq!(v.0, k);
        assert_eq!(v.2, true);
        v.1 = v.1.saturating_add(10_000);
    }

    let mut min_val = u32::MAX;
    for v in graph.values_mut() {
        if v.1 < min_val {
            min_val = v.1;
        }
        *v = (v.0, v.1, false);
    }
    assert!(min_val >= 10_000);


    let before = graph.len();
    assert_eq!(before, expected_remaining);
    graph.retain(|_k, v| v.2);
    assert_eq!(graph.len(), 0);
    assert!(graph.is_empty());

    graph.clear();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
}