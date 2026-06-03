use indexmap::map::raw_entry_v1::RawEntryMut;
use indexmap::map::RawEntryApiV1;
use indexmap::IndexMap;
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Clone, Debug)]
struct TaggedKey {
    id: u32,
    label: String,
}

impl TaggedKey {
    fn new(id: u32, label: &str) -> Self {
        Self {
            id,
            label: label.to_string(),
        }
    }
}

impl PartialEq for TaggedKey {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TaggedKey {}

impl Hash for TaggedKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn make_hash<K: Hash, V, S: BuildHasher>(map: &IndexMap<K, V, S>, key: &K) -> u64 {
    let mut state = map.hasher().build_hasher();
    key.hash(&mut state);
    state.finish()
}

fn snapshot(map: &IndexMap<TaggedKey, Vec<i32>>) -> Vec<(u32, String, Vec<i32>)> {
    map.iter()
        .map(|(key, value)| (key.id, key.label.clone(), value.clone()))
        .collect()
}

#[test]
fn raw_immutable_lookup_by_hash_reports_key_value_and_index() {
    let mut map: IndexMap<TaggedKey, Vec<i32>> = IndexMap::new();
    map.insert(TaggedKey::new(10, "ten"), vec![10]);
    map.insert(TaggedKey::new(20, "twenty"), vec![20, 21]);
    map.insert(TaggedKey::new(30, "thirty"), vec![30]);

    let key = TaggedKey::new(20, "lookup-label-does-not-matter");
    let hash = make_hash(&map, &key);

    let found = map
        .raw_entry_v1()
        .from_hash(hash, |stored| stored.id == 20)
        .expect("key with matching hash and predicate should exist");
    assert_eq!(found.0.id, 20);
    assert_eq!(found.0.label, "twenty");
    assert_eq!(found.1.as_slice(), &[20, 21]);

    let full = map
        .raw_entry_v1()
        .from_hash_full(hash, |stored| stored.id == 20)
        .expect("full lookup should include insertion index");
    assert_eq!(full.0, 1);
    assert_eq!(full.1.id, 20);
    assert_eq!(full.2.as_slice(), &[20, 21]);

    assert_eq!(
        map.raw_entry_v1()
            .index_from_hash(hash, |stored| stored.id == 20),
        Some(1)
    );
    assert_eq!(
        map.raw_entry_v1()
            .from_hash(hash, |stored| stored.id == 99),
        None
    );
}

#[test]
fn raw_entry_and_modify_or_insert_with_are_lazy_and_return_mutable_value() {
    let mut map: IndexMap<TaggedKey, Vec<i32>> = IndexMap::new();
    map.insert(TaggedKey::new(1, "one"), vec![1]);
    map.insert(TaggedKey::new(2, "two"), vec![2]);

    let existing = TaggedKey::new(1, "ignored");
    let existing_hash = make_hash(&map, &existing);
    let mut inserted_existing = false;

    map.raw_entry_mut_v1()
        .from_hash(existing_hash, |key| key.id == 1)
        .and_modify(|_key, value| {
            value.push(10);
        })
        .or_insert_with(|| {
            inserted_existing = true;
            (TaggedKey::new(1, "replacement"), vec![99])
        })
        .1
        .push(100);

    assert!(!inserted_existing);
    assert_eq!(map.get(&TaggedKey::new(1, "any")).map(Vec::as_slice), Some(&[1, 10, 100][..]));
    assert_eq!(map.len(), 2);

    let missing = TaggedKey::new(3, "three");
    let missing_hash = make_hash(&map, &missing);
    let inserted = map
        .raw_entry_mut_v1()
        .from_hash(missing_hash, |key| key.id == 3)
        .and_modify(|_key, value| value.push(-1))
        .or_insert_with(|| (TaggedKey::new(3, "three"), vec![3]));

    assert_eq!(inserted.0.id, 3);
    assert_eq!(inserted.0.label, "three");
    inserted.1.push(30);

    assert_eq!(map.get(&TaggedKey::new(3, "any")).map(Vec::as_slice), Some(&[3, 30][..]));
    assert_eq!(map.get_index_of(&TaggedKey::new(3, "any")), Some(2));
}

#[test]
fn raw_occupied_entry_can_mutate_key_value_replace_key_and_remove_entries() {
    let mut map: IndexMap<TaggedKey, Vec<i32>> = IndexMap::new();
    map.insert(TaggedKey::new(1, "alpha"), vec![1]);
    map.insert(TaggedKey::new(2, "beta"), vec![2]);
    map.insert(TaggedKey::new(3, "gamma"), vec![3]);

    let lookup = TaggedKey::new(2, "lookup");
    let hash = make_hash(&map, &lookup);

    match map
        .raw_entry_mut_v1()
        .from_hash(hash, |key| key.id == 2)
    {
        RawEntryMut::Occupied(mut entry) => {
            assert_eq!(entry.index(), 1);
            entry.key_mut().label.push_str("-mutated");

            let (key, value) = entry.get_key_value_mut();
            assert_eq!(key.label, "beta-mutated");
            value.push(20);

            let (key, value) = entry.into_key_value_mut();
            key.label.push_str("-again");
            value.push(200);
        }
        RawEntryMut::Vacant(_) => panic!("expected occupied entry"),
    }

    assert_eq!(
        snapshot(&map),
        vec![
            (1, "alpha".to_string(), vec![1]),
            (2, "beta-mutated-again".to_string(), vec![2, 20, 200]),
            (3, "gamma".to_string(), vec![3]),
        ]
    );

    let lookup = TaggedKey::new(2, "lookup");
    let hash = make_hash(&map, &lookup);
    match map
        .raw_entry_mut_v1()
        .from_hash(hash, |key| key.id == 2)
    {
        RawEntryMut::Occupied(mut entry) => {
            let old_key = entry.insert_key(TaggedKey::new(2, "beta-rekeyed"));
            assert_eq!(old_key.id, 2);
            assert_eq!(old_key.label, "beta-mutated-again");
            assert_eq!(entry.key().label, "beta-rekeyed");
        }
        RawEntryMut::Vacant(_) => panic!("expected occupied entry"),
    }

    let lookup = TaggedKey::new(2, "lookup");
    let hash = make_hash(&map, &lookup);
    match map
        .raw_entry_mut_v1()
        .from_hash(hash, |key| key.id == 2)
    {
        RawEntryMut::Occupied(entry) => {
            let key = entry.into_key();
            key.label.push_str("-into-key");
        }
        RawEntryMut::Vacant(_) => panic!("expected occupied entry"),
    }

    assert_eq!(
        map.get_key_value(&TaggedKey::new(2, "lookup"))
            .map(|(key, value)| (key.label.as_str(), value.as_slice())),
        Some(("beta-rekeyed-into-key", &[2, 20, 200][..]))
    );

    let lookup = TaggedKey::new(1, "lookup");
    let hash = make_hash(&map, &lookup);
    let removed = match map
        .raw_entry_mut_v1()
        .from_hash(hash, |key| key.id == 1)
    {
        RawEntryMut::Occupied(entry) => entry.remove_entry(),
        RawEntryMut::Vacant(_) => panic!("expected occupied entry"),
    };
    assert_eq!(removed.0.id, 1);
    assert_eq!(removed.0.label, "alpha");
    assert_eq!(removed.1, vec![1]);

    let lookup = TaggedKey::new(2, "lookup");
    let hash = make_hash(&map, &lookup);
    let shifted = match map
        .raw_entry_mut_v1()
        .from_hash(hash, |key| key.id == 2)
    {
        RawEntryMut::Occupied(entry) => entry.shift_remove_entry(),
        RawEntryMut::Vacant(_) => panic!("expected occupied entry"),
    };
    assert_eq!(shifted.0.id, 2);
    assert_eq!(shifted.0.label, "beta-rekeyed-into-key");
    assert_eq!(shifted.1, vec![2, 20, 200]);

    assert_eq!(snapshot(&map), vec![(3, "gamma".to_string(), vec![3])]);
}

#[test]
fn raw_vacant_entry_inserts_with_precomputed_hash_without_rehashing_lookup() {
    let mut map: IndexMap<TaggedKey, Vec<i32>> = IndexMap::new();
    map.insert(TaggedKey::new(1, "one"), vec![1]);

    let missing = TaggedKey::new(4, "four");
    let missing_hash = make_hash(&map, &missing);

    match map
        .raw_entry_mut_v1()
        .from_hash(missing_hash, |key| key.id == 4)
    {
        RawEntryMut::Occupied(_) => panic!("entry should be vacant before insertion"),
        RawEntryMut::Vacant(entry) => {
            assert_eq!(entry.index(), 1);
            let (key, value) =
                entry.insert_hashed_nocheck(missing_hash, TaggedKey::new(4, "four"), vec![4]);
            assert_eq!(key.id, 4);
            assert_eq!(key.label, "four");
            value.push(40);
        }
    }

    assert_eq!(map.len(), 2);
    assert_eq!(map.get_index_of(&TaggedKey::new(4, "ignored")), Some(1));
    assert_eq!(
        map.get(&TaggedKey::new(4, "ignored")).map(Vec::as_slice),
        Some(&[4, 40][..])
    );

    let found = map
        .raw_entry_v1()
        .from_hash(missing_hash, |key| key.id == 4)
        .expect("inserted key should be discoverable by the same hash");
    assert_eq!(found.0.label, "four");
    assert_eq!(found.1.as_slice(), &[4, 40]);
}