use dashmap::mapref::entry::Entry;
use dashmap::DashMap;

#[test]
fn test_occupied_entry_into_ref_mutates_in_place() {
    let map: DashMap<String, i64> = DashMap::new();
    map.insert("counter".to_string(), 10);
    assert_eq!(map.len(), 1);
    assert_eq!(*map.get("counter").unwrap().value(), 10);

    match map.entry("counter".to_string()) {
        Entry::Occupied(occ) => {

            assert_eq!(*occ.get(), 10);
            let mut r = occ.into_ref();
            assert_eq!(*r.value(), 10);
            *r.value_mut() = 250;
            assert_eq!(*r.value(), 250);
        }
        Entry::Vacant(_) => panic!("expected Occupied entry"),
    }

    assert_eq!(map.len(), 1);
    assert_eq!(*map.get("counter").unwrap().value(), 250);
    assert!(map.contains_key("counter"));
}

#[test]
fn test_occupied_entry_into_key_returns_supplied_key() {
    let map: DashMap<String, u32> = DashMap::new();
    map.insert("present".to_string(), 1);
    map.insert("other".to_string(), 2);
    assert_eq!(map.len(), 2);

    let supplied = "present".to_string();
    let supplied_ptr = supplied.as_ptr();
    let supplied_len = supplied.len();

    match map.entry(supplied) {
        Entry::Occupied(occ) => {
            assert_eq!(*occ.get(), 1);
            let k = occ.into_key();
            assert_eq!(k, "present");
            assert_eq!(k.len(), supplied_len);

            assert_eq!(k.as_ptr(), supplied_ptr);
        }
        Entry::Vacant(_) => panic!("expected Occupied"),
    }


    assert_eq!(map.len(), 2);
    assert!(map.contains_key("present"));
    assert!(map.contains_key("other"));
    assert_eq!(*map.get("present").unwrap().value(), 1);
}

#[test]
fn test_occupied_entry_remove_entry_returns_kv_and_removes() {
    let map: DashMap<String, Vec<u8>> = DashMap::new();
    map.insert("a".to_string(), vec![1, 2, 3]);
    map.insert("b".to_string(), vec![9]);
    map.insert("c".to_string(), vec![]);
    assert_eq!(map.len(), 3);
    assert!(map.contains_key("a"));

    match map.entry("a".to_string()) {
        Entry::Occupied(occ) => {
            assert_eq!(occ.get().len(), 3);
            let (k, v) = occ.remove_entry();
            assert_eq!(k, "a");
            assert_eq!(k.len(), 1);
            assert_eq!(v, vec![1, 2, 3]);
            assert_eq!(v.len(), 3);
        }
        Entry::Vacant(_) => panic!("expected Occupied"),
    }

    assert_eq!(map.len(), 2);
    assert!(!map.contains_key("a"));
    assert!(map.contains_key("b"));
    assert!(map.contains_key("c"));


    match map.entry("b".to_string()) {
        Entry::Occupied(occ) => {
            let (k, v) = occ.remove_entry();
            assert_eq!(k, "b");
            assert_eq!(v, vec![9]);
        }
        Entry::Vacant(_) => panic!("expected Occupied"),
    }
    assert_eq!(map.len(), 1);
    match map.entry("c".to_string()) {
        Entry::Occupied(occ) => {
            let (k, v) = occ.remove_entry();
            assert_eq!(k, "c");
            assert_eq!(v.len(), 0);
        }
        Entry::Vacant(_) => panic!("expected Occupied"),
    }
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
}

#[test]
fn test_occupied_entry_replace_entry_returns_old_kv() {
    let map: DashMap<String, String> = DashMap::new();
    let original_key = "name".to_string();
    let original_key_ptr = original_key.as_ptr();
    map.insert(original_key, "alice".to_string());
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("name").unwrap().value(), "alice");



    let new_supplied_key = "name".to_string();
    let new_supplied_key_ptr = new_supplied_key.as_ptr();
    assert_ne!(original_key_ptr, new_supplied_key_ptr);

    match map.entry(new_supplied_key) {
        Entry::Occupied(occ) => {
            assert_eq!(occ.get(), "alice");
            let (old_k, old_v) = occ.replace_entry("bob".to_string());
            assert_eq!(old_k, "name");
            assert_eq!(old_v, "alice");
            assert_eq!(old_v.len(), 5);

            assert_eq!(old_k.as_ptr(), original_key_ptr);
        }
        Entry::Vacant(_) => panic!("expected Occupied"),
    }

    assert_eq!(map.len(), 1);
    assert!(map.contains_key("name"));
    assert_eq!(map.get("name").unwrap().value(), "bob");
    assert_eq!(map.get("name").unwrap().value().len(), 3);
}

#[test]
fn test_occupied_entry_combined_workflow() {


    let map: DashMap<String, i32> = DashMap::new();
    for (k, v) in [("x", 1), ("y", 2), ("z", 3), ("w", 4)] {
        map.insert(k.to_string(), v);
    }
    assert_eq!(map.len(), 4);


    match map.entry("x".to_string()) {
        Entry::Occupied(occ) => {
            let mut r = occ.into_ref();
            *r.value_mut() *= 10;
        }
        Entry::Vacant(_) => panic!("x should be occupied"),
    }
    assert_eq!(*map.get("x").unwrap().value(), 10);


    let (old_k, old_v) = match map.entry("y".to_string()) {
        Entry::Occupied(occ) => occ.replace_entry(222),
        Entry::Vacant(_) => panic!("y should be occupied"),
    };
    assert_eq!(old_k, "y");
    assert_eq!(old_v, 2);
    assert_eq!(*map.get("y").unwrap().value(), 222);


    let (rk, rv) = match map.entry("z".to_string()) {
        Entry::Occupied(occ) => occ.remove_entry(),
        Entry::Vacant(_) => panic!("z should be occupied"),
    };
    assert_eq!(rk, "z");
    assert_eq!(rv, 3);
    assert!(!map.contains_key("z"));
    assert_eq!(map.len(), 3);


    let supplied = "w".to_string();
    let p = supplied.as_ptr();
    let returned = match map.entry(supplied) {
        Entry::Occupied(occ) => {
            assert_eq!(*occ.get(), 4);
            occ.into_key()
        }
        Entry::Vacant(_) => panic!("w should be occupied"),
    };
    assert_eq!(returned, "w");
    assert_eq!(returned.as_ptr(), p);
    assert_eq!(map.len(), 3);
    assert!(map.contains_key("w"));
}