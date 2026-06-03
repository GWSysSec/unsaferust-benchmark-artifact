use indexmap::map::Entry;
use indexmap::IndexMap;

#[test]
fn entry_chains_modify_existing_and_lazily_insert_missing() {
    let mut map: IndexMap<String, Vec<i32>> = IndexMap::new();

    map.insert("alpha".to_string(), vec![1, 2]);
    map.insert("beta".to_string(), vec![10]);

    let mut insert_called = false;
    let alpha_values = map
        .entry("alpha".to_string())
        .and_modify(|values| {
            values.push(3);
            values[0] *= 10;
        })
        .or_insert_with(|| {
            insert_called = true;
            vec![99]
        });

    alpha_values.push(4);

    assert!(!insert_called, "or_insert_with must not run for occupied entries");
    assert_eq!(map.get("alpha").map(Vec::as_slice), Some(&[10, 2, 3, 4][..]));
    assert_eq!(map.get("beta").map(Vec::as_slice), Some(&[10][..]));
    assert_eq!(map.len(), 2);
    assert_eq!(map.get_index_of("alpha"), Some(0));

    let gamma_values = map
        .entry("gamma".to_string())
        .and_modify(|values| values.push(-1))
        .or_insert_with(|| vec![7, 8]);

    gamma_values.push(9);

    assert_eq!(map.get("gamma").map(Vec::as_slice), Some(&[7, 8, 9][..]));
    assert_eq!(map.len(), 3);
    assert_eq!(map.get_index_of("gamma"), Some(2));
}

#[test]
fn or_insert_with_key_uses_borrowed_key_without_replacing_existing_values() {
    let mut map: IndexMap<String, usize> = IndexMap::new();

    let first = map
        .entry("short".to_string())
        .or_insert_with_key(|key| key.len());
    assert_eq!(*first, 5);

    let second = map
        .entry("a much longer key".to_string())
        .or_insert_with_key(|key| key.bytes().filter(|byte| *byte == b' ').count());
    assert_eq!(*second, 3);

    let existing = map
        .entry("short".to_string())
        .or_insert_with_key(|key| key.len() * 100);
    assert_eq!(*existing, 5, "existing value must not be recomputed");

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("short"), Some(&5));
    assert_eq!(map.get("a much longer key"), Some(&3));

    let keys: Vec<&str> = map.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["short", "a much longer key"]);
}

#[test]
fn and_modify_can_be_combined_with_entry_matching_and_insert_defaults() {
    let mut counts: IndexMap<String, usize> = IndexMap::new();

    for word in ["red", "blue", "red", "green", "blue", "red"] {
        counts
            .entry(word.to_string())
            .and_modify(|count| *count += 1)
            .or_insert_with(|| 1);
    }

    assert_eq!(counts.get("red"), Some(&3));
    assert_eq!(counts.get("blue"), Some(&2));
    assert_eq!(counts.get("green"), Some(&1));
    assert_eq!(counts.get("missing"), None);

    match counts.entry("blue".to_string()) {
        Entry::Occupied(mut occupied) => {
            assert_eq!(occupied.index(), 1);
            assert_eq!(occupied.key(), "blue");
            assert_eq!(*occupied.get(), 2);
            assert_eq!(occupied.insert(20), 2);
        }
        Entry::Vacant(_) => panic!("blue should already be present"),
    }

    counts
        .entry("yellow".to_string())
        .and_modify(|count| *count += 100)
        .or_insert_with_key(|key| key.len());

    assert_eq!(counts.get("blue"), Some(&20));
    assert_eq!(counts.get("yellow"), Some(&6));
    assert_eq!(counts.len(), 4);
}