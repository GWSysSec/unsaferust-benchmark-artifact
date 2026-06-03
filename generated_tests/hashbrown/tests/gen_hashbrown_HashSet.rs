use hashbrown::HashSet;

#[test]
fn test_hashset_retain_filters_correctly() {
    let mut set: HashSet<i32> = HashSet::new();
    for i in 0..20 {
        set.insert(i);
    }
    assert_eq!(set.len(), 20);


    set.retain(|&x| x % 2 == 0);

    assert_eq!(set.len(), 10);
    assert!(set.contains(&0));
    assert!(set.contains(&2));
    assert!(set.contains(&18));
    assert!(!set.contains(&1));
    assert!(!set.contains(&3));
    assert!(!set.contains(&19));


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


    let extracted: Vec<i32> = set.extract_if(|&x| x % 3 == 0).collect();

    assert_eq!(extracted.len(), 10);
    assert_eq!(set.len(), 20);


    for item in &extracted {
        assert_eq!(item % 3, 0);
    }


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


    let mut extractor = set.extract_if(|&x| x % 5 == 0);
    let first = extractor.next();
    assert!(first.is_some());
    let first_val = first.unwrap();
    assert_eq!(first_val % 5, 0);

    let second = extractor.next();
    assert!(second.is_some());
    let second_val = second.unwrap();
    assert_eq!(second_val % 5, 0);


    drop(extractor);


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

    assert_eq!(set.capacity(), cap_before);


    set.insert("new_item".to_string());
    assert_eq!(set.len(), 1);
    assert!(set.contains("new_item"));
}

#[test]
fn test_hashset_allocator() {
    let set: HashSet<i32> = HashSet::new();
    let _alloc = set.allocator();


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


    set.shrink_to(10);
    let cap_after = set.capacity();
    assert!(cap_after >= 3);
    assert!(cap_after >= 10);
    assert!(cap_after <= cap_before);


    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
    assert_eq!(set.len(), 3);


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


    let result3 = set.get_or_insert("hello".to_string());
    assert_eq!(result3, "hello");
    assert_eq!(set.len(), 2);
}

#[test]
fn test_hashset_get_or_insert_existing_value() {
    let mut set: HashSet<i32> = HashSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    assert_eq!(set.len(), 3);


    let val = set.get_or_insert(10);
    assert_eq!(*val, 10);
    assert_eq!(set.len(), 3);

    let val = set.get_or_insert(20);
    assert_eq!(*val, 20);
    assert_eq!(set.len(), 3);


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


    let result = set.get_or_insert_with("cherry", |s| s.to_string());
    assert_eq!(result, "cherry");
    assert_eq!(set.len(), 3);
    assert!(set.contains("cherry"));


    let result = set.get_or_insert_with("apple", |s| s.to_string());
    assert_eq!(result, "apple");
    assert_eq!(set.len(), 3);


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


    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));


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


    for i in 0..50 {
        set.insert(i);
    }
    assert_eq!(set.len(), 50);


    set.retain(|&x| x % 2 == 1);
    assert_eq!(set.len(), 25);
    assert!(!set.contains(&0));
    assert!(set.contains(&1));
    assert!(!set.contains(&2));
    assert!(set.contains(&49));


    let extracted: Vec<i32> = set.extract_if(|&x| x % 5 == 0).collect();
    assert_eq!(extracted.len(), 5);
    assert_eq!(set.len(), 20);
    assert!(!set.contains(&5));
    assert!(!set.contains(&15));
    assert!(set.contains(&1));
    assert!(set.contains(&3));


    let taken = set.take(&1);
    assert_eq!(taken, Some(1));
    assert_eq!(set.len(), 19);


    let val = set.get_or_insert(3);
    assert_eq!(*val, 3);
    assert_eq!(set.len(), 19);


    let val = set.get_or_insert(100);
    assert_eq!(*val, 100);
    assert_eq!(set.len(), 20);


    set.shrink_to(20);
    assert!(set.capacity() >= 20);
    assert_eq!(set.len(), 20);


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


    set.retain(|&x| x % 7 == 0);

    let expected_count = (0..10_000).filter(|x| x % 7 == 0).count();
    assert_eq!(set.len(), expected_count);


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


    let extracted: Vec<i32> = set.extract_if(|_| true).collect();
    assert_eq!(extracted.len(), 20);
    assert_eq!(set.len(), 0);
    assert!(set.is_empty());


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


    let extracted: Vec<i32> = set.extract_if(|_| false).collect();
    assert_eq!(extracted.len(), 0);
    assert_eq!(set.len(), 10);


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


    let taken = set.take("two").unwrap();
    assert_eq!(taken, "two");
    assert_eq!(set.len(), 2);
    assert!(!set.contains("two"));


    let modified = format!("{}_modified", taken);
    set.insert(modified);
    assert_eq!(set.len(), 3);
    assert!(set.contains("two_modified"));
    assert!(!set.contains("two"));


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


    set.retain(|&x| x < 5);
    assert_eq!(set.len(), 5);

    assert_eq!(set.capacity(), big_cap);


    set.shrink_to(5);
    let small_cap = set.capacity();
    assert!(small_cap < big_cap);
    assert!(small_cap >= 5);


    assert!(set.contains(&0));
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
    assert!(set.contains(&4));
    assert!(!set.contains(&5));
    assert_eq!(set.len(), 5);
}