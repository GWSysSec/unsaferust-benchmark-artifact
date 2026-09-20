use dashmap::DashMap;

#[test]
fn iter_mut_updates_every_entry_and_preserves_keys() {
    let scores: DashMap<String, i32> = DashMap::with_capacity(8);

    assert!(scores.is_empty());
    assert_eq!(scores.len(), 0);

    assert_eq!(scores.insert("alpha".to_string(), 10), None);
    assert_eq!(scores.insert("beta".to_string(), 20), None);
    assert_eq!(scores.insert("gamma".to_string(), 30), None);
    assert_eq!(scores.insert("delta".to_string(), 40), None);

    assert_eq!(scores.len(), 4);
    assert!(scores.contains_key("alpha"));
    assert!(scores.contains_key("beta"));
    assert!(scores.contains_key("gamma"));
    assert!(scores.contains_key("delta"));

    scores.iter_mut().for_each(|mut entry| {
        let increment = entry.key().len() as i32;
        *entry.value_mut() += increment;
    });

    assert_eq!(*scores.get("alpha").expect("alpha should exist"), 15);
    assert_eq!(*scores.get("beta").expect("beta should exist"), 24);
    assert_eq!(*scores.get("gamma").expect("gamma should exist"), 35);
    assert_eq!(*scores.get("delta").expect("delta should exist"), 45);

    let total_after_first_update: i32 = scores.iter_mut().map(|entry| *entry.value()).sum();
    assert_eq!(total_after_first_update, 119);

    scores.iter_mut().for_each(|mut entry| {
        if entry.key().starts_with('g') || entry.key().starts_with('d') {
            *entry.value_mut() *= 2;
        } else {
            *entry.value_mut() -= 1;
        }
    });

    assert_eq!(*scores.get("alpha").expect("alpha should still exist"), 14);
    assert_eq!(*scores.get("beta").expect("beta should still exist"), 23);
    assert_eq!(*scores.get("gamma").expect("gamma should still exist"), 70);
    assert_eq!(*scores.get("delta").expect("delta should still exist"), 90);

    let final_total: i32 = scores.iter_mut().map(|entry| *entry.value()).sum();
    assert_eq!(final_total, 197);
    assert_eq!(scores.len(), 4);
}

#[test]
fn iter_mut_handles_empty_and_single_entry_maps() {
    let empty: DashMap<&'static str, usize> = DashMap::new();

    let empty_count = empty.iter_mut().count();
    assert_eq!(empty_count, 0);
    assert!(empty.is_empty());

    let single: DashMap<&'static str, usize> = DashMap::new();
    assert_eq!(single.insert("only", 1), None);

    single.iter_mut().for_each(|mut entry| {
        assert_eq!(*entry.key(), "only");
        *entry.value_mut() += 41;
    });

    assert_eq!(single.len(), 1);
    assert!(single.contains_key("only"));
    assert_eq!(*single.get("only").expect("single entry should exist"), 42);

    let seen_values: Vec<usize> = single.iter_mut().map(|entry| *entry.value()).collect();
    assert_eq!(seen_values, vec![42]);
}