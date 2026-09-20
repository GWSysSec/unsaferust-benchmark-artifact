use indexmap::map::raw_entry_v1::RawEntryMut;
use indexmap::map::RawEntryApiV1;
use indexmap::IndexMap;
use std::hash::{BuildHasher, Hash, Hasher};

fn make_hash<K: Hash, V, S: BuildHasher>(map: &IndexMap<K, V, S>, key: &K) -> u64 {
    let mut state = map.hasher().build_hasher();
    key.hash(&mut state);
    state.finish()
}

#[test]
fn vacant_raw_entry_insert_hashed_nocheck_inserts_with_precomputed_hash() {
    let mut map: IndexMap<String, i32> = IndexMap::new();

    assert_eq!(map.insert_full("alpha".to_string(), 10), (0, None));
    assert_eq!(map.insert_full("beta".to_string(), 20), (1, None));
    assert_eq!(map.insert_full("gamma".to_string(), 30), (2, None));
    assert_eq!(map.len(), 3);

    let delta = "delta".to_string();
    let delta_hash = make_hash(&map, &delta);

    match map.raw_entry_mut_v1().from_key(&delta) {
        RawEntryMut::Occupied(_) => panic!("delta should not be present before insertion"),
        RawEntryMut::Vacant(entry) => {
            assert_eq!(entry.index(), 3);

            let (inserted_key, inserted_value) =
                entry.insert_hashed_nocheck(delta_hash, delta.clone(), 40);

            assert_eq!(inserted_key.as_str(), "delta");
            assert_eq!(*inserted_value, 40);

            *inserted_value += 2;
        }
    }

    assert_eq!(map.len(), 4);
    assert!(map.contains_key("delta"));
    assert_eq!(map.get("delta"), Some(&42));
    assert_eq!(
        map.get_full("delta").map(|(index, key, value)| (index, key.as_str(), *value)),
        Some((3, "delta", 42))
    );

    let delta_hash_after_insert = make_hash(&map, &delta);
    assert_eq!(delta_hash_after_insert, delta_hash);

    match map
        .raw_entry_mut_v1()
        .from_key_hashed_nocheck(delta_hash_after_insert, "delta")
    {
        RawEntryMut::Occupied(entry) => {
            assert_eq!(entry.index(), 3);
            assert_eq!(entry.key().as_str(), "delta");
            assert_eq!(*entry.get(), 42);
        }
        RawEntryMut::Vacant(_) => panic!("delta should be found with the same precomputed hash"),
    }

    assert_eq!(map.get_index(0).map(|(key, value)| (key.as_str(), *value)), Some(("alpha", 10)));
    assert_eq!(map.get_index(1).map(|(key, value)| (key.as_str(), *value)), Some(("beta", 20)));
    assert_eq!(map.get_index(2).map(|(key, value)| (key.as_str(), *value)), Some(("gamma", 30)));
    assert_eq!(map.get_index(3).map(|(key, value)| (key.as_str(), *value)), Some(("delta", 42)));
}

#[test]
fn insert_hashed_nocheck_can_insert_into_empty_map_and_value_remains_mutable() {
    let mut map: IndexMap<String, Vec<i32>> = IndexMap::new();

    assert!(map.is_empty());

    let key = "numbers".to_string();
    let hash = make_hash(&map, &key);

    match map.raw_entry_mut_v1().from_hash(hash, |existing| existing == &key) {
        RawEntryMut::Occupied(_) => panic!("empty map cannot have an occupied raw entry"),
        RawEntryMut::Vacant(entry) => {
            assert_eq!(entry.index(), 0);

            let (inserted_key, inserted_value) =
                entry.insert_hashed_nocheck(hash, key.clone(), vec![1, 2]);

            assert_eq!(inserted_key, "numbers");
            inserted_value.push(3);
            assert_eq!(inserted_value.as_slice(), &[1, 2, 3]);
        }
    }

    assert_eq!(map.len(), 1);
    assert_eq!(map.get("numbers").map(Vec::as_slice), Some(&[1, 2, 3][..]));

    let removed = map.shift_remove("numbers");
    assert_eq!(removed, Some(vec![1, 2, 3]));
    assert!(map.is_empty());
}