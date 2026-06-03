use hashbrown::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};

fn compute_hash<K: Hash + ?Sized, S: BuildHasher>(hash_builder: &S, key: &K) -> u64 {
    let mut state = hash_builder.build_hasher();
    key.hash(&mut state);
    state.finish()
}

#[test]
fn test_hashmap_allocator() {
    let map: HashMap<i32, i32> = HashMap::new();
    let _alloc = map.allocator();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());

    let mut map2: HashMap<String, Vec<u8>> = HashMap::new();
    map2.insert("hello".to_string(), vec![1, 2, 3]);
    map2.insert("world".to_string(), vec![4, 5, 6]);
    let _alloc2 = map2.allocator();
    assert_eq!(map2.len(), 2);
    assert_eq!(map2["hello"], vec![1, 2, 3]);
    assert_eq!(map2["world"], vec![4, 5, 6]);


    map2.insert("foo".to_string(), vec![7, 8, 9]);
    let _alloc3 = map2.allocator();
    assert_eq!(map2.len(), 3);
    assert!(map2.contains_key("foo"));
}

#[test]
fn test_hashmap_retain() {
    let mut map: HashMap<i32, i32> = HashMap::new();
    for i in 0..20 {
        map.insert(i, i * 10);
    }
    assert_eq!(map.len(), 20);


    map.retain(|&k, _| k % 2 == 0);
    assert_eq!(map.len(), 10);


    for (&k, &v) in map.iter() {
        assert_eq!(k % 2, 0);
        assert_eq!(v, k * 10);
    }


    assert_eq!(map.get(&0), Some(&0));
    assert_eq!(map.get(&2), Some(&20));
    assert_eq!(map.get(&4), Some(&40));
    assert_eq!(map.get(&1), None);
    assert_eq!(map.get(&3), None);


    map.retain(|_, v| {
        *v += 1;
        true
    });
    assert_eq!(map.len(), 10);
    assert_eq!(map.get(&0), Some(&1));
    assert_eq!(map.get(&2), Some(&21));


    map.retain(|_, _| false);
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
}

#[test]
fn test_hashmap_extract_if() {
    let mut map: HashMap<i32, String> = HashMap::new();
    for i in 0..15 {
        map.insert(i, format!("val_{}", i));
    }
    assert_eq!(map.len(), 15);


    let extracted: Vec<(i32, String)> = map.extract_if(|&k, _| k < 5).collect();
    assert_eq!(extracted.len(), 5);
    assert_eq!(map.len(), 10);


    for (k, v) in &extracted {
        assert!(*k < 5);
        assert_eq!(*v, format!("val_{}", k));
    }


    for (&k, v) in map.iter() {
        assert!(k >= 5);
        assert_eq!(*v, format!("val_{}", k));
    }

    assert!(map.contains_key(&5));
    assert!(map.contains_key(&14));
    assert!(!map.contains_key(&0));
    assert!(!map.contains_key(&4));


    let remaining: Vec<_> = map.extract_if(|_, _| true).collect();
    assert_eq!(remaining.len(), 10);
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
}

#[test]
fn test_hashmap_clear() {
    let mut map: HashMap<&str, i64> = HashMap::new();
    map.insert("alpha", 100);
    map.insert("beta", 200);
    map.insert("gamma", 300);
    map.insert("delta", 400);
    assert_eq!(map.len(), 4);
    assert!(!map.is_empty());

    let cap_before = map.capacity();
    map.clear();

    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
    assert!(map.capacity() >= cap_before);
    assert_eq!(map.get("alpha"), None);
    assert_eq!(map.get("beta"), None);
    assert_eq!(map.get("gamma"), None);
    assert_eq!(map.get("delta"), None);


    map.insert("epsilon", 500);
    assert_eq!(map.len(), 1);
    assert_eq!(map["epsilon"], 500);
}

#[test]
fn test_hashmap_shrink_to() {
    let mut map: HashMap<i32, i32> = HashMap::with_capacity(100);
    assert!(map.capacity() >= 100);

    for i in 0..10 {
        map.insert(i, i * 2);
    }
    assert_eq!(map.len(), 10);

    let cap_before = map.capacity();
    assert!(cap_before >= 100);


    map.shrink_to(20);
    let cap_after = map.capacity();
    assert!(cap_after >= 10);
    assert!(cap_after <= cap_before);


    for i in 0..10 {
        assert_eq!(map.get(&i), Some(&(i * 2)));
    }
    assert_eq!(map.len(), 10);


    map.shrink_to(0);
    assert!(map.capacity() >= 10);
    assert_eq!(map.len(), 10);
    assert_eq!(map.get(&5), Some(&10));
    assert_eq!(map.get(&9), Some(&18));
}

#[test]
fn test_hashmap_get_key_value_mut() {
    let mut map: HashMap<String, Vec<i32>> = HashMap::new();
    map.insert("first".to_string(), vec![1, 2, 3]);
    map.insert("second".to_string(), vec![4, 5, 6]);
    map.insert("third".to_string(), vec![7, 8, 9]);


    let result = map.get_key_value_mut("first");
    assert!(result.is_some());
    let (key, value) = result.unwrap();
    assert_eq!(key, "first");
    assert_eq!(value, &vec![1, 2, 3]);


    value.push(10);
    assert_eq!(map["first"], vec![1, 2, 3, 10]);


    let result2 = map.get_key_value_mut("second");
    assert!(result2.is_some());
    let (key2, value2) = result2.unwrap();
    assert_eq!(key2, "second");
    value2.clear();
    assert_eq!(map["second"], vec![]);


    let result3 = map.get_key_value_mut("nonexistent");
    assert!(result3.is_none());


    assert_eq!(map.len(), 3);
    assert_eq!(map["third"], vec![7, 8, 9]);
}

#[test]
fn test_hashmap_get_many_unchecked_mut() {
    let mut map: HashMap<&str, i32> = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);
    map.insert("d", 4);
    map.insert("e", 5);

    assert_eq!(map.len(), 5);



    let results = unsafe { map.get_many_unchecked_mut([&"a", &"c", &"e"]) };
    let [r0, r1, r2] = results;
    assert!(r0.is_some());
    assert!(r1.is_some());
    assert!(r2.is_some());
    assert_eq!(*r0.unwrap(), 1);
    assert_eq!(*r1.unwrap(), 3);
    assert_eq!(*r2.unwrap(), 5);



    let results = unsafe { map.get_many_unchecked_mut([&"a", &"b"]) };
    let [ra, rb] = results;
    *ra.unwrap() = 100;
    *rb.unwrap() = 200;
    assert_eq!(map["a"], 100);
    assert_eq!(map["b"], 200);



    let results = unsafe { map.get_many_unchecked_mut([&"a", &"z"]) };
    let [ra2, rz] = results;
    assert!(ra2.is_some());
    assert!(rz.is_none());
    assert_eq!(*ra2.unwrap(), 100);
}

#[test]
fn test_hashmap_get_many_key_value_unchecked_mut() {
    let mut map: HashMap<String, Vec<u8>> = HashMap::new();
    map.insert("x".to_string(), vec![10, 20]);
    map.insert("y".to_string(), vec![30, 40]);
    map.insert("z".to_string(), vec![50, 60]);

    assert_eq!(map.len(), 3);


    let results = unsafe { map.get_many_key_value_unchecked_mut([&"x".to_string(), &"z".to_string()]) };
    let [r0, r1] = results;
    assert!(r0.is_some());
    assert!(r1.is_some());

    let (k0, v0) = r0.unwrap();
    let (k1, v1) = r1.unwrap();
    assert_eq!(k0.as_str(), "x");
    assert_eq!(k1.as_str(), "z");
    assert_eq!(v0, &vec![10, 20]);
    assert_eq!(v1, &vec![50, 60]);



    let results = unsafe { map.get_many_key_value_unchecked_mut([&"x".to_string(), &"y".to_string()]) };
    let [rx, ry] = results;
    rx.unwrap().1.push(99);
    ry.unwrap().1.push(88);
    assert_eq!(map["x"], vec![10, 20, 99]);
    assert_eq!(map["y"], vec![30, 40, 88]);



    let results = unsafe { map.get_many_key_value_unchecked_mut([&"w".to_string(), &"z".to_string()]) };
    let [rw, rz] = results;
    assert!(rw.is_none());
    assert!(rz.is_some());
    let (k, v) = rz.unwrap();
    assert_eq!(k.as_str(), "z");
    assert_eq!(v, &vec![50, 60]);
}

#[test]
fn test_hashmap_try_insert() {
    let mut map: HashMap<i32, &str> = HashMap::new();


    let result = map.try_insert(1, "one");
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), "one");
    assert_eq!(map.len(), 1);


    let result = map.try_insert(2, "two");
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), "two");
    assert_eq!(map.len(), 2);


    let result = map.try_insert(1, "uno");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(*err.entry.key(), 1);
    assert_eq!(err.value, "uno");
    assert_eq!(map.len(), 2);


    assert_eq!(map[&1], "one");
    assert_eq!(map[&2], "two");


    let result = map.try_insert(3, "three");
    assert!(result.is_ok());
    assert_eq!(map.len(), 3);
    assert_eq!(map[&3], "three");
}

#[test]
fn test_hashmap_combined_workflow() {

    let mut map: HashMap<i32, String> = HashMap::with_capacity(50);
    let _alloc = map.allocator();


    for i in 0..30 {
        map.insert(i, format!("item_{}", i));
    }
    assert_eq!(map.len(), 30);


    let err = map.try_insert(5, "new_five".to_string());
    assert!(err.is_err());
    assert_eq!(map[&5], "item_5");


    let ok = map.try_insert(100, "item_100".to_string());
    assert!(ok.is_ok());
    assert_eq!(map.len(), 31);


    map.retain(|&k, _| k % 3 == 0);

    assert_eq!(map.len(), 10);
    assert!(map.contains_key(&0));
    assert!(map.contains_key(&27));
    assert!(!map.contains_key(&100));
    assert!(!map.contains_key(&1));


    let extracted: Vec<_> = map.extract_if(|&k, _| k > 15).collect();
    assert_eq!(extracted.len(), 4);
    assert_eq!(map.len(), 6);


    let kv = map.get_key_value_mut(&9);
    assert!(kv.is_some());
    let (k, v) = kv.unwrap();
    assert_eq!(*k, 9);
    v.push_str("_modified");
    assert_eq!(map[&9], "item_9_modified");


    map.shrink_to(6);
    assert!(map.capacity() >= 6);
    assert_eq!(map.len(), 6);


    map.clear();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
    assert_eq!(map.get(&0), None);
}

#[test]
fn test_extract_if_partial_consumption() {
    let mut map: HashMap<u32, u32> = HashMap::new();
    for i in 0..20 {
        map.insert(i, i * i);
    }
    assert_eq!(map.len(), 20);


    let mut extractor = map.extract_if(|&k, _| k >= 10);
    let first = extractor.next();
    assert!(first.is_some());
    let (k, v) = first.unwrap();
    assert!(k >= 10);
    assert_eq!(v, k * k);

    let second = extractor.next();
    assert!(second.is_some());


    drop(extractor);



    assert!(map.len() <= 18);
    assert!(map.len() >= 8);


    for (&k, &v) in map.iter() {
        assert_eq!(v, k * k);
    }
}

#[test]
fn test_retain_with_large_map() {
    let mut map: HashMap<u64, u64> = HashMap::new();
    for i in 0..1000 {
        map.insert(i, i.wrapping_mul(7));
    }
    assert_eq!(map.len(), 1000);


    map.retain(|&k, _| k % 2 != 0 && k % 3 != 0 && k % 5 != 0 && k > 0);


    for (&k, &v) in map.iter() {
        assert!(k % 2 != 0);
        assert!(k % 3 != 0);
        assert!(k % 5 != 0);
        assert!(k > 0);
        assert_eq!(v, k.wrapping_mul(7));
    }

    assert!(map.len() > 0);
    assert!(map.len() < 1000);

    assert!(map.len() > 200);
    assert!(map.len() < 300);
}

#[test]
fn test_shrink_to_boundary_conditions() {
    let mut map: HashMap<i32, i32> = HashMap::with_capacity(256);
    assert!(map.capacity() >= 256);


    map.shrink_to(0);

    assert_eq!(map.len(), 0);


    for i in 0..8 {
        map.insert(i, i);
    }
    assert_eq!(map.len(), 8);

    map.shrink_to(8);
    assert!(map.capacity() >= 8);
    assert_eq!(map.len(), 8);


    for i in 0..8 {
        assert_eq!(map.get(&i), Some(&i));
    }


    let cap = map.capacity();
    map.shrink_to(1000);
    assert_eq!(map.capacity(), cap);
    assert_eq!(map.len(), 8);
}

#[test]
fn test_try_insert_error_details() {
    let mut map: HashMap<String, i32> = HashMap::new();
    map.insert("key1".to_string(), 42);
    map.insert("key2".to_string(), 84);


    let r = map.try_insert("key3".to_string(), 126);
    assert!(r.is_ok());
    assert_eq!(*r.unwrap(), 126);


    let r = map.try_insert("key1".to_string(), 999);
    assert!(r.is_err());
    let err = r.unwrap_err();
    assert_eq!(err.value, 999);
    assert_eq!(*err.entry.key(), "key1");

    assert_eq!(map["key1"], 42);
    assert_eq!(map.len(), 3);


    let r2 = map.try_insert("key2".to_string(), 0);
    assert!(r2.is_err());
    let err2 = r2.unwrap_err();
    let debug_str = format!("{:?}", err2);
    assert!(!debug_str.is_empty());
}