use dashmap::DashSet;

#[test]
fn set_capacity_and_hashing_are_consistent() {
    let set: DashSet<String> = DashSet::with_capacity(32);

    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert!(set.capacity() >= 32);

    let alpha = String::from("alpha");
    let beta = String::from("beta");

    let alpha_hash_first = set.hash_usize(&alpha);
    let alpha_hash_second = set.hash_usize(&alpha);
    let beta_hash = set.hash_usize(&beta);

    assert_eq!(alpha_hash_first, alpha_hash_second);

    assert!(set.insert(alpha.clone()));
    assert!(set.insert(beta.clone()));
    assert!(!set.insert(alpha.clone()));

    assert_eq!(set.len(), 2);
    assert!(set.contains("alpha"));
    assert!(set.contains("beta"));
    assert!(!set.contains("gamma"));

    let alpha_ref = set.get("alpha").expect("alpha should be present");
    assert_eq!(alpha_ref.key(), "alpha");
    drop(alpha_ref);

    assert_eq!(set.hash_usize(&alpha), alpha_hash_first);
    assert_eq!(set.hash_usize(&beta), beta_hash);
}

#[test]
fn set_remove_if_retain_shrink_and_clear_workflow() {
    let set: DashSet<String> = DashSet::with_capacity(64);

    for key in [
        "apple",
        "apricot",
        "banana",
        "blueberry",
        "blackberry",
        "cherry",
        "clementine",
        "date",
    ] {
        assert!(set.insert(key.to_string()), "{key} should be newly inserted");
    }

    assert_eq!(set.len(), 8);
    assert!(set.contains("banana"));
    assert!(set.contains("blackberry"));
    assert!(set.contains("date"));

    let not_removed = set.remove_if("banana", |key| key.starts_with('a'));
    assert_eq!(not_removed, None);
    assert!(set.contains("banana"));
    assert_eq!(set.len(), 8);

    let removed = set.remove_if("banana", |key| key.starts_with('b'));
    assert_eq!(removed.as_deref(), Some("banana"));
    assert!(!set.contains("banana"));
    assert_eq!(set.len(), 7);

    let missing = set.remove_if("missing", |_| true);
    assert_eq!(missing, None);
    assert_eq!(set.len(), 7);

    set.retain(|key| key.starts_with('a') || key.starts_with('c'));

    assert_eq!(set.len(), 4);
    assert!(set.contains("apple"));
    assert!(set.contains("apricot"));
    assert!(set.contains("cherry"));
    assert!(set.contains("clementine"));
    assert!(!set.contains("blueberry"));
    assert!(!set.contains("blackberry"));
    assert!(!set.contains("date"));

    let capacity_before_shrink = set.capacity();
    set.shrink_to_fit();
    let capacity_after_shrink = set.capacity();

    assert!(capacity_after_shrink >= set.len());
    assert!(capacity_after_shrink <= capacity_before_shrink);

    set.clear();

    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert!(!set.contains("apple"));
    assert!(!set.contains("clementine"));

    set.shrink_to_fit();
    assert!(set.capacity() >= set.len());
}