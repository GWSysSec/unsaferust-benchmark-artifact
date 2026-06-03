use hashbrown::HashMap;
use hashbrown::hash_map::RawEntryMut;
use core::hash::{BuildHasher, Hash, Hasher};

fn compute_hash<K: Hash + ?Sized, S: BuildHasher>(hash_builder: &S, key: &K) -> u64 {
    let mut state = hash_builder.build_hasher();
    key.hash(&mut state);
    state.finish()
}

#[test]
fn test_raw_occupied_entry_mut_key_mut() {
    let mut map = HashMap::new();
    map.insert(String::from("alpha"), 100);
    map.insert(String::from("beta"), 200);
    map.insert(String::from("gamma"), 300);

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("alpha"), Some(&100));

    let hash = compute_hash(map.hasher(), "alpha");
    match map.raw_entry_mut().from_hash(hash, |q| q == "alpha") {
        RawEntryMut::Occupied(mut o) => {
            let key = o.key_mut();
            assert_eq!(key, "alpha");

            key.push_str("_modified");
            assert_eq!(o.get(), &100);
        }
        RawEntryMut::Vacant(_) => unreachable!(),
    }


    assert_eq!(map.len(), 3);


    let found = map.iter().any(|(k, v)| k == "alpha_modified" && *v == 100);
    assert!(found);


    assert_eq!(map.get("beta"), Some(&200));
    assert_eq!(map.get("gamma"), Some(&300));
}

#[test]
fn test_raw_occupied_entry_mut_into_key() {
    let mut map = HashMap::new();
    map.insert(10i32, String::from("ten"));
    map.insert(20, String::from("twenty"));
    map.insert(30, String::from("thirty"));

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&10), Some(&String::from("ten")));

    let hash = compute_hash(map.hasher(), &20);
    let key_ref: &mut i32 = match map.raw_entry_mut().from_hash(hash, |q| *q == 20) {
        RawEntryMut::Occupied(o) => o.into_key(),
        RawEntryMut::Vacant(_) => unreachable!(),
    };

    assert_eq!(*key_ref, 20);

    *key_ref = 25;


    assert_eq!(map.len(), 3);

    let has_25 = map.iter().any(|(k, v)| *k == 25 && v == "twenty");
    assert!(has_25);
    assert_eq!(map.get(&10), Some(&String::from("ten")));
    assert_eq!(map.get(&30), Some(&String::from("thirty")));
}

#[test]
fn test_raw_occupied_entry_mut_into_mut() {
    let mut map = HashMap::new();
    map.insert("x", vec![1, 2, 3]);
    map.insert("y", vec![4, 5, 6]);
    map.insert("z", vec![7, 8, 9]);

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("x"), Some(&vec![1, 2, 3]));

    let hash = compute_hash(map.hasher(), &"x");
    let val_ref: &mut Vec<i32> = match map.raw_entry_mut().from_hash(hash, |q| *q == "x") {
        RawEntryMut::Occupied(o) => o.into_mut(),
        RawEntryMut::Vacant(_) => unreachable!(),
    };

    assert_eq!(val_ref, &vec![1, 2, 3]);
    val_ref.push(10);
    val_ref.push(11);

    assert_eq!(map.get("x"), Some(&vec![1, 2, 3, 10, 11]));
    assert_eq!(map.get("y"), Some(&vec![4, 5, 6]));
    assert_eq!(map.get("z"), Some(&vec![7, 8, 9]));
    assert_eq!(map.len(), 3);
}

#[test]
fn test_raw_occupied_entry_mut_get_key_value_mut() {
    let mut map = HashMap::new();
    map.insert(String::from("one"), 1u64);
    map.insert(String::from("two"), 2);
    map.insert(String::from("three"), 3);
    map.insert(String::from("four"), 4);

    assert_eq!(map.len(), 4);

    let hash = compute_hash(map.hasher(), "three");
    match map.raw_entry_mut().from_hash(hash, |q| q == "three") {
        RawEntryMut::Occupied(mut o) => {
            let (k, v) = o.get_key_value_mut();
            assert_eq!(k.as_str(), "three");
            assert_eq!(*v, 3);
            *v = 33;
            k.push_str("_updated");
        }
        RawEntryMut::Vacant(_) => unreachable!(),
    }

    assert_eq!(map.len(), 4);

    let found = map.iter().any(|(k, v)| k == "three_updated" && *v == 33);
    assert!(found);
    assert_eq!(map.get("one"), Some(&1));
    assert_eq!(map.get("two"), Some(&2));
    assert_eq!(map.get("four"), Some(&4));
}

#[test]
fn test_raw_occupied_entry_mut_into_key_value() {
    let mut map = HashMap::new();
    map.insert(100i64, 1000i64);
    map.insert(200, 2000);
    map.insert(300, 3000);

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&200), Some(&2000));

    let hash = compute_hash(map.hasher(), &200);
    let (k_ref, v_ref): (&mut i64, &mut i64) =
        match map.raw_entry_mut().from_hash(hash, |q| *q == 200) {
            RawEntryMut::Occupied(o) => o.into_key_value(),
            RawEntryMut::Vacant(_) => unreachable!(),
        };

    assert_eq!(*k_ref, 200);
    assert_eq!(*v_ref, 2000);

    *k_ref = 250;
    *v_ref = 2500;

    assert_eq!(map.len(), 3);
    let found = map.iter().any(|(k, v)| *k == 250 && *v == 2500);
    assert!(found);
    assert_eq!(map.get(&100), Some(&1000));
    assert_eq!(map.get(&300), Some(&3000));
}

#[test]
fn test_raw_occupied_entry_mut_insert_key() {
    let mut map = HashMap::new();
    map.insert(String::from("hello"), 42u32);
    map.insert(String::from("world"), 99);
    map.insert(String::from("foo"), 7);

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("hello"), Some(&42));

    let hash = compute_hash(map.hasher(), "hello");
    match map.raw_entry_mut().from_hash(hash, |q| q == "hello") {
        RawEntryMut::Occupied(mut o) => {
            assert_eq!(o.get(), &42);
            let old_key = o.insert_key(String::from("hello_replaced"));
            assert_eq!(old_key, "hello");

            assert_eq!(o.get(), &42);
        }
        RawEntryMut::Vacant(_) => unreachable!(),
    }

    assert_eq!(map.len(), 3);

    let found = map.iter().any(|(k, v)| k == "hello_replaced" && *v == 42);
    assert!(found);
    assert_eq!(map.get("world"), Some(&99));
    assert_eq!(map.get("foo"), Some(&7));
}

#[test]
fn test_raw_occupied_entry_mut_replace_entry_with_some() {
    let mut map = HashMap::new();
    map.insert(1u32, String::from("one"));
    map.insert(2, String::from("two"));
    map.insert(3, String::from("three"));
    map.insert(4, String::from("four"));

    assert_eq!(map.len(), 4);
    assert_eq!(map.get(&2), Some(&String::from("two")));

    let hash = compute_hash(map.hasher(), &2);
    let result = match map.raw_entry_mut().from_hash(hash, |q| *q == 2) {
        RawEntryMut::Occupied(o) => o.replace_entry_with(|k, v| {
            assert_eq!(*k, 2);
            assert_eq!(v, "two");
            Some(String::from("TWO_REPLACED"))
        }),
        RawEntryMut::Vacant(_) => unreachable!(),
    };


    match result {
        RawEntryMut::Occupied(o) => {
            assert_eq!(*o.key(), 2);
            assert_eq!(o.get(), &String::from("TWO_REPLACED"));
        }
        RawEntryMut::Vacant(_) => panic!("Expected Occupied after replace_entry_with returning Some"),
    }

    assert_eq!(map.len(), 4);
    assert_eq!(map.get(&2), Some(&String::from("TWO_REPLACED")));
    assert_eq!(map.get(&1), Some(&String::from("one")));
    assert_eq!(map.get(&3), Some(&String::from("three")));
}

#[test]
fn test_raw_occupied_entry_mut_replace_entry_with_none() {
    let mut map = HashMap::new();
    map.insert(10u32, String::from("ten"));
    map.insert(20, String::from("twenty"));
    map.insert(30, String::from("thirty"));

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&20), Some(&String::from("twenty")));

    let hash = compute_hash(map.hasher(), &20);
    let result = match map.raw_entry_mut().from_hash(hash, |q| *q == 20) {
        RawEntryMut::Occupied(o) => o.replace_entry_with(|k, v| {
            assert_eq!(*k, 20);
            assert_eq!(v, "twenty");
            None
        }),
        RawEntryMut::Vacant(_) => unreachable!(),
    };


    match result {
        RawEntryMut::Vacant(_) => {}
        RawEntryMut::Occupied(_) => panic!("Expected Vacant after replace_entry_with returning None"),
    }

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&20), None);
    assert_eq!(map.get(&10), Some(&String::from("ten")));
    assert_eq!(map.get(&30), Some(&String::from("thirty")));
}

#[test]
fn test_raw_occupied_entry_mut_combined_workflow() {

    let mut map: HashMap<String, Vec<i32>> = HashMap::new();
    map.insert(String::from("data"), vec![1, 2, 3]);
    map.insert(String::from("info"), vec![10, 20]);
    map.insert(String::from("meta"), vec![100]);

    assert_eq!(map.len(), 3);


    let hash = compute_hash(map.hasher(), "data");
    match map.raw_entry_mut().from_hash(hash, |q| q == "data") {
        RawEntryMut::Occupied(mut o) => {
            let (k, v) = o.get_key_value_mut();
            assert_eq!(k, "data");
            assert_eq!(v, &vec![1, 2, 3]);
            v.push(4);
            v.push(5);
        }
        RawEntryMut::Vacant(_) => unreachable!(),
    }

    let data_val = map.get("data").unwrap();
    assert_eq!(data_val, &vec![1, 2, 3, 4, 5]);


    let hash = compute_hash(map.hasher(), "info");
    match map.raw_entry_mut().from_hash(hash, |q| q == "info") {
        RawEntryMut::Occupied(mut o) => {
            let old = o.insert_key(String::from("information"));
            assert_eq!(old, "info");
            assert_eq!(o.get(), &vec![10, 20]);
        }
        RawEntryMut::Vacant(_) => unreachable!(),
    }

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("info"), None);
    let found_info = map.iter().any(|(k, v)| k == "information" && *v == vec![10, 20]);
    assert!(found_info);


    let hash = compute_hash(map.hasher(), "meta");
    match map.raw_entry_mut().from_hash(hash, |q| q == "meta") {
        RawEntryMut::Occupied(o) => {
            let result = o.replace_entry_with(|_k, v| {
                if v.len() < 2 {
                    None
                } else {
                    Some(v)
                }
            });
            match result {
                RawEntryMut::Vacant(_) => {}
                RawEntryMut::Occupied(_) => panic!("Should have been removed"),
            }
        }
        RawEntryMut::Vacant(_) => unreachable!(),
    }

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("meta"), None);
}

#[test]
fn test_raw_occupied_entry_mut_key_mut_with_numeric_keys() {
    let mut map = HashMap::new();
    for i in 0..50u64 {
        map.insert(i, i * i);
    }

    assert_eq!(map.len(), 50);
    assert_eq!(map.get(&25), Some(&625));

    let hash = compute_hash(map.hasher(), &25u64);
    match map.raw_entry_mut().from_hash(hash, |q| *q == 25) {
        RawEntryMut::Occupied(mut o) => {
            let k = o.key_mut();
            assert_eq!(*k, 25);

            *k = 2500;

            assert_eq!(*o.get(), 625);
        }
        RawEntryMut::Vacant(_) => unreachable!(),
    }

    assert_eq!(map.len(), 50);

    let found = map.iter().any(|(k, v)| *k == 2500 && *v == 625);
    assert!(found);

    assert_eq!(map.get(&0), Some(&0));
    assert_eq!(map.get(&49), Some(&2401));
    assert_eq!(map.get(&1), Some(&1));
}

#[test]
fn test_raw_occupied_entry_mut_into_mut_large_value() {
    let mut map = HashMap::new();
    map.insert("small", vec![1u8; 10]);
    map.insert("medium", vec![2u8; 100]);
    map.insert("large", vec![3u8; 1000]);

    assert_eq!(map.len(), 3);
    assert_eq!(map.get("large").unwrap().len(), 1000);

    let hash = compute_hash(map.hasher(), &"large");
    let val: &mut Vec<u8> = match map.raw_entry_mut().from_hash(hash, |q| *q == "large") {
        RawEntryMut::Occupied(o) => o.into_mut(),
        RawEntryMut::Vacant(_) => unreachable!(),
    };

    assert_eq!(val.len(), 1000);
    assert_eq!(val[0], 3);
    val.truncate(5);
    val.extend_from_slice(&[99, 98, 97]);

    assert_eq!(map.get("large").unwrap().len(), 8);
    assert_eq!(&map.get("large").unwrap()[5..], &[99, 98, 97]);
    assert_eq!(map.get("small").unwrap().len(), 10);
    assert_eq!(map.get("medium").unwrap().len(), 100);
}