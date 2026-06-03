use indexmap::map::raw_entry_v1::RawEntryMut;
use indexmap::map::RawEntryApiV1;
use indexmap::IndexMap;
use std::hash::{BuildHasher, Hash, Hasher};

fn make_hash<K: Hash, S: BuildHasher>(map: &IndexMap<K, i32, S>, key: &K) -> u64 {
    let mut state = map.hasher().build_hasher();
    key.hash(&mut state);
    state.finish()
}

#[test]
fn raw_entry_mut_from_hash_updates_existing_entry_and_preserves_index() {
    let mut map: IndexMap<String, i32> = IndexMap::new();

    assert_eq!(map.insert_full("alpha".to_string(), 10), (0, None));
    assert_eq!(map.insert_full("beta".to_string(), 20), (1, None));
    assert_eq!(map.insert_full("gamma".to_string(), 30), (2, None));
    assert_eq!(map.len(), 3);

    let beta = "beta".to_string();
    let beta_hash = make_hash(&map, &beta);

    match map
        .raw_entry_mut_v1()
        .from_hash(beta_hash, |key| key.as_str() == "beta")
    {
        RawEntryMut::Occupied(mut entry) => {
            assert_eq!(entry.index(), 1);
            assert_eq!(entry.key().as_str(), "beta");
            assert_eq!(*entry.get(), 20);

            let old = entry.insert(25);
            assert_eq!(old, 20);

            let (key, value) = entry.get_key_value_mut();
            assert_eq!(key.as_str(), "beta");
            *value += 5;
        }
        RawEntryMut::Vacant(_) => panic!("expected beta to be occupied"),
    }

    assert_eq!(map.get("beta"), Some(&30));
    assert_eq!(map.get_index(1), Some((&"beta".to_string(), &30)));
    assert_eq!(
        map.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>(),
        vec![("alpha", 10), ("beta", 30), ("gamma", 30)]
    );

    let alpha_hash = make_hash(&map, &"alpha".to_string());
    match map
        .raw_entry_mut_v1()
        .from_hash(alpha_hash, |key| key.starts_with('a'))
    {
        RawEntryMut::Occupied(entry) => {
            let value = entry.into_mut();
            *value *= 2;
        }
        RawEntryMut::Vacant(_) => panic!("expected alpha to be occupied"),
    }

    assert_eq!(map.get("alpha"), Some(&20));
    assert_eq!(map.get_index_of("alpha"), Some(0));
}

#[test]
fn raw_entry_mut_from_hash_inserts_when_predicate_finds_no_match() {
    let mut map: IndexMap<String, i32> = IndexMap::with_capacity(2);

    map.insert("red".to_string(), 1);
    map.insert("green".to_string(), 2);

    let blue = "blue".to_string();
    let blue_hash = make_hash(&map, &blue);

    match map
        .raw_entry_mut_v1()
        .from_hash(blue_hash, |key| key.as_str() == "blue")
    {
        RawEntryMut::Occupied(_) => panic!("blue should not exist yet"),
        RawEntryMut::Vacant(entry) => {
            assert_eq!(entry.index(), 2);

            let (key, value) = entry.insert_hashed_nocheck(blue_hash, blue, 3);
            assert_eq!(key.as_str(), "blue");
            assert_eq!(*value, 3);

            *value = 4;
        }
    }

    assert_eq!(map.len(), 3);
    assert!(map.contains_key("blue"));
    assert_eq!(map.get("blue"), Some(&4));
    assert_eq!(map.get_index_of("blue"), Some(2));

    let green = "green".to_string();
    let green_hash = make_hash(&map, &green);
    let len_before = map.len();

    match map
        .raw_entry_mut_v1()
        .from_hash(green_hash, |key| key.as_str() == "purple")
    {
        RawEntryMut::Occupied(_) => {
            panic!("predicate should reject every existing key, even when using a valid hash")
        }
        RawEntryMut::Vacant(entry) => {
            assert_eq!(entry.index(), len_before);
        }
    }

    assert_eq!(map.len(), len_before);
    assert_eq!(
        map.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>(),
        vec![("red", 1), ("green", 2), ("blue", 4)]
    );
}