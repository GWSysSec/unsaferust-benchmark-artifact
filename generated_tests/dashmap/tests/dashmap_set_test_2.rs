use dashmap::DashSet;

#[test]
fn set_hashing_is_stable_and_tracks_inserted_keys() {
    let set: DashSet<String> = DashSet::with_capacity(32);

    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert!(!set.contains("alpha"));

    let alpha = String::from("alpha");
    let beta = String::from("beta");
    let gamma = String::from("gamma");

    let alpha_hash = set.hash_usize(&alpha);
    let beta_hash = set.hash_usize(&beta);
    let gamma_hash = set.hash_usize(&gamma);

    assert_eq!(alpha_hash, set.hash_usize(&"alpha"));
    assert_eq!(beta_hash, set.hash_usize(&"beta"));
    assert_eq!(gamma_hash, set.hash_usize(&"gamma"));

    assert!(set.insert(alpha.clone()));
    assert!(set.insert(beta.clone()));
    assert!(set.insert(gamma.clone()));
    assert!(!set.insert(alpha.clone()));

    assert_eq!(set.len(), 3);
    assert!(set.contains("alpha"));
    assert!(set.contains("beta"));
    assert!(set.contains("gamma"));

    let alpha_ref = set
        .get("alpha")
        .expect("alpha should be present after insertion");
    assert_eq!(alpha_ref.key(), "alpha");
    drop(alpha_ref);

    assert_eq!(set.hash_usize(&alpha), alpha_hash);
    assert_eq!(set.hash_usize(&beta), beta_hash);
    assert_eq!(set.hash_usize(&gamma), gamma_hash);
}

#[test]
fn set_remove_if_only_removes_when_predicate_accepts() {
    let set: DashSet<String> = DashSet::new();

    for value in ["apple", "apricot", "banana", "blueberry", "cherry"] {
        assert!(set.insert(value.to_string()));
    }

    assert_eq!(set.len(), 5);

    let rejected = set.remove_if("apple", |key| key.starts_with('b'));
    assert_eq!(rejected, None);
    assert!(set.contains("apple"));
    assert_eq!(set.len(), 5);

    let removed = set.remove_if("apple", |key| key.starts_with('a'));
    assert_eq!(removed, Some("apple".to_string()));
    assert!(!set.contains("apple"));
    assert_eq!(set.len(), 4);

    let missing = set.remove_if("does-not-exist", |_| true);
    assert_eq!(missing, None);
    assert_eq!(set.len(), 4);

    let still_present = set
        .get("banana")
        .expect("banana should not have been removed");
    assert_eq!(still_present.key(), "banana");
    drop(still_present);

    let removed_banana = set.remove_if("banana", |key| key.len() == "banana".len());
    assert_eq!(removed_banana, Some("banana".to_string()));
    assert!(!set.contains("banana"));
    assert!(set.contains("apricot"));
    assert!(set.contains("blueberry"));
    assert!(set.contains("cherry"));
    assert_eq!(set.len(), 3);
}

#[test]
fn set_retain_filters_existing_items_and_allows_further_mutation() {
    let set: DashSet<i32> = DashSet::with_capacity(16);

    for value in 0..10 {
        assert!(set.insert(value));
    }

    assert_eq!(set.len(), 10);
    assert!(set.capacity() >= 10);

    set.retain(|value| value % 2 == 0);

    assert_eq!(set.len(), 5);
    for value in [0, 2, 4, 6, 8] {
        assert!(set.contains(&value), "expected even value {value} to remain");
    }
    for value in [1, 3, 5, 7, 9] {
        assert!(
            !set.contains(&value),
            "expected odd value {value} to be removed"
        );
    }

    assert!(!set.insert(2));
    assert!(set.insert(11));
    assert!(set.insert(12));
    assert_eq!(set.len(), 7);

    set.retain(|value| *value >= 6);

    assert_eq!(set.len(), 4);
    assert!(!set.contains(&0));
    assert!(!set.contains(&2));
    assert!(!set.contains(&4));
    assert!(set.contains(&6));
    assert!(set.contains(&8));
    assert!(set.contains(&11));
    assert!(set.contains(&12));

    let mut retained_values: Vec<i32> = set.iter().map(|item| *item.key()).collect();
    retained_values.sort_unstable();
    assert_eq!(retained_values, vec![6, 8, 11, 12]);

    set.clear();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn set_retain_can_remove_everything_and_remove_if_stays_safe_on_empty_set() {
    let set: DashSet<&'static str> = DashSet::new();

    assert!(set.insert("temporary"));
    assert!(set.insert("transient"));
    assert_eq!(set.len(), 2);

    set.retain(|_| false);

    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert!(!set.contains("temporary"));
    assert!(!set.contains("transient"));

    assert_eq!(set.remove_if("temporary", |_| true), None);

    assert!(set.insert("persistent"));
    assert_eq!(set.len(), 1);
    assert_eq!(
        set.remove_if("persistent", |key| *key == "persistent"),
        Some("persistent")
    );
    assert!(set.is_empty());
}