use hashbrown::HashMap;
use hashbrown::hash_map::Entry;

#[test]
fn test_or_insert_with_basic() {
    let mut map: HashMap<&str, Vec<i32>> = HashMap::new();

    // Pre-state: map is empty
    assert_eq!(map.len(), 0);
    assert!(!map.contains_key("numbers"));

    // or_insert_with on a vacant entry should call the closure
    let val = map.entry("numbers").or_insert_with(|| vec![1, 2, 3]);
    assert_eq!(val, &vec![1, 2, 3]);
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("numbers"));

    // or_insert_with on an occupied entry should NOT call the closure
    {
        let val = map.entry("numbers").or_insert_with(|| vec![99, 100]);
        assert_eq!(val, &vec![1, 2, 3]);
    }
    assert_eq!(map.len(), 1);

    // Mutate through the returned reference
    let val = map.entry("numbers").or_insert_with(|| vec![]);
    val.push(4);
    assert_eq!(map["numbers"], vec![1, 2, 3, 4]);
}

#[test]
fn test_or_insert_with_multiple_entries() {
    let mut map: HashMap<i32, String> = HashMap::new();
    let mut call_count = 0;

    assert_eq!(map.len(), 0);

    for i in 0..5 {
        map.entry(i).or_insert_with(|| {
            call_count += 1;
            format!("value_{}", i)
        });
    }

    assert_eq!(map.len(), 5);
    assert_eq!(call_count, 5);
    assert_eq!(map[&0], "value_0");
    assert_eq!(map[&4], "value_4");

    // Insert again — closure should not be called
    for i in 0..5 {
        map.entry(i).or_insert_with(|| {
            call_count += 1;
            format!("new_value_{}", i)
        });
    }

    assert_eq!(call_count, 5); // unchanged
    assert_eq!(map.len(), 5);
    assert_eq!(map[&2], "value_2");
}

#[test]
fn test_or_insert_with_key_basic() {
    let mut map: HashMap<String, usize> = HashMap::new();

    assert_eq!(map.len(), 0);

    // or_insert_with_key receives the key as argument
    let val = map
        .entry("hello".to_string())
        .or_insert_with_key(|key| key.len());

    assert_eq!(*val, 5);
    assert_eq!(map.len(), 1);
    assert_eq!(map["hello"], 5);

    // On occupied entry, closure is not called
    let val = map
        .entry("hello".to_string())
        .or_insert_with_key(|key| key.len() * 100);

    assert_eq!(*val, 5); // unchanged
    assert_eq!(map.len(), 1);
}

#[test]
fn test_or_insert_with_key_derives_from_key() {
    let mut map: HashMap<String, String> = HashMap::new();

    let keys = vec!["alpha", "beta", "gamma", "delta"];

    for k in &keys {
        map.entry(k.to_string())
            .or_insert_with_key(|key| format!("{}_derived", key));
    }

    assert_eq!(map.len(), 4);
    assert_eq!(map["alpha"], "alpha_derived");
    assert_eq!(map["beta"], "beta_derived");
    assert_eq!(map["gamma"], "gamma_derived");
    assert_eq!(map["delta"], "delta_derived");

    // Re-insert should not change values
    map.entry("alpha".to_string())
        .or_insert_with_key(|key| format!("{}_new", key));
    assert_eq!(map["alpha"], "alpha_derived");
    assert_eq!(map.len(), 4);
}

#[test]
fn test_and_modify_on_occupied() {
    let mut map: HashMap<&str, i32> = HashMap::new();
    map.insert("counter", 10);

    assert_eq!(map.len(), 1);
    assert_eq!(map["counter"], 10);

    // and_modify should run the closure on occupied entry
    map.entry("counter").and_modify(|v| *v += 5).or_insert(0);

    assert_eq!(map["counter"], 15);
    assert_eq!(map.len(), 1);

    // Chain multiple and_modify calls
    map.entry("counter")
        .and_modify(|v| *v *= 2)
        .and_modify(|v| *v += 1)
        .or_insert(0);

    assert_eq!(map["counter"], 31);
    assert_eq!(map.len(), 1);
}

#[test]
fn test_and_modify_on_vacant() {
    let mut map: HashMap<&str, i32> = HashMap::new();

    assert_eq!(map.len(), 0);
    assert!(!map.contains_key("missing"));

    // and_modify should NOT run the closure on vacant entry
    map.entry("missing").and_modify(|v| *v += 100).or_insert(42);

    assert_eq!(map["missing"], 42);
    assert_eq!(map.len(), 1);

    // Now it's occupied, and_modify should run
    map.entry("missing").and_modify(|v| *v += 8).or_insert(0);
    assert_eq!(map["missing"], 50);
    assert_eq!(map.len(), 1);
}

#[test]
fn test_and_modify_chained_with_or_insert_with() {
    let mut map: HashMap<String, Vec<i32>> = HashMap::new();

    // Vacant: and_modify skipped, or_insert_with called
    map.entry("list".to_string())
        .and_modify(|v| v.push(999))
        .or_insert_with(|| vec![1]);

    assert_eq!(map["list"], vec![1]);
    assert_eq!(map.len(), 1);

    // Occupied: and_modify runs, or_insert_with skipped
    map.entry("list".to_string())
        .and_modify(|v| v.push(2))
        .or_insert_with(|| vec![999]);

    assert_eq!(map["list"], vec![1, 2]);

    // Another round
    map.entry("list".to_string())
        .and_modify(|v| v.push(3))
        .or_insert_with(|| vec![999]);

    assert_eq!(map["list"], vec![1, 2, 3]);
    assert_eq!(map.len(), 1);
}

#[test]
fn test_and_replace_entry_with_occupied_returns_some() {
    let mut map: HashMap<&str, u32> = HashMap::new();
    map.insert("x", 100);
    map.insert("y", 200);

    assert_eq!(map.len(), 2);
    assert_eq!(map["x"], 100);

    // When closure returns Some, the entry is replaced
    let entry = map
        .entry("x")
        .and_replace_entry_with(|_k, v| Some(v * 3));

    // The entry should still be occupied
    match entry {
        Entry::Occupied(ref o) => {
            assert_eq!(*o.get(), 300);
            assert_eq!(*o.key(), "x");
        }
        Entry::Vacant(_) => panic!("expected occupied"),
    }

    assert_eq!(map["x"], 300);
    assert_eq!(map.len(), 2);
}

#[test]
fn test_and_replace_entry_with_occupied_returns_none() {
    let mut map: HashMap<&str, u32> = HashMap::new();
    map.insert("a", 10);
    map.insert("b", 20);
    map.insert("c", 30);

    assert_eq!(map.len(), 3);

    // When closure returns None, the entry is removed
    let entry = map
        .entry("b")
        .and_replace_entry_with(|_k, _v| None);

    // The entry should now be vacant
    match entry {
        Entry::Vacant(v) => {
            assert_eq!(*v.key(), "b");
        }
        Entry::Occupied(_) => panic!("expected vacant"),
    }

    assert!(!map.contains_key("b"));
    assert_eq!(map.len(), 2);
    assert_eq!(map["a"], 10);
    assert_eq!(map["c"], 30);
}

#[test]
fn test_and_replace_entry_with_on_vacant() {
    let mut map: HashMap<&str, u32> = HashMap::new();
    map.insert("exists", 5);

    assert_eq!(map.len(), 1);
    assert!(!map.contains_key("ghost"));

    // On vacant entry, closure is not called, entry remains vacant
    let entry = map
        .entry("ghost")
        .and_replace_entry_with(|_k, _v| {
            panic!("should not be called on vacant");
        });

    match entry {
        Entry::Vacant(v) => {
            assert_eq!(*v.key(), "ghost");
        }
        Entry::Occupied(_) => panic!("expected vacant"),
    }

    assert!(!map.contains_key("ghost"));
    assert_eq!(map.len(), 1);
    assert_eq!(map["exists"], 5);
}

#[test]
fn test_or_default_on_vacant() {
    let mut map: HashMap<&str, i32> = HashMap::new();

    assert_eq!(map.len(), 0);

    // or_default on vacant inserts Default::default()
    {
        let val = map.entry("zero").or_default();
        assert_eq!(*val, 0);
    }
    assert_eq!(map.len(), 1);
    assert_eq!(map["zero"], 0);

    // Mutate through reference
    {
        let val = map.entry("zero").or_default();
        *val = 77;
    }
    assert_eq!(map["zero"], 77);
    assert_eq!(map.len(), 1);
}

#[test]
fn test_or_default_on_occupied() {
    let mut map: HashMap<&str, String> = HashMap::new();
    map.insert("greeting", "hello".to_string());

    assert_eq!(map.len(), 1);
    assert_eq!(map["greeting"], "hello");

    // or_default on occupied returns existing value
    let val = map.entry("greeting").or_default();
    assert_eq!(val, "hello");
    assert_eq!(map.len(), 1);

    // or_default on vacant inserts empty string (String::default())
    let val = map.entry("empty").or_default();
    assert_eq!(val, "");
    assert_eq!(map.len(), 2);
    assert_eq!(map["empty"], "");
}

#[test]
fn test_or_default_with_vec() {
    let mut map: HashMap<i32, Vec<i32>> = HashMap::new();

    assert_eq!(map.len(), 0);

    // Default for Vec is empty vec
    let val = map.entry(1).or_default();
    assert_eq!(val, &Vec::<i32>::new());
    assert!(val.is_empty());

    val.push(10);
    val.push(20);

    assert_eq!(map[&1], vec![10, 20]);
    assert_eq!(map.len(), 1);

    // Access again — should get existing vec
    let val = map.entry(1).or_default();
    assert_eq!(val, &vec![10, 20]);
    assert_eq!(map.len(), 1);
}

#[test]
fn test_entry_workflow_combined() {
    let mut word_counts: HashMap<&str, usize> = HashMap::new();
    let text = ["the", "quick", "brown", "fox", "the", "quick", "the"];

    // Count words using and_modify + or_insert
    for word in &text {
        word_counts
            .entry(word)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    assert_eq!(word_counts.len(), 4);
    assert_eq!(word_counts["the"], 3);
    assert_eq!(word_counts["quick"], 2);
    assert_eq!(word_counts["brown"], 1);
    assert_eq!(word_counts["fox"], 1);

    // Remove entries with count < 2 using and_replace_entry_with
    let keys: Vec<&str> = word_counts.keys().copied().collect();
    for key in keys {
        word_counts
            .entry(key)
            .and_replace_entry_with(|_k, v| if v >= 2 { Some(v) } else { None });
    }

    assert_eq!(word_counts.len(), 2);
    assert!(word_counts.contains_key("the"));
    assert!(word_counts.contains_key("quick"));
    assert!(!word_counts.contains_key("brown"));
    assert!(!word_counts.contains_key("fox"));
}

#[test]
fn test_or_insert_with_expensive_computation() {
    let mut map: HashMap<u32, u64> = HashMap::new();
    let mut computation_count = 0u32;

    // Simulate expensive computation that should only run for vacant entries
    let expensive = |n: u32, counter: &mut u32| -> u64 {
        *counter += 1;
        (n as u64) * (n as u64) * 1000
    };

    // First insertion — computation runs
    map.entry(5).or_insert_with(|| expensive(5, &mut computation_count));
    assert_eq!(computation_count, 1);
    assert_eq!(map[&5], 25000);

    // Second access — computation should NOT run
    map.entry(5).or_insert_with(|| expensive(5, &mut computation_count));
    assert_eq!(computation_count, 1);
    assert_eq!(map[&5], 25000);

    // New key — computation runs
    map.entry(10).or_insert_with(|| expensive(10, &mut computation_count));
    assert_eq!(computation_count, 2);
    assert_eq!(map[&10], 100000);
    assert_eq!(map.len(), 2);
}

#[test]
fn test_or_insert_with_key_string_lengths() {
    let mut map: HashMap<String, usize> = HashMap::new();

    let words = vec![
        "rust", "hashbrown", "swisstable", "map", "set",
    ];

    for w in &words {
        map.entry(w.to_string())
            .or_insert_with_key(|k| k.len());
    }

    assert_eq!(map.len(), 5);
    assert_eq!(map["rust"], 4);
    assert_eq!(map["hashbrown"], 9);
    assert_eq!(map["swisstable"], 10);
    assert_eq!(map["map"], 3);
    assert_eq!(map["set"], 3);

    // Verify re-insertion doesn't change values
    map.entry("rust".to_string())
        .or_insert_with_key(|k| k.len() * 100);
    assert_eq!(map["rust"], 4);
    assert_eq!(map.len(), 5);
}

#[test]
fn test_and_modify_with_or_default_pattern() {
    // Common pattern: increment counter or start at default
    let mut map: HashMap<char, u32> = HashMap::new();
    let chars = "abracadabra";

    for c in chars.chars() {
        map.entry(c).and_modify(|count| *count += 1).or_default();
        // After or_default, first occurrence has value 0, need to add 1
    }

    // With this pattern: first time or_default gives 0, subsequent and_modify adds 1
    // So counts are off by one from actual frequency. Let's verify the logic:
    // 'a' appears 5 times: first time -> 0, then +1 four times -> 4
    // 'b' appears 2 times: first time -> 0, then +1 once -> 1
    // 'r' appears 2 times: first time -> 0, then +1 once -> 1
    // 'c' appears 1 time: first time -> 0 -> 0
    // 'd' appears 1 time: first time -> 0 -> 0
    assert_eq!(map[&'a'], 4);
    assert_eq!(map[&'b'], 1);
    assert_eq!(map[&'r'], 1);
    assert_eq!(map[&'c'], 0);
    assert_eq!(map[&'d'], 0);
    assert_eq!(map.len(), 5);
}