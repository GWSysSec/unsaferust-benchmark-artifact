use slotmap::hop::HopSlotMap;
use slotmap::{DefaultKey, Key};

#[test]
fn test_with_key_is_empty_capacity_reserve() {
    let mut sm: HopSlotMap<DefaultKey, u32> = HopSlotMap::with_key();
    assert_eq!(sm.is_empty(), true);
    assert_eq!(sm.len(), 0);

    let cap_before = sm.capacity();
    sm.reserve(1000);
    let cap_after_reserve = sm.capacity();
    assert!(cap_after_reserve >= 1000, "cap_after_reserve={}", cap_after_reserve);
    assert!(cap_after_reserve >= cap_before);
    assert_eq!(sm.is_empty(), true);
    assert_eq!(sm.len(), 0);

    let mut keys = Vec::with_capacity(500);
    for i in 0..500u32 {
        keys.push(sm.insert(i));
    }
    assert_eq!(sm.len(), 500);
    assert_eq!(sm.is_empty(), false);
    assert!(sm.capacity() >= cap_after_reserve);
    assert_eq!(sm[keys[0]], 0);
    assert_eq!(sm[keys[249]], 249);
    assert_eq!(sm[keys[499]], 499);


    let cap_pre2 = sm.capacity();
    sm.reserve(10);
    assert!(sm.capacity() >= cap_pre2);
    assert_eq!(sm.len(), 500);
}

#[test]
fn test_insert_with_key_self_reference() {
    let mut sm: HopSlotMap<DefaultKey, (DefaultKey, String)> = HopSlotMap::with_key();
    assert_eq!(sm.is_empty(), true);

    let k1 = sm.insert_with_key(|k| (k, "first".to_string()));
    assert_eq!(sm.len(), 1);
    assert_eq!(sm[k1].0, k1);
    assert_eq!(sm[k1].1, "first");

    let k2 = sm.insert_with_key(|k| (k, format!("id={}", k.data().as_ffi())));
    assert_ne!(k1, k2);
    assert_eq!(sm.len(), 2);
    assert_eq!(sm[k2].0, k2);
    assert!(sm[k2].1.starts_with("id="));

    assert_eq!(sm[k1].0, k1);
    assert_eq!(sm[k1].1, "first");


    let mut ks = Vec::new();
    for _ in 0..50 {
        ks.push(sm.insert_with_key(|k| (k, String::new())));
    }
    for k in &ks {
        assert_eq!(sm[*k].0, *k);
    }
    assert_eq!(sm.len(), 52);
}

#[test]
fn test_try_insert_with_key_success_and_failure() {
    let mut sm: HopSlotMap<DefaultKey, String> = HopSlotMap::with_key();

    let ok_result: Result<DefaultKey, &'static str> =
        sm.try_insert_with_key(|_k| Ok("hello".to_string()));
    assert!(ok_result.is_ok());
    let k1 = ok_result.unwrap();
    assert_eq!(sm.len(), 1);
    assert_eq!(sm[k1], "hello");

    let err_result: Result<DefaultKey, &'static str> =
        sm.try_insert_with_key(|_k| Err("nope"));
    assert!(err_result.is_err());
    assert_eq!(err_result.unwrap_err(), "nope");
    assert_eq!(sm.len(), 1);
    assert_eq!(sm[k1], "hello");
    assert!(sm.get(k1).is_some());


    let ok2: Result<DefaultKey, &'static str> =
        sm.try_insert_with_key(|k| Ok(format!("ffi={}", k.data().as_ffi())));
    assert!(ok2.is_ok());
    let k2 = ok2.unwrap();
    assert_ne!(k1, k2);
    assert_eq!(sm.len(), 2);
    assert_eq!(sm[k2], format!("ffi={}", k2.data().as_ffi()));


    let mut inserted = 0usize;
    for i in 0..20 {
        let r: Result<DefaultKey, i32> = sm.try_insert_with_key(|_k| {
            if i % 2 == 0 { Ok(format!("v{}", i)) } else { Err(i) }
        });
        if r.is_ok() { inserted += 1; }
    }
    assert_eq!(inserted, 10);
    assert_eq!(sm.len(), 12);
}

#[test]
fn test_retain_keeps_even_values() {
    let mut sm: HopSlotMap<DefaultKey, i32> = HopSlotMap::with_key();
    let mut keys = Vec::new();
    for i in 0..20 {
        keys.push(sm.insert(i));
    }
    assert_eq!(sm.len(), 20);
    assert_eq!(sm.is_empty(), false);

    let mut visited = 0usize;
    sm.retain(|_k, v| {
        visited += 1;
        *v % 2 == 0
    });
    assert_eq!(visited, 20);
    assert_eq!(sm.len(), 10);

    for (i, k) in keys.iter().enumerate() {
        if i % 2 == 0 {
            assert_eq!(sm.get(*k), Some(&(i as i32)));
        } else {
            assert_eq!(sm.get(*k), None);
        }
    }


    sm.retain(|_k, v| { *v += 100; true });
    assert_eq!(sm.len(), 10);
    assert_eq!(sm[keys[0]], 100);
    assert_eq!(sm[keys[2]], 102);
    assert_eq!(sm[keys[18]], 118);


    sm.retain(|_k, _v| false);
    assert_eq!(sm.len(), 0);
    assert_eq!(sm.is_empty(), true);
}

#[test]
fn test_clear_preserves_capacity_invalidates_keys() {
    let mut sm: HopSlotMap<DefaultKey, String> = HopSlotMap::with_key();
    sm.reserve(64);
    let cap_reserved = sm.capacity();
    assert!(cap_reserved >= 64);

    let k1 = sm.insert("a".to_string());
    let k2 = sm.insert("b".to_string());
    let k3 = sm.insert("c".to_string());
    assert_eq!(sm.len(), 3);
    assert_eq!(sm.is_empty(), false);

    sm.clear();
    assert_eq!(sm.len(), 0);
    assert_eq!(sm.is_empty(), true);
    assert!(sm.capacity() >= cap_reserved);
    assert_eq!(sm.get(k1), None);
    assert_eq!(sm.get(k2), None);
    assert_eq!(sm.get(k3), None);


    let k4 = sm.insert("new".to_string());
    assert_eq!(sm.len(), 1);
    assert_eq!(sm[k4], "new");
    assert_ne!(k4, k1);




    sm.clear();
    assert_eq!(sm.len(), 0);
    assert_eq!(sm.get(k4), None);
}

#[test]
fn test_get_unchecked_mut_unsafe() {
    let mut sm: HopSlotMap<DefaultKey, i64> = HopSlotMap::with_key();
    let k1 = sm.insert(100);
    let k2 = sm.insert(200);
    let k3 = sm.insert(300);
    assert_eq!(sm.len(), 3);
    assert_eq!(sm[k1], 100);
    assert_eq!(sm[k2], 200);
    assert_eq!(sm[k3], 300);

    unsafe {
        let v = sm.get_unchecked_mut(k2);
        assert_eq!(*v, 200);
        *v = 999;
    }
    assert_eq!(sm[k1], 100);
    assert_eq!(sm[k2], 999);
    assert_eq!(sm[k3], 300);

    unsafe {
        *sm.get_unchecked_mut(k1) += 1;
        *sm.get_unchecked_mut(k3) -= 50;
    }
    assert_eq!(sm[k1], 101);
    assert_eq!(sm[k3], 250);
    assert_eq!(sm.len(), 3);
}

#[test]
fn test_get_disjoint_mut_valid_duplicate_and_removed() {
    let mut sm: HopSlotMap<DefaultKey, u32> = HopSlotMap::with_key();
    let k1 = sm.insert(1);
    let k2 = sm.insert(2);
    let k3 = sm.insert(3);
    let k4 = sm.insert(4);
    assert_eq!(sm.len(), 4);


    let got = sm.get_disjoint_mut([k1, k2, k3]);
    assert!(got.is_some());
    let refs = got.unwrap();
    assert_eq!(*refs[0], 1);
    assert_eq!(*refs[1], 2);
    assert_eq!(*refs[2], 3);
    *refs[0] = 10;
    *refs[1] = 20;
    *refs[2] = 30;

    assert_eq!(sm[k1], 10);
    assert_eq!(sm[k2], 20);
    assert_eq!(sm[k3], 30);
    assert_eq!(sm[k4], 4);


    let dup = sm.get_disjoint_mut([k1, k1]);
    assert!(dup.is_none());
    assert_eq!(sm[k1], 10);


    let removed = sm.remove(k4);
    assert_eq!(removed, Some(4));
    let bad = sm.get_disjoint_mut([k1, k4]);
    assert!(bad.is_none());


    let one = sm.get_disjoint_mut([k2]);
    assert!(one.is_some());
    let [r] = one.unwrap();
    assert_eq!(*r, 20);
    *r = 222;
    assert_eq!(sm[k2], 222);
}

#[test]
fn test_get_disjoint_unchecked_mut_unsafe() {
    let mut sm: HopSlotMap<DefaultKey, String> = HopSlotMap::with_key();
    let k1 = sm.insert("alpha".to_string());
    let k2 = sm.insert("beta".to_string());
    let k3 = sm.insert("gamma".to_string());
    let k4 = sm.insert("delta".to_string());
    assert_eq!(sm.len(), 4);

    unsafe {
        let refs = sm.get_disjoint_unchecked_mut([k1, k2, k3, k4]);
        assert_eq!(refs[0], "alpha");
        assert_eq!(refs[1], "beta");
        assert_eq!(refs[2], "gamma");
        assert_eq!(refs[3], "delta");
        refs[0].push_str("-X");
        refs[1].push_str("-Y");
        refs[2].push_str("-Z");
        refs[3].push_str("-W");
    }

    assert_eq!(sm[k1], "alpha-X");
    assert_eq!(sm[k2], "beta-Y");
    assert_eq!(sm[k3], "gamma-Z");
    assert_eq!(sm[k4], "delta-W");
    assert_eq!(sm.len(), 4);


    unsafe {
        let [r] = sm.get_disjoint_unchecked_mut([k2]);
        r.clear();
        r.push_str("reset");
    }
    assert_eq!(sm[k2], "reset");
    assert_eq!(sm[k1], "alpha-X");
}

#[test]
fn test_iter_mut_modifies_all_values() {
    let mut sm: HopSlotMap<DefaultKey, i32> = HopSlotMap::with_key();
    let keys: Vec<_> = (0..10).map(|i| sm.insert(i)).collect();
    assert_eq!(sm.len(), 10);

    let mut count = 0usize;
    let mut sum_before = 0i32;
    for (_k, v) in sm.iter_mut() {
        sum_before += *v;
        *v *= 2;
        count += 1;
    }
    assert_eq!(count, 10);
    assert_eq!(sum_before, 45);

    for (i, k) in keys.iter().enumerate() {
        assert_eq!(sm[*k], (i as i32) * 2);
    }


    sm.remove(keys[3]);
    sm.remove(keys[7]);
    assert_eq!(sm.len(), 8);

    let mut seen_keys = Vec::new();
    for (k, v) in sm.iter_mut() {
        seen_keys.push(k);
        *v = -*v;
    }
    assert_eq!(seen_keys.len(), 8);
    assert!(!seen_keys.contains(&keys[3]));
    assert!(!seen_keys.contains(&keys[7]));

    assert_eq!(sm[keys[0]], 0);
    assert_eq!(sm[keys[1]], -2);
    assert_eq!(sm[keys[9]], -18);
    assert_eq!(sm.get(keys[3]), None);
    assert_eq!(sm.get(keys[7]), None);
}

#[test]
fn test_values_mut_uppercase_and_length_invariants() {
    let mut sm: HopSlotMap<DefaultKey, String> = HopSlotMap::with_key();
    let k1 = sm.insert("one".to_string());
    let k2 = sm.insert("two".to_string());
    let k3 = sm.insert("three".to_string());
    assert_eq!(sm.len(), 3);

    let total_before: usize = sm.values().map(|s| s.len()).sum();
    assert_eq!(total_before, 3 + 3 + 5);

    let mut visits = 0usize;
    for v in sm.values_mut() {
        *v = v.to_uppercase();
        visits += 1;
    }
    assert_eq!(visits, 3);

    let total_after: usize = sm.values().map(|s| s.len()).sum();
    assert_eq!(total_after, total_before);

    assert_eq!(sm[k1], "ONE");
    assert_eq!(sm[k2], "TWO");
    assert_eq!(sm[k3], "THREE");


    for v in sm.values_mut() {
        v.push('!');
    }
    assert_eq!(sm[k1], "ONE!");
    assert_eq!(sm[k2], "TWO!");
    assert_eq!(sm[k3], "THREE!");
    assert_eq!(sm.len(), 3);
}