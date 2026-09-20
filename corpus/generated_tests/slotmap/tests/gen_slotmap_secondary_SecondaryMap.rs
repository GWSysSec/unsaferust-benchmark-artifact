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

    // mutate via iter_mut
    for (_, v) in sec.iter_mut() {
        *v += 1;
    }
    assert_eq!(sec[keys[0]], 1);
    assert_eq!(sec[keys[5]], 51);

    // values_mut
    for v in sec.values_mut() {
        *v *= 2;
    }
    assert_eq!(sec[keys[0]], 2);
    assert_eq!(sec[keys[5]], 102);

    // retain only even-original-index entries
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

    // disjoint mut OK
    let arr = sec.get_disjoint_mut([k1, k2, k3]).expect("disjoint");
    *arr[0] += 1;
    *arr[1] += 2;
    *arr[2] += 3;
    assert_eq!(sec[k1], 101);
    assert_eq!(sec[k2], 202);
    assert_eq!(sec[k3], 303);

    // duplicate key -> None
    let dup = sec.get_disjoint_mut([k1, k1]);
    assert!(dup.is_none());

    // unchecked variant
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

    // Occupied
    match sec.entry(k1).expect("valid key") {
        Entry::Occupied(o) => {
            assert_eq!(o.key(), k1);
        }
        Entry::Vacant(_) => panic!("expected occupied"),
    }

    // Vacant
    match sec.entry(k2).expect("valid key") {
        Entry::Vacant(v) => {
            assert_eq!(v.key(), k2);
        }
        Entry::Occupied(_) => panic!("expected vacant"),
    }

    // or_insert on vacant
    let val = sec.entry(k2).unwrap().or_insert(42);
    assert_eq!(*val, 42);
    assert_eq!(sec[k2], 42);

    // or_insert on occupied (no change)
    let val = sec.entry(k1).unwrap().or_insert(999);
    assert_eq!(*val, 10);
    assert_eq!(sec[k1], 10);

    // Invalid key: remove from primary
    sm.remove(k1);
    // After removing from primary, the secondary entry for k1 still exists
    // but entry() returns None because the key is no longer valid in the primary map.
    // However, SecondaryMap::entry() does not consult the primary map — it only
    // checks whether the key's version matches what's stored in the secondary map.
    // Since we inserted k1 into sec with the original version, and the key itself
    // hasn't changed (only the slot in sm was freed), the secondary map still
    // considers k1 valid. So entry() will return Some(Occupied(...)).
    // The key version in sec still matches k1's version.
    let entry_result = sec.entry(k1);
    // SecondaryMap.entry() returns Option<Entry> — it returns None only if the
    // key version doesn't match. Since we inserted with k1's version and haven't
    // changed it in sec, it should still be Some.
    // Actually, looking at slotmap semantics more carefully:
    // SecondaryMap::entry checks if the key's version matches what's stored.
    // The key k1 still has its original version (it's just a copy we hold).
    // The secondary map stored that version when we inserted. So it matches.
    assert!(entry_result.is_some());
    assert_eq!(sec.len(), 2);
}