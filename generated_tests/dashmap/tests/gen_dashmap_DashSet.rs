use dashmap::DashSet;

#[test]
fn test_dashset_with_capacity_and_basic_ops() {
    let set: DashSet<i32> = DashSet::with_capacity(64);
    assert_eq!(set.len(), 0);
    assert!(set.is_empty());

    for i in 0..50 {
        set.insert(i);
    }
    assert_eq!(set.len(), 50);
    assert!(!set.is_empty());

    assert!(set.contains(&0));
    assert!(set.contains(&49));
    assert!(!set.contains(&100));
    assert!(!set.contains(&-1));
}

#[test]
fn test_dashset_hash_usize_consistency() {
    let set: DashSet<String> = DashSet::with_capacity(16);
    let key = String::from("hello");
    let h1 = set.hash_usize(&key);
    let h2 = set.hash_usize(&key);
    assert_eq!(h1, h2);

    let key2 = String::from("world");
    let h3 = set.hash_usize(&key2);

    assert_eq!(set.hash_usize(&key2), h3);

    set.insert(key.clone());
    assert!(set.contains(&key));
    assert_eq!(set.hash_usize(&key), h1);
}

#[test]
fn test_dashset_remove_if() {
    let set: DashSet<i32> = DashSet::new();
    for i in 0..10 {
        set.insert(i);
    }
    assert_eq!(set.len(), 10);


    let res = set.remove_if(&5, |_| false);
    assert!(res.is_none());
    assert!(set.contains(&5));
    assert_eq!(set.len(), 10);


    let res = set.remove_if(&5, |k| *k == 5);
    assert_eq!(res, Some(5));
    assert!(!set.contains(&5));
    assert_eq!(set.len(), 9);


    let res = set.remove_if(&999, |_| true);
    assert!(res.is_none());
    assert_eq!(set.len(), 9);
}

#[test]
fn test_dashset_retain_and_clear_and_shrink() {
    let set: DashSet<i32> = DashSet::with_capacity(128);
    for i in 0..100 {
        set.insert(i);
    }
    assert_eq!(set.len(), 100);

    set.retain(|k| *k % 2 == 0);
    assert_eq!(set.len(), 50);
    assert!(set.contains(&0));
    assert!(set.contains(&98));
    assert!(!set.contains(&1));
    assert!(!set.contains(&99));

    set.shrink_to_fit();
    assert_eq!(set.len(), 50);
    assert!(set.contains(&50));

    set.clear();
    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
    assert!(!set.contains(&0));

    set.shrink_to_fit();
    assert_eq!(set.len(), 0);

    set.insert(7);
    assert!(set.contains(&7));
    assert_eq!(set.len(), 1);
}