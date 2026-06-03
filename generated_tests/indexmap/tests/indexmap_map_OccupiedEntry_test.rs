#![allow(deprecated)]

use indexmap::map::Entry;
use indexmap::IndexMap;

fn ordered_entries(map: &IndexMap<&'static str, i32>) -> Vec<(&'static str, i32)> {
    map.iter().map(|(key, value)| (*key, *value)).collect()
}

#[test]
fn occupied_entry_remove_entry_removes_pair_and_backfills_with_last_entry() {
    let mut map = IndexMap::new();

    assert_eq!(map.insert("alpha", 10), None);
    assert_eq!(map.insert("bravo", 20), None);
    assert_eq!(map.insert("charlie", 30), None);
    assert_eq!(map.insert("delta", 40), None);
    assert_eq!(map.insert("echo", 50), None);

    assert_eq!(map.len(), 5);
    assert_eq!(map.get_index_of("bravo"), Some(1));
    assert_eq!(map.last(), Some((&"echo", &50)));

    let removed = match map.entry("bravo") {
        Entry::Occupied(entry) => {
            assert_eq!(entry.index(), 1);
            assert_eq!(entry.key(), &"bravo");
            assert_eq!(entry.get(), &20);
            entry.remove_entry()
        }
        Entry::Vacant(_) => panic!("bravo should be occupied before removal"),
    };

    assert_eq!(removed, ("bravo", 20));
    assert_eq!(map.len(), 4);
    assert!(!map.contains_key("bravo"));
    assert_eq!(map.get("bravo"), None);

    assert_eq!(map.get_index(0), Some((&"alpha", &10)));
    assert_eq!(map.get_index(1), Some((&"echo", &50)));
    assert_eq!(map.get_index(2), Some((&"charlie", &30)));
    assert_eq!(map.get_index(3), Some((&"delta", &40)));
    assert_eq!(
        ordered_entries(&map),
        vec![("alpha", 10), ("echo", 50), ("charlie", 30), ("delta", 40)]
    );

    assert_eq!(map.insert("foxtrot", 60), None);
    assert_eq!(map.get_index_of("foxtrot"), Some(4));
    assert_eq!(map.pop(), Some(("foxtrot", 60)));
}

#[test]
fn occupied_entry_shift_remove_entry_preserves_relative_order_after_removed_entry() {
    let mut map = IndexMap::new();

    for (key, value) in [
        ("alpha", 100),
        ("bravo", 200),
        ("charlie", 300),
        ("delta", 400),
        ("echo", 500),
        ("foxtrot", 600),
    ] {
        assert_eq!(map.insert(key, value), None);
    }

    {
        let value = map
            .get_mut("delta")
            .expect("delta should be available for a pre-removal update");
        *value += 44;
    }

    assert_eq!(map.get_full("delta"), Some((3, &"delta", &444)));
    assert_eq!(map.get_index_of("echo"), Some(4));

    let removed = match map.entry("delta") {
        Entry::Occupied(mut entry) => {
            assert_eq!(entry.index(), 3);
            assert_eq!(entry.key(), &"delta");
            assert_eq!(entry.insert(445), 444);
            assert_eq!(entry.get(), &445);
            entry.shift_remove_entry()
        }
        Entry::Vacant(_) => panic!("delta should be occupied before shift removal"),
    };

    assert_eq!(removed, ("delta", 445));
    assert_eq!(map.len(), 5);
    assert!(!map.contains_key("delta"));
    assert_eq!(map.get_index_of("echo"), Some(3));
    assert_eq!(map.get_index_of("foxtrot"), Some(4));
    assert_eq!(
        ordered_entries(&map),
        vec![
            ("alpha", 100),
            ("bravo", 200),
            ("charlie", 300),
            ("echo", 500),
            ("foxtrot", 600),
        ]
    );

    assert_eq!(map.shift_insert(2, "golf", 700), None);
    assert_eq!(
        ordered_entries(&map),
        vec![
            ("alpha", 100),
            ("bravo", 200),
            ("golf", 700),
            ("charlie", 300),
            ("echo", 500),
            ("foxtrot", 600),
        ]
    );
}

#[test]
fn occupied_entry_removal_methods_handle_first_and_last_edge_positions() {
    let mut remove_last = IndexMap::new();
    assert_eq!(remove_last.insert("one", 1), None);
    assert_eq!(remove_last.insert("two", 2), None);
    assert_eq!(remove_last.insert("three", 3), None);

    let removed_last = match remove_last.entry("three") {
        Entry::Occupied(entry) => {
            assert_eq!(entry.index(), 2);
            entry.remove_entry()
        }
        Entry::Vacant(_) => panic!("three should be occupied"),
    };

    assert_eq!(removed_last, ("three", 3));
    assert_eq!(remove_last.len(), 2);
    assert_eq!(ordered_entries(&remove_last), vec![("one", 1), ("two", 2)]);

    let mut shift_first = IndexMap::new();
    assert_eq!(shift_first.insert("one", 1), None);
    assert_eq!(shift_first.insert("two", 2), None);
    assert_eq!(shift_first.insert("three", 3), None);

    let removed_first = match shift_first.entry("one") {
        Entry::Occupied(entry) => {
            assert_eq!(entry.index(), 0);
            entry.shift_remove_entry()
        }
        Entry::Vacant(_) => panic!("one should be occupied"),
    };

    assert_eq!(removed_first, ("one", 1));
    assert_eq!(shift_first.len(), 2);
    assert_eq!(shift_first.first(), Some((&"two", &2)));
    assert_eq!(shift_first.last(), Some((&"three", &3)));
    assert_eq!(ordered_entries(&shift_first), vec![("two", 2), ("three", 3)]);
}