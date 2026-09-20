use indexmap::map::raw_entry_v1::RawEntryMut;
use indexmap::map::RawEntryApiV1;
use indexmap::IndexMap;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
struct TaggedKey {
    id: u32,
    tag: String,
}

impl TaggedKey {
    fn new(id: u32, tag: &str) -> Self {
        Self {
            id,
            tag: tag.to_string(),
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

fn ordered_entries(map: &IndexMap<TaggedKey, i32>) -> Vec<(u32, String, i32)> {
    map.iter()
        .map(|(key, value)| (key.id, key.tag.clone(), *value))
        .collect()
}

#[test]
fn raw_occupied_entry_allows_mutating_stored_key_metadata_and_value() {
    let mut map: IndexMap<TaggedKey, i32> = IndexMap::new();
    assert_eq!(map.insert_full(TaggedKey::new(1, "one-original"), 10), (0, None));
    assert_eq!(map.insert_full(TaggedKey::new(2, "two-original"), 20), (1, None));
    assert_eq!(map.insert_full(TaggedKey::new(3, "three-original"), 30), (2, None));

    match map.raw_entry_mut_v1().from_key(&TaggedKey::new(2, "lookup tag ignored")) {
        RawEntryMut::Occupied(mut entry) => {
            assert_eq!(entry.index(), 1);
            assert_eq!(entry.key().id, 2);
            assert_eq!(entry.key().tag, "two-original");

            entry.key_mut().tag.push_str("-via-key-mut");

            let (key, value) = entry.get_key_value_mut();
            assert_eq!(key.id, 2);
            assert_eq!(key.tag, "two-original-via-key-mut");
            *value += 5;
            key.tag = "two-final".to_string();
        }
        RawEntryMut::Vacant(_) => panic!("expected occupied entry for id 2"),
    }

    assert_eq!(map.get(&TaggedKey::new(2, "different lookup tag")), Some(&25));
    let (index, stored_key, stored_value) = map
        .get_full(&TaggedKey::new(2, "another lookup tag"))
        .expect("id 2 should still be findable after non-hash key mutation");
    assert_eq!(index, 1);
    assert_eq!(stored_key.tag, "two-final");
    assert_eq!(*stored_value, 25);

    assert_eq!(
        ordered_entries(&map),
        vec![
            (1, "one-original".to_string(), 10),
            (2, "two-final".to_string(), 25),
            (3, "three-original".to_string(), 30),
        ]
    );
}

#[test]
fn raw_occupied_entry_into_key_and_into_key_value_mut_return_map_bound_references() {
    let mut map: IndexMap<TaggedKey, i32> = IndexMap::new();
    map.insert(TaggedKey::new(10, "ten"), 100);
    map.insert(TaggedKey::new(20, "twenty"), 200);
    map.insert(TaggedKey::new(30, "thirty"), 300);

    match map.raw_entry_mut_v1().from_key(&TaggedKey::new(10, "ignored")) {
        RawEntryMut::Occupied(entry) => {
            let key = entry.into_key();
            assert_eq!(key.id, 10);
            key.tag = "ten-renamed".to_string();
        }
        RawEntryMut::Vacant(_) => panic!("expected occupied entry for id 10"),
    }

    match map.raw_entry_mut_v1().from_key(&TaggedKey::new(30, "ignored")) {
        RawEntryMut::Occupied(entry) => {
            let (key, value) = entry.into_key_value_mut();
            assert_eq!(key.id, 30);
            assert_eq!(*value, 300);
            key.tag.push_str("-renamed");
            *value = 333;
        }
        RawEntryMut::Vacant(_) => panic!("expected occupied entry for id 30"),
    }

    assert_eq!(map.get(&TaggedKey::new(10, "lookup")), Some(&100));
    assert_eq!(map.get(&TaggedKey::new(30, "lookup")), Some(&333));
    assert_eq!(
        ordered_entries(&map),
        vec![
            (10, "ten-renamed".to_string(), 100),
            (20, "twenty".to_string(), 200),
            (30, "thirty-renamed".to_string(), 333),
        ]
    );
}

#[test]
fn raw_occupied_insert_key_replaces_stored_key_without_changing_value_or_index() {
    let mut map: IndexMap<TaggedKey, i32> = IndexMap::new();
    map.insert(TaggedKey::new(1, "first"), 11);
    map.insert(TaggedKey::new(2, "second"), 22);
    map.insert(TaggedKey::new(3, "third"), 33);

    match map.raw_entry_mut_v1().from_key(&TaggedKey::new(2, "lookup")) {
        RawEntryMut::Occupied(mut entry) => {
            assert_eq!(entry.index(), 1);
            let old_key = entry.insert_key(TaggedKey::new(2, "second-replaced"));
            assert_eq!(old_key.id, 2);
            assert_eq!(old_key.tag, "second");
            assert_eq!(entry.index(), 1);
            assert_eq!(*entry.get(), 22);
            assert_eq!(entry.key().tag, "second-replaced");
        }
        RawEntryMut::Vacant(_) => panic!("expected occupied entry for id 2"),
    }

    assert_eq!(map.get_index_of(&TaggedKey::new(2, "lookup")), Some(1));
    assert_eq!(
        ordered_entries(&map),
        vec![
            (1, "first".to_string(), 11),
            (2, "second-replaced".to_string(), 22),
            (3, "third".to_string(), 33),
        ]
    );
}

#[test]
fn raw_occupied_remove_entry_swap_removes_pair_and_moves_last_into_gap() {
    let mut map: IndexMap<TaggedKey, i32> = IndexMap::new();
    map.insert(TaggedKey::new(1, "one"), 10);
    map.insert(TaggedKey::new(2, "two"), 20);
    map.insert(TaggedKey::new(3, "three"), 30);
    map.insert(TaggedKey::new(4, "four"), 40);

    let removed = match map.raw_entry_mut_v1().from_key(&TaggedKey::new(2, "ignored")) {
        RawEntryMut::Occupied(entry) => entry.remove_entry(),
        RawEntryMut::Vacant(_) => panic!("expected occupied entry for id 2"),
    };

    assert_eq!(removed.0.id, 2);
    assert_eq!(removed.0.tag, "two");
    assert_eq!(removed.1, 20);
    assert_eq!(map.len(), 3);
    assert!(!map.contains_key(&TaggedKey::new(2, "ignored")));
    assert_eq!(
        ordered_entries(&map),
        vec![
            (1, "one".to_string(), 10),
            (4, "four".to_string(), 40),
            (3, "three".to_string(), 30),
        ]
    );
    assert_eq!(map.get_index_of(&TaggedKey::new(4, "ignored")), Some(1));
}

#[test]
fn raw_occupied_shift_remove_entry_preserves_relative_order_of_following_entries() {
    let mut map: IndexMap<TaggedKey, i32> = IndexMap::new();
    map.insert(TaggedKey::new(1, "one"), 10);
    map.insert(TaggedKey::new(2, "two"), 20);
    map.insert(TaggedKey::new(3, "three"), 30);
    map.insert(TaggedKey::new(4, "four"), 40);

    let removed = match map.raw_entry_mut_v1().from_key(&TaggedKey::new(2, "ignored")) {
        RawEntryMut::Occupied(entry) => entry.shift_remove_entry(),
        RawEntryMut::Vacant(_) => panic!("expected occupied entry for id 2"),
    };

    assert_eq!(removed.0.id, 2);
    assert_eq!(removed.0.tag, "two");
    assert_eq!(removed.1, 20);
    assert_eq!(map.len(), 3);
    assert!(!map.contains_key(&TaggedKey::new(2, "ignored")));
    assert_eq!(
        ordered_entries(&map),
        vec![
            (1, "one".to_string(), 10),
            (3, "three".to_string(), 30),
            (4, "four".to_string(), 40),
        ]
    );
    assert_eq!(map.get_index_of(&TaggedKey::new(3, "ignored")), Some(1));
    assert_eq!(map.get_index_of(&TaggedKey::new(4, "ignored")), Some(2));
}