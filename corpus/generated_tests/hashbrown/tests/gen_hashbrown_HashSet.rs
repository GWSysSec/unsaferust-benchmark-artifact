use hashbrown::HashSet;

#[test]
fn test_hashset_retain_filters_correctly() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..20 {
        set.insert(i);
    }
    assert_eq!(set.len(), 20);

    // Retain only even numbers
    set.retain(|&x| x % 2 == 0);

    assert_eq!(set.len(), 10);
    assert!(set.contains(&0));
    assert!(set.contains(&2));
    assert!(set.contains(&18));
    assert!(!set.contains(&1));
    assert!(!set.contains(&3));
    assert!(!set.contains(&19));

    // Retain only numbers > 10
    set.retain(|&x| x > 10);
    assert_eq!(set.len(), 4);
    assert!(set.contains(&12));
    assert!(set.contains(&14));
    assert!(set.contains(&16));
    assert!(set.contains(&18));
    assert!(!set.contains(&0));
    assert!(!set.contains(&10));
}

#[test]
fn test_hashset_retain_empty_set() {
    let mut set: HashSet<i32> = HashSet::new();
    assert_eq!(set.len(), 0);

    set.retain(|_| false);
    assert_eq!(set.len(), 0);

    set.insert(42);
    set.insert(99);
    assert_eq!(set.len(), 2);

    set.retain(|_| false);
    assert_eq!(set.len(), 0);
    assert!(!set.contains(&42));
    assert!(!set.contains(&99));
    assert!(set.is_empty());
}

#[test]
fn test_hashset_extract_if_basic() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..30 {
        set.insert(i);
    }
    assert_eq!(set.len(), 30);

    // Extract all multiples of 3
    let extracted: Vec<i32> = set.extract_if(|&x| x % 3 == 0).collect();

    assert_eq!(extracted.len(), 10);
    assert_eq!(set.len(), 20);

    // Verify extracted items are all multiples of 3
    for item in &extracted {
        assert_eq!(item % 3, 0);
    }

    // Verify remaining items are NOT multiples of 3
    for item in set.iter() {
        assert_ne!(item % 3, 0);
    }

    assert!(!set.contains(&0));
    assert!(!set.contains(&9));
    assert!(set.contains(&1));
    assert!(set.contains(&2));
}

#[test]
fn test_hashset_extract_if_partial_consumption() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..50 {
        set.insert(i);
    }
    assert_eq!(set.len(), 50);

    // Only take a few items from the iterator
    let mut extractor = set.extract_if(|&x| x % 5 == 0);
    let first = extractor.next();
    assert!(first.is_some());
    let first_val = first.unwrap();
    assert_eq!(first_val % 5, 0);

    let second = extractor.next();
    assert!(second.is_some());
    let second_val = second.unwrap();
    assert_eq!(second_val % 5, 0);

    // Drop the iterator without consuming all
    drop(extractor);

    // The set should have lost at least the 2 items we consumed
    assert!(set.len() <= 48);
    assert!(!set.contains(&first_val));
    assert!(!set.contains(&second_val));
}

#[test]
fn test_hashset_clear() {
    let mut set: HashSet<String> = HashSet::new();
    set.insert("hello".to_string());
    set.insert("world".to_string());
    set.insert("foo".to_string());
    set.insert("bar".to_string());
    set.insert("baz".to_string());

    assert_eq!(set.len(), 5);
    assert!(set.contains("hello"));

    let cap_before = set.capacity();
    set.clear();

    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
    assert!(!set.contains("hello"));
    assert!(!set.contains("world"));
    // Capacity is retained after clear
    assert_eq!(set.capacity(), cap_before);

    // Can reuse after clear
    set.insert("new_item".to_string());
    assert_eq!(set.len(), 1);
    assert!(set.contains("new_item"));
}

#[test]
fn test_hashset_allocator() {
    let set: HashSet<i32> = HashSet::new();
    let _alloc = set.allocator();
    // Just verify we can call allocator() and it returns a reference
    // For the default allocator, we can at least verify the set works
    assert_eq!(set.len(), 0);

    let mut set2: HashSet<i32> = HashSet::new();
    set2.insert(1);
    set2.insert(2);
    set2.insert(3);
    let _alloc2 = set2.allocator();
    assert_eq!(set2.len(), 3);
    assert!(set2.contains(&1));
    assert!(set2.contains(&2));
    assert!(set2.contains(&3));
    // Allocator reference doesn't prevent further use
    set2.insert(4);
    assert_eq!(set2.len(), 4);
}

#[test]
fn test_hashset_shrink_to() {
    let mut set: HashSet<i32> = HashSet::with_capacity(100);
    assert!(set.capacity() >= 100);

    set.insert(1);
    set.insert(2);
    set.insert(3);
    assert_eq!(set.len(), 3);

    let cap_before = set.capacity();
    assert!(cap_before >= 100);

    // Shrink to a smaller capacity
    set.shrink_to(10);
    let cap_after = set.capacity();
    assert!(cap_after >= 3); // Must still hold all elements
    assert!(cap_after >= 10); // Must respect min_capacity
    assert!(cap_after <= cap_before); // Should have shrunk

    // All elements still present
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
    assert_eq!(set.len(), 3);

    // Shrink to 0 (should shrink to fit current elements)
    set.shrink_to(0);
    assert!(set.capacity() >= 3);
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
}

#[test]
fn test_hashset_shrink_to_larger_than_capacity() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..10 {
        set.insert(i);
    }
    assert_eq!(set.len(), 10);

    let cap_before = set.capacity();
    // Shrink to a value larger than current capacity should be a no-op
    set.shrink_to(1000);
    let cap_after = set.capacity();
    assert_eq!(cap_before, cap_after);
    assert_eq!(set.len(), 10);
    assert!(set.contains(&0));
    assert!(set.contains(&9));
}

#[test]
fn test_hashset_get_or_insert_new_value() {
    let mut set: HashSet<String> = HashSet::new();
    assert_eq!(set.len(), 0);

    let result = set.get_or_insert("hello".to_string());
    assert_eq!(result, "hello");
    assert_eq!(set.len(), 1);

    let result2 = set.get_or_insert("world".to_string());
    assert_eq!(result2, "world");
    assert_eq!(set.len(), 2);

    assert!(set.contains("hello"));
    assert!(set.contains("world"));

    // Insert existing value - should return reference to existing
    let result3 = set.get_or_insert("hello".to_string());
    assert_eq!(result3, "hello");
    assert_eq!(set.len(), 2); // No new insertion
}

#[test]
fn test_hashset_get_or_insert_existing_value() {
    let mut set: HashSet<i32> = HashSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    assert_eq!(set.len(), 3);

    // get_or_insert with existing value
    let val = set.get_or_insert(10);
    assert_eq!(*val, 10);
    assert_eq!(set.len(), 3);

    let val = set.get_or_insert(20);
    assert_eq!(*val, 20);
    assert_eq!(set.len(), 3);

    // get_or_insert with new value
    let val = set.get_or_insert(40);
    assert_eq!(*val, 40);
    assert_eq!(set.len(), 4);

    assert!(set.contains(&10));
    assert!(set.contains(&20));
    assert!(set.contains(&30));
    assert!(set.contains(&40));
}

#[test]
fn test_hashset_get_or_insert_with_new() {
    let mut set: HashSet<String> = HashSet::new();
    set.insert("apple".to_string());
    set.insert("banana".to_string());
    assert_eq!(set.len(), 2);

    // Insert a new value using get_or_insert_with
    let result = set.get_or_insert_with("cherry", |s| s.to_string());
    assert_eq!(result, "cherry");
    assert_eq!(set.len(), 3);
    assert!(set.contains("cherry"));

    // Try to insert existing value - should return existing
    let result = set.get_or_insert_with("apple", |s| s.to_string());
    assert_eq!(result, "apple");
    assert_eq!(set.len(), 3);

    // Another new value - the closure must produce a value equivalent to the lookup key
    let result = set.get_or_insert_with("date", |s| s.to_string());
    assert_eq!(result, "date");
    assert_eq!(set.len(), 4);
}

#[test]
fn test_hashset_get_or_insert_with_existing() {
    let mut set: HashSet<String> = HashSet::new();
    set.insert("hello".to_string());
    set.insert("world".to_string());
    set.insert("foo".to_string());

    assert_eq!(set.len(), 3);

    // Existing key - closure should NOT be called
    let result = set.get_or_insert_with("hello", |_| panic!("should not be called"));
    assert_eq!(result, "hello");
    assert_eq!(set.len(), 3);

    let result = set.get_or_insert_with("world", |_| panic!("should not be called"));
    assert_eq!(result, "world");
    assert_eq!(set.len(), 3);

    let result = set.get_or_insert_with("foo", |_| panic!("should not be called"));
    assert_eq!(result, "foo");
    assert_eq!(set.len(), 3);
}

#[test]
fn test_hashset_take_existing() {
    let mut set: HashSet<String> = HashSet::new();
    set.insert("alpha".to_string());
    set.insert("beta".to_string());
    set.insert("gamma".to_string());
    set.insert("delta".to_string());

    assert_eq!(set.len(), 4);

    let taken = set.take("alpha");
    assert_eq!(taken, Some("alpha".to_string()));
    assert_eq!(set.len(), 3);
    assert!(!set.contains("alpha"));

    let taken = set.take("gamma");
    assert_eq!(taken, Some("gamma".to_string()));
    assert_eq!(set.len(), 2);
    assert!(!set.contains("gamma"));

    // Remaining elements still present
    assert!(set.contains("beta"));
    assert!(set.contains("delta"));
}

#[test]
fn test_hashset_take_nonexistent() {
    let mut set: HashSet<i32> = HashSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    assert_eq!(set.len(), 3);

    let taken = set.take(&99);
    assert_eq!(taken, None);
    assert_eq!(set.len(), 3);

    let taken = set.take(&0);
    assert_eq!(taken, None);
    assert_eq!(set.len(), 3);

    // Original elements unchanged
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));

    // Take existing then try again
    let taken = set.take(&2);
    assert_eq!(taken, Some(2));
    assert_eq!(set.len(), 2);

    let taken_again = set.take(&2);
    assert_eq!(taken_again, None);
    assert_eq!(set.len(), 2);
}

#[test]
fn test_hashset_combined_workflow() {
    let mut set: HashSet<i32> = HashSet::with_capacity(64);
    assert!(set.capacity() >= 64);

    // Populate
    for i in 0..50 {
        set.insert(i);
    }
    assert_eq!(set.len(), 50);

    // Retain only odds
    set.retain(|&x| x % 2 == 1);
    assert_eq!(set.len(), 25);
    assert!(!set.contains(&0));
    assert!(set.contains(&1));
    assert!(!set.contains(&2));
    assert!(set.contains(&49));

    // Extract multiples of 5 (from the odds: 5, 15, 25, 35, 45)
    let extracted: Vec<i32> = set.extract_if(|&x| x % 5 == 0).collect();
    assert_eq!(extracted.len(), 5);
    assert_eq!(set.len(), 20);
    assert!(!set.contains(&5));
    assert!(!set.contains(&15));
    assert!(set.contains(&1));
    assert!(set.contains(&3));

    // Take a specific element
    let taken = set.take(&1);
    assert_eq!(taken, Some(1));
    assert_eq!(set.len(), 19);

    // get_or_insert existing
    let val = set.get_or_insert(3);
    assert_eq!(*val, 3);
    assert_eq!(set.len(), 19);

    // get_or_insert new
    let val = set.get_or_insert(100);
    assert_eq!(*val, 100);
    assert_eq!(set.len(), 20);

    // Shrink
    set.shrink_to(20);
    assert!(set.capacity() >= 20);
    assert_eq!(set.len(), 20);

    // Clear
    set.clear();
    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
}

#[test]
fn test_hashset_retain_with_large_set() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..10_000 {
        set.insert(i);
    }
    assert_eq!(set.len(), 10_000);

    // Retain only numbers divisible by 7
    set.retain(|&x| x % 7 == 0);

    let expected_count = (0..10_000).filter(|x| x % 7 == 0).count();
    assert_eq!(set.len(), expected_count);

    // Verify all remaining are divisible by 7
    for &item in set.iter() {
        assert_eq!(item % 7, 0);
    }

    assert!(set.contains(&0));
    assert!(set.contains(&7));
    assert!(set.contains(&14));
    assert!(!set.contains(&1));
    assert!(!set.contains(&6));
}

#[test]
fn test_hashset_extract_if_all_elements() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..20 {
        set.insert(i);
    }
    assert_eq!(set.len(), 20);

    // Extract everything
    let extracted: Vec<i32> = set.extract_if(|_| true).collect();
    assert_eq!(extracted.len(), 20);
    assert_eq!(set.len(), 0);
    assert!(set.is_empty());

    // Verify all original elements were extracted
    let mut sorted = extracted.clone();
    sorted.sort();
    let expected: Vec<i32> = (0..20).collect();
    assert_eq!(sorted, expected);
}

#[test]
fn test_hashset_extract_if_no_elements() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..10 {
        set.insert(i);
    }
    assert_eq!(set.len(), 10);

    // Extract nothing
    let extracted: Vec<i32> = set.extract_if(|_| false).collect();
    assert_eq!(extracted.len(), 0);
    assert_eq!(set.len(), 10);

    // All elements still present
    for i in 0..10 {
        assert!(set.contains(&i));
    }
}

#[test]
fn test_hashset_take_and_reinsert() {
    let mut set: HashSet<String> = HashSet::new();
    set.insert("one".to_string());
    set.insert("two".to_string());
    set.insert("three".to_string());

    assert_eq!(set.len(), 3);

    // Take and reinsert
    let taken = set.take("two").unwrap();
    assert_eq!(taken, "two");
    assert_eq!(set.len(), 2);
    assert!(!set.contains("two"));

    // Reinsert with modification
    let modified = format!("{}_modified", taken);
    set.insert(modified);
    assert_eq!(set.len(), 3);
    assert!(set.contains("two_modified"));
    assert!(!set.contains("two"));

    // Take from empty lookup
    let not_found = set.take("nonexistent");
    assert_eq!(not_found, None);
    assert_eq!(set.len(), 3);
}

#[test]
fn test_hashset_shrink_to_after_removals() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..1000 {
        set.insert(i);
    }
    assert_eq!(set.len(), 1000);
    let big_cap = set.capacity();
    assert!(big_cap >= 1000);

    // Remove most elements
    set.retain(|&x| x < 5);
    assert_eq!(set.len(), 5);
    // Capacity hasn't changed yet
    assert_eq!(set.capacity(), big_cap);

    // Now shrink
    set.shrink_to(5);
    let small_cap = set.capacity();
    assert!(small_cap < big_cap);
    assert!(small_cap >= 5);

    // Elements still intact
    assert!(set.contains(&0));
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
    assert!(set.contains(&4));
    assert!(!set.contains(&5));
    assert_eq!(set.len(), 5);
}