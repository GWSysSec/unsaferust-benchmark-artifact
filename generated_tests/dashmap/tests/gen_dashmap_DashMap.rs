use dashmap::DashMap;
use std::collections::hash_map::RandomState;

#[test]
fn test_capacity_and_shard_constructors() {
    let m: DashMap<i32, String> = DashMap::with_capacity(128);
    assert_eq!(m.len(), 0);
    assert!(m.is_empty());
    for i in 0..50 {
        m.insert(i, format!("v{}", i));
    }
    assert_eq!(m.len(), 50);
    assert_eq!(m.get(&7).unwrap().value(), "v7");

    let m2: DashMap<i32, i32> = DashMap::with_shard_amount(4);
    for i in 0..20 {
        m2.insert(i, i * 2);
    }
    assert_eq!(m2.len(), 20);
    assert_eq!(*m2.get(&10).unwrap(), 20);

    let m3: DashMap<i32, i32> = DashMap::with_capacity_and_shard_amount(64, 8);
    for i in 0..32 {
        m3.insert(i, i + 1);
    }
    assert_eq!(m3.len(), 32);
    assert_eq!(*m3.get(&0).unwrap(), 1);
    assert_eq!(*m3.get(&31).unwrap(), 32);

    let m4: DashMap<String, i32, RandomState> =
        DashMap::with_hasher_and_shard_amount(RandomState::new(), 4);
    m4.insert("hello".to_string(), 42);
    m4.insert("world".to_string(), 99);
    assert_eq!(m4.len(), 2);
    assert_eq!(*m4.get("hello").unwrap(), 42);
}

#[test]
fn test_hash_usize_and_hasher() {
    let m: DashMap<String, i32> = DashMap::new();
    let _h: &RandomState = m.hasher();

    let h1 = m.hash_usize(&"abc".to_string());
    let h2 = m.hash_usize(&"abc".to_string());
    assert_eq!(h1, h2, "hash must be deterministic per-instance");

    let h3 = m.hash_usize(&"different".to_string());

    assert_ne!(h1, h3);

    m.insert("abc".to_string(), 1);
    assert_eq!(*m.get("abc").unwrap(), 1);
}

#[test]
fn test_remove_if_and_remove_if_mut() {
    let m: DashMap<i32, i32> = DashMap::new();
    for i in 0..10 {
        m.insert(i, i * 10);
    }
    assert_eq!(m.len(), 10);


    let r = m.remove_if(&3, |_k, v| *v > 1000);
    assert!(r.is_none());
    assert!(m.contains_key(&3));


    let r = m.remove_if(&3, |k, v| *k == 3 && *v == 30);
    assert_eq!(r, Some((3, 30)));
    assert!(!m.contains_key(&3));
    assert_eq!(m.len(), 9);


    let r = m.remove_if(&999, |_, _| true);
    assert!(r.is_none());


    let r = m.remove_if_mut(&5, |_k, v| {
        *v += 1;
        false
    });
    assert!(r.is_none());
    assert!(m.contains_key(&5));

    let r = m.remove_if_mut(&5, |_k, _v| true);
    assert_eq!(r.map(|(k, _)| k), Some(5));
    assert_eq!(m.len(), 8);
}

#[test]
fn test_retain_clear_shrink() {
    let m: DashMap<i32, i32> = DashMap::with_capacity(256);
    for i in 0..100 {
        m.insert(i, i);
    }
    assert_eq!(m.len(), 100);

    m.retain(|_k, v| *v % 2 == 0);
    assert_eq!(m.len(), 50);
    assert!(m.contains_key(&0));
    assert!(!m.contains_key(&1));
    assert!(m.contains_key(&98));

    m.shrink_to_fit();
    assert_eq!(m.len(), 50);
    assert_eq!(*m.get(&50).unwrap(), 50);

    m.clear();
    assert_eq!(m.len(), 0);
    assert!(m.is_empty());
    assert!(m.get(&0).is_none());
}

#[test]
fn test_alter_and_alter_all() {
    let m: DashMap<&'static str, i32> = DashMap::new();
    m.insert("a", 1);
    m.insert("b", 2);
    m.insert("c", 3);

    m.alter("a", |_k, v| v + 100);
    assert_eq!(*m.get("a").unwrap(), 101);
    assert_eq!(*m.get("b").unwrap(), 2);


    m.alter("missing", |_k, v| v + 1);
    assert!(!m.contains_key("missing"));

    m.alter_all(|_k, v| v * 2);
    assert_eq!(*m.get("a").unwrap(), 202);
    assert_eq!(*m.get("b").unwrap(), 4);
    assert_eq!(*m.get("c").unwrap(), 6);
    assert_eq!(m.len(), 3);
}

#[test]
fn test_view_and_try_entry() {
    let m: DashMap<i32, String> = DashMap::with_capacity_and_shard_amount(32, 4);
    m.insert(1, "one".to_string());
    m.insert(2, "two".to_string());

    let r = m.view(&1, |k, v| (*k, v.clone()));
    assert_eq!(r, Some((1, "one".to_string())));

    let none = m.view(&999, |_, _| 0);
    assert!(none.is_none());


    {
        let e = m.try_entry(3).expect("try_entry should succeed");
        e.or_insert("three".to_string());
    }
    assert_eq!(*m.get(&3).unwrap(), "three");


    {
        let e = m.try_entry(3).unwrap();
        e.and_modify(|v| v.push_str("!")).or_insert("x".to_string());
    }
    assert_eq!(*m.get(&3).unwrap(), "three!");



    let key_a = 100;
    m.insert(key_a, "a".to_string());
    let _guard = m.get_mut(&key_a).unwrap();
    let blocked = m.try_entry(key_a);
    assert!(blocked.is_none());
    drop(_guard);
    assert!(m.try_entry(key_a).is_some());
}