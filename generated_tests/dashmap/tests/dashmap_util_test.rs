use dashmap::DashMap;

#[test]
fn alter_replaces_owned_value_using_context_key() {
    let map: DashMap<&'static str, String> = DashMap::new();

    assert_eq!(map.insert("user-7", String::from("initial")), None);

    map.alter("user-7", |key, old_value| {
        assert_eq!(*key, "user-7");
        assert_eq!(old_value, "initial");
        format!("{key}:{old_value}:updated")
    });

    let updated = map.get("user-7").expect("user-7 entry should exist");
    assert_eq!(&*updated, "user-7:initial:updated");
}

#[test]
fn alter_handles_empty_and_non_copy_values() {
    let map: DashMap<&'static str, Vec<usize>> = DashMap::new();

    assert_eq!(map.insert("values", Vec::new()), None);

    map.alter("values", |key, old_values| {
        assert_eq!(*key, "values");
        assert!(old_values.is_empty());
        (1..=4).collect()
    });

    {
        let updated = map.get("values").expect("values entry should exist");
        assert_eq!(&*updated, &vec![1, 2, 3, 4]);
    }

    map.alter("values", |_key, old_values| {
        old_values.into_iter().map(|value| value + 10).collect()
    });

    let updated = map.get("values").expect("values entry should still exist");
    assert_eq!(&*updated, &vec![11, 12, 13, 14]);
}

#[test]
fn alter_can_update_dashmap_entries_in_place() {
    let map: DashMap<&'static str, Vec<i32>> = DashMap::new();

    assert!(map.is_empty());
    assert_eq!(map.insert("numbers", vec![1, 2, 3]), None);
    assert_eq!(map.len(), 1);

    map.alter("numbers", |entry_key, mut old_values| {
        assert_eq!(*entry_key, "numbers");
        old_values.push(entry_key.len() as i32);
        old_values.into_iter().map(|value| value * 2).collect()
    });

    let updated = map.get("numbers").expect("numbers entry should still exist");
    assert_eq!(&*updated, &vec![2, 4, 6, 14]);
    assert!(map.contains_key("numbers"));
}