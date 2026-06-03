use slotmap::DenseSlotMap;

#[test]
fn test_dense_basic_lifecycle() {
    let mut sm: DenseSlotMap<slotmap::DefaultKey, i32> = DenseSlotMap::with_key();
    assert!(sm.is_empty());
    assert_eq!(sm.len(), 0);

    sm.reserve(100);
    assert!(sm.capacity() >= 100);
    let cap_before = sm.capacity();

    let k1 = sm.insert(10);
    let k2 = sm.insert(20);
    let k3 = sm.insert(30);

    assert!(!sm.is_empty());
    assert_eq!(sm.len(), 3);
    assert_eq!(sm.capacity(), cap_before);
    assert_eq!(sm[k1], 10);
    assert_eq!(sm[k2], 20);
    assert_eq!(sm[k3], 30);

    sm.clear();
    assert!(sm.is_empty());
    assert_eq!(sm.len(), 0);
    assert_eq!(sm.get(k1), None);
}

#[test]
fn test_dense_insert_with_key_and_try() {
    let mut sm: DenseSlotMap<slotmap::DefaultKey, (slotmap::DefaultKey, i32)> =
        DenseSlotMap::with_key();

    let k = sm.insert_with_key(|key| (key, 42));
    assert_eq!(sm[k].0, k);
    assert_eq!(sm[k].1, 42);

    let res: Result<slotmap::DefaultKey, &'static str> =
        sm.try_insert_with_key(|key| Ok((key, 99)));
    assert!(res.is_ok());
    let k2 = res.unwrap();
    assert_eq!(sm[k2].0, k2);
    assert_eq!(sm[k2].1, 99);
    assert_eq!(sm.len(), 2);

    let res_err: Result<slotmap::DefaultKey, &'static str> =
        sm.try_insert_with_key(|_| Err("nope"));
    assert!(res_err.is_err());
    assert_eq!(res_err.unwrap_err(), "nope");
    assert_eq!(sm.len(), 2);
}

#[test]
fn test_dense_retain_and_iter_mut() {
    let mut sm: DenseSlotMap<slotmap::DefaultKey, i32> = DenseSlotMap::new();
    let keys: Vec<_> = (0..10).map(|i| sm.insert(i)).collect();
    assert_eq!(sm.len(), 10);

    for (_, v) in sm.iter_mut() {
        *v *= 10;
    }
    assert_eq!(sm[keys[3]], 30);
    assert_eq!(sm[keys[7]], 70);

    for v in sm.values_mut() {
        *v += 1;
    }
    assert_eq!(sm[keys[0]], 1);
    assert_eq!(sm[keys[5]], 51);

    sm.retain(|_k, v| *v >= 30);
    assert_eq!(sm.len(), 7);
    assert!(sm.get(keys[0]).is_none());
    assert!(sm.get(keys[1]).is_none());
    assert_eq!(sm.get(keys[3]).copied(), Some(31));
    assert_eq!(sm.get(keys[9]).copied(), Some(91));
}

#[test]
fn test_dense_get_unchecked_and_disjoint() {
    let mut sm: DenseSlotMap<slotmap::DefaultKey, i32> = DenseSlotMap::new();
    let k1 = sm.insert(100);
    let k2 = sm.insert(200);
    let k3 = sm.insert(300);

    unsafe {
        let v = sm.get_unchecked_mut(k2);
        *v = 250;
    }
    assert_eq!(sm[k2], 250);

    let disj = sm.get_disjoint_mut([k1, k3]);
    assert!(disj.is_some());
    let arr = disj.unwrap();
    *arr[0] = 111;
    *arr[1] = 333;
    assert_eq!(sm[k1], 111);
    assert_eq!(sm[k3], 333);
    assert_eq!(sm[k2], 250);


    let dup = sm.get_disjoint_mut([k1, k1]);
    assert!(dup.is_none());

    unsafe {
        let arr2 = sm.get_disjoint_unchecked_mut([k1, k2, k3]);
        *arr2[0] = 1;
        *arr2[1] = 2;
        *arr2[2] = 3;
    }
    assert_eq!(sm[k1], 1);
    assert_eq!(sm[k2], 2);
    assert_eq!(sm[k3], 3);
}

#[test]
fn test_dense_capacity_reserve() {
    let mut sm: DenseSlotMap<slotmap::DefaultKey, String> = DenseSlotMap::with_key();
    assert_eq!(sm.len(), 0);
    sm.reserve(50);
    let cap = sm.capacity();
    assert!(cap >= 50);

    for i in 0..30 {
        sm.insert(format!("v{}", i));
    }
    assert_eq!(sm.len(), 30);
    assert!(sm.capacity() >= 50);
    assert!(!sm.is_empty());

    sm.clear();
    assert!(sm.is_empty());
    assert!(sm.capacity() >= 50);
}