use slotmap::secondary::Entry;
use slotmap::{SecondaryMap, SlotMap};

#[test]
fn test_secondary_basic_capacity_and_empty() {
    let mut sm: SlotMap<_, i32> = SlotMap::new();
    let mut sec: SecondaryMap<_, String> = SecondaryMap::new();

    assert!(sec.is_empty());
    assert_eq!(sec.len(), 0);
    let initial_cap = sec.capacity();
    assert_eq!(initial_cap, 0);

    sec.set_capacity(32);
    assert!(sec.capacity() >= 32);
    assert!(sec.is_empty());

    let k1 = sm.insert(10);
    let k2 = sm.insert(20);
    let k3 = sm.insert(30);

    sec.insert(k1, "one".to_string());
    sec.insert(k2, "two".to_string());
    sec.insert(k3, "three".to_string());

    assert!(!sec.is_empty());
    assert_eq!(sec.len(), 3);
    assert!(sec.capacity() >= 32);

    sec.clear();
    assert!(sec.is_empty());
    assert_eq!(sec.len(), 0);
    assert!(sec.capacity() >= 32);
}

#[test]
fn test_secondary_retain_and_iter_mut() {
    let mut sm: SlotMap<_, i32> = SlotMap::new();
    let mut sec: SecondaryMap<_, i32> = SecondaryMap::new();

    let keys: Vec<_> = (0..10).map(|i| sm.insert(i)).collect();
    for (i, k) in keys.iter().enumerate() {
        sec.insert(*k, i as i32 * 10);
    }
    assert_eq!(sec.len(), 10);


    for (_, v) in sec.iter_mut() {
        *v += 1;
    }
    assert_eq!(sec[keys[0]], 1);
    assert_eq!(sec[keys[5]], 51);


    for v in sec.values_mut() {
        *v *= 2;
    }
    assert_eq!(sec[keys[0]], 2);
    assert_eq!(sec[keys[5]], 102);


    sec.retain(|k, _v| keys.iter().position(|x| *x == k).unwrap() % 2 == 0);
    assert_eq!(sec.len(), 5);
    assert!(sec.contains_key(keys[0]));
    assert!(!sec.contains_key(keys[1]));
    assert!(sec.contains_key(keys[2]));
    assert!(!sec.contains_key(keys[3]));
}

#[test]
fn test_secondary_get_disjoint_and_unchecked() {
    let mut sm: SlotMap<_, i32> = SlotMap::new();
    let mut sec: SecondaryMap<_, i32> = SecondaryMap::new();

    let k1 = sm.insert(1);
    let k2 = sm.insert(2);
    let k3 = sm.insert(3);
    sec.insert(k1, 100);
    sec.insert(k2, 200);
    sec.insert(k3, 300);


    let arr = sec.get_disjoint_mut([k1, k2, k3]).expect("disjoint");
    *arr[0] += 1;
    *arr[1] += 2;
    *arr[2] += 3;
    assert_eq!(sec[k1], 101);
    assert_eq!(sec[k2], 202);
    assert_eq!(sec[k3], 303);


    let dup = sec.get_disjoint_mut([k1, k1]);
    assert!(dup.is_none());


    unsafe {
        let v = sec.get_unchecked_mut(k2);
        *v = 999;
    }
    assert_eq!(sec[k2], 999);

    unsafe {
        let arr = sec.get_disjoint_unchecked_mut([k1, k3]);
        *arr[0] = 7;
        *arr[1] = 8;
    }
    assert_eq!(sec[k1], 7);
    assert_eq!(sec[k3], 8);
}

#[test]
fn test_secondary_entry_api() {
    let mut sm: SlotMap<_, i32> = SlotMap::new();
    let mut sec: SecondaryMap<_, i32> = SecondaryMap::new();

    let k1 = sm.insert(1);
    let k2 = sm.insert(2);
    sec.insert(k1, 10);


    match sec.entry(k1).expect("valid key") {
        Entry::Occupied(o) => {
            assert_eq!(o.key(), k1);
        }
        Entry::Vacant(_) => panic!("expected occupied"),
    }


    match sec.entry(k2).expect("valid key") {
        Entry::Vacant(v) => {
            assert_eq!(v.key(), k2);
        }
        Entry::Occupied(_) => panic!("expected vacant"),
    }


    let val = sec.entry(k2).unwrap().or_insert(42);
    assert_eq!(*val, 42);
    assert_eq!(sec[k2], 42);


    let val = sec.entry(k1).unwrap().or_insert(999);
    assert_eq!(*val, 10);
    assert_eq!(sec[k1], 10);


    sm.remove(k1);








    let entry_result = sec.entry(k1);







    assert!(entry_result.is_some());
    assert_eq!(sec.len(), 2);
}