use indexmap::IndexSet;

#[test]
fn test_sort_unstable_basic() {
    let mut set = IndexSet::new();
    set.insert(5);
    set.insert(3);
    set.insert(8);
    set.insert(1);
    set.insert(4);

    assert_eq!(set.len(), 5);
    assert_eq!(set.get_index(0), Some(&5));
    assert_eq!(set.get_index(1), Some(&3));

    set.sort_unstable();

    assert_eq!(set.get_index(0), Some(&1));
    assert_eq!(set.get_index(1), Some(&3));
    assert_eq!(set.get_index(2), Some(&4));
    assert_eq!(set.get_index(3), Some(&5));
    assert_eq!(set.get_index(4), Some(&8));
    assert_eq!(set.len(), 5);
    assert!(set.contains(&1));
    assert!(set.contains(&8));
}

#[test]
fn test_sort_unstable_already_sorted() {
    let mut set = IndexSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);
    set.insert(4);

    set.sort_unstable();

    assert_eq!(set.get_index(0), Some(&1));
    assert_eq!(set.get_index(1), Some(&2));
    assert_eq!(set.get_index(2), Some(&3));
    assert_eq!(set.get_index(3), Some(&4));
    assert_eq!(set.len(), 4);
    assert!(set.contains(&1));
    assert!(set.contains(&4));
    assert!(!set.contains(&5));
}

#[test]
fn test_sort_unstable_by_reverse() {
    let mut set = IndexSet::new();
    set.insert(10);
    set.insert(2);
    set.insert(7);
    set.insert(5);
    set.insert(1);

    set.sort_unstable_by(|a, b| b.cmp(a));

    assert_eq!(set.get_index(0), Some(&10));
    assert_eq!(set.get_index(1), Some(&7));
    assert_eq!(set.get_index(2), Some(&5));
    assert_eq!(set.get_index(3), Some(&2));
    assert_eq!(set.get_index(4), Some(&1));
    assert_eq!(set.len(), 5);
    assert!(set.contains(&10));
    assert!(set.contains(&1));
}

#[test]
fn test_sort_unstable_by_custom_comparator() {
    let mut set = IndexSet::new();
    set.insert("banana");
    set.insert("apple");
    set.insert("cherry");
    set.insert("date");


    set.sort_unstable_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));

    assert_eq!(set.get_index(0), Some(&"date"));
    assert_eq!(set.get_index(1), Some(&"apple"));
    assert_eq!(set.get_index(2), Some(&"banana"));
    assert_eq!(set.get_index(3), Some(&"cherry"));
    assert_eq!(set.len(), 4);
    assert!(set.contains("apple"));
    assert!(set.contains("cherry"));
    assert!(!set.contains("fig"));
}

#[test]
fn test_sorted_unstable_by_consumes_set() {
    let mut set = IndexSet::new();
    set.insert(30);
    set.insert(10);
    set.insert(20);
    set.insert(50);
    set.insert(40);

    let iter = set.sorted_unstable_by(|a, b| a.cmp(b));
    let collected: Vec<i32> = iter.collect();

    assert_eq!(collected.len(), 5);
    assert_eq!(collected[0], 10);
    assert_eq!(collected[1], 20);
    assert_eq!(collected[2], 30);
    assert_eq!(collected[3], 40);
    assert_eq!(collected[4], 50);
    assert_eq!(collected.first(), Some(&10));
    assert_eq!(collected.last(), Some(&50));
}

#[test]
fn test_sorted_unstable_by_reverse() {
    let mut set = IndexSet::new();
    set.insert("x");
    set.insert("a");
    set.insert("m");
    set.insert("z");

    let iter = set.sorted_unstable_by(|a, b| b.cmp(a));
    let collected: Vec<&str> = iter.collect();

    assert_eq!(collected.len(), 4);
    assert_eq!(collected[0], "z");
    assert_eq!(collected[1], "x");
    assert_eq!(collected[2], "m");
    assert_eq!(collected[3], "a");
    assert_eq!(collected.first(), Some(&"z"));
    assert_eq!(collected.last(), Some(&"a"));
    assert_ne!(collected[0], "a");
    assert_ne!(collected[3], "z");
}

#[test]
fn test_sort_by_cached_key() {
    let mut set = IndexSet::new();
    set.insert("hello");
    set.insert("hi");
    set.insert("hey");
    set.insert("greetings");
    set.insert("yo");


    set.sort_by_cached_key(|s| s.len());

    assert_eq!(set.get_index(0), Some(&"hi"));
    assert_eq!(set.get_index(1), Some(&"yo"));
    assert_eq!(set.get_index(2), Some(&"hey"));
    assert_eq!(set.get_index(3), Some(&"hello"));
    assert_eq!(set.get_index(4), Some(&"greetings"));
    assert_eq!(set.len(), 5);
    assert!(set.contains("hello"));
    assert!(set.contains("greetings"));
}

#[test]
fn test_sort_by_cached_key_numeric() {
    let mut set = IndexSet::new();
    set.insert(100i32);
    set.insert(-50);
    set.insert(25);
    set.insert(-10);
    set.insert(0);


    set.sort_by_cached_key(|x| x.abs());

    assert_eq!(set.get_index(0), Some(&0));
    assert_eq!(set.get_index(1), Some(&-10));
    assert_eq!(set.get_index(2), Some(&25));
    assert_eq!(set.get_index(3), Some(&-50));
    assert_eq!(set.get_index(4), Some(&100));
    assert_eq!(set.len(), 5);
    assert!(set.contains(&-50));
    assert!(set.contains(&100));
}

#[test]
fn test_binary_search_found() {
    let mut set = IndexSet::new();
    set.insert(2);
    set.insert(5);
    set.insert(8);
    set.insert(12);
    set.insert(15);
    set.sort_unstable();

    let result = set.binary_search(&8);
    assert_eq!(result, Ok(2));

    let result = set.binary_search(&2);
    assert_eq!(result, Ok(0));

    let result = set.binary_search(&15);
    assert_eq!(result, Ok(4));

    let result = set.binary_search(&5);
    assert_eq!(result, Ok(1));

    let result = set.binary_search(&12);
    assert_eq!(result, Ok(3));


    let result = set.binary_search(&1);
    assert_eq!(result, Err(0));

    let result = set.binary_search(&20);
    assert_eq!(result, Err(5));

    let result = set.binary_search(&10);
    assert_eq!(result, Err(3));
}

#[test]
fn test_binary_search_by() {
    let mut set = IndexSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    set.insert(40);
    set.insert(50);
    set.sort_unstable();

    let result = set.binary_search_by(|probe| probe.cmp(&30));
    assert_eq!(result, Ok(2));

    let result = set.binary_search_by(|probe| probe.cmp(&10));
    assert_eq!(result, Ok(0));

    let result = set.binary_search_by(|probe| probe.cmp(&50));
    assert_eq!(result, Ok(4));

    let result = set.binary_search_by(|probe| probe.cmp(&25));
    assert_eq!(result, Err(2));

    let result = set.binary_search_by(|probe| probe.cmp(&5));
    assert_eq!(result, Err(0));

    let result = set.binary_search_by(|probe| probe.cmp(&55));
    assert_eq!(result, Err(5));

    let result = set.binary_search_by(|probe| probe.cmp(&20));
    assert_eq!(result, Ok(1));

    let result = set.binary_search_by(|probe| probe.cmp(&40));
    assert_eq!(result, Ok(3));
}

#[test]
fn test_binary_search_by_key() {
    let mut set: IndexSet<(i32, &str)> = IndexSet::new();
    set.insert((1, "one"));
    set.insert((3, "three"));
    set.insert((5, "five"));
    set.insert((7, "seven"));
    set.insert((9, "nine"));
    set.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    let result = set.binary_search_by_key(&5, |&(k, _)| k);
    assert_eq!(result, Ok(2));

    let result = set.binary_search_by_key(&1, |&(k, _)| k);
    assert_eq!(result, Ok(0));

    let result = set.binary_search_by_key(&9, |&(k, _)| k);
    assert_eq!(result, Ok(4));

    let result = set.binary_search_by_key(&4, |&(k, _)| k);
    assert_eq!(result, Err(2));

    let result = set.binary_search_by_key(&0, |&(k, _)| k);
    assert_eq!(result, Err(0));

    let result = set.binary_search_by_key(&10, |&(k, _)| k);
    assert_eq!(result, Err(5));

    let result = set.binary_search_by_key(&7, |&(k, _)| k);
    assert_eq!(result, Ok(3));

    let result = set.binary_search_by_key(&3, |&(k, _)| k);
    assert_eq!(result, Ok(1));
}

#[test]
fn test_partition_point() {
    let mut set = IndexSet::new();
    set.insert(1);
    set.insert(3);
    set.insert(5);
    set.insert(7);
    set.insert(9);
    set.insert(11);
    set.sort_unstable();


    let point = set.partition_point(|&x| x < 6);
    assert_eq!(point, 3);

    let point = set.partition_point(|&x| x < 1);
    assert_eq!(point, 0);

    let point = set.partition_point(|&x| x < 12);
    assert_eq!(point, 6);

    let point = set.partition_point(|&x| x < 5);
    assert_eq!(point, 2);

    let point = set.partition_point(|&x| x <= 5);
    assert_eq!(point, 3);

    let point = set.partition_point(|&x| x < 8);
    assert_eq!(point, 4);

    let point = set.partition_point(|&x| x < 10);
    assert_eq!(point, 5);

    let point = set.partition_point(|&x| x < 0);
    assert_eq!(point, 0);
}

#[test]
fn test_into_boxed_slice() {
    let mut set = IndexSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    set.insert(40);
    set.insert(50);

    let len = set.len();
    assert_eq!(len, 5);

    let boxed = set.into_boxed_slice();
    assert_eq!(boxed.len(), 5);
    assert!(boxed.iter().any(|x| *x == 10));
    assert!(boxed.iter().any(|x| *x == 20));
    assert!(boxed.iter().any(|x| *x == 30));
    assert!(boxed.iter().any(|x| *x == 40));
    assert!(boxed.iter().any(|x| *x == 50));
    assert!(!boxed.iter().any(|x| *x == 60));
    assert_eq!(boxed.first(), Some(&10));
    assert_eq!(boxed.last(), Some(&50));
}

#[test]
fn test_into_boxed_slice_sorted() {
    let mut set = IndexSet::new();
    set.insert(99);
    set.insert(1);
    set.insert(50);
    set.insert(25);
    set.sort_unstable();

    let boxed = set.into_boxed_slice();
    assert_eq!(boxed.len(), 4);
    assert_eq!(boxed[0], 1);
    assert_eq!(boxed[1], 25);
    assert_eq!(boxed[2], 50);
    assert_eq!(boxed[3], 99);
    assert_eq!(boxed.first(), Some(&1));
    assert_eq!(boxed.last(), Some(&99));
    assert!(boxed.iter().any(|x| *x == 50));
    assert!(!boxed.iter().any(|x| *x == 0));
}

#[test]
fn test_get_range_valid() {
    let mut set = IndexSet::new();
    set.insert("a");
    set.insert("b");
    set.insert("c");
    set.insert("d");
    set.insert("e");

    let slice = set.get_range(1..4);
    assert!(slice.is_some());
    let slice = slice.unwrap();
    assert_eq!(slice.len(), 3);
    assert!(slice.iter().any(|x| *x == "b"));
    assert!(slice.iter().any(|x| *x == "c"));
    assert!(slice.iter().any(|x| *x == "d"));
    assert!(!slice.iter().any(|x| *x == "a"));
    assert!(!slice.iter().any(|x| *x == "e"));
    assert_eq!(slice.first(), Some(&"b"));
    assert_eq!(slice.last(), Some(&"d"));
}

#[test]
fn test_get_range_full_and_empty() {
    let mut set = IndexSet::new();
    set.insert(100);
    set.insert(200);
    set.insert(300);


    let full = set.get_range(..);
    assert!(full.is_some());
    let full = full.unwrap();
    assert_eq!(full.len(), 3);
    assert_eq!(full.first(), Some(&100));
    assert_eq!(full.last(), Some(&300));


    let empty = set.get_range(1..1);
    assert!(empty.is_some());
    let empty = empty.unwrap();
    assert_eq!(empty.len(), 0);


    let oob = set.get_range(0..10);
    assert!(oob.is_none());


    let partial = set.get_range(..2);
    assert!(partial.is_some());
    assert_eq!(partial.unwrap().len(), 2);
}

#[test]
fn test_shift_remove_index_basic() {
    let mut set = IndexSet::new();
    set.insert("first");
    set.insert("second");
    set.insert("third");
    set.insert("fourth");
    set.insert("fifth");

    assert_eq!(set.len(), 5);
    assert_eq!(set.get_index(0), Some(&"first"));


    let removed = set.shift_remove_index(2);
    assert_eq!(removed, Some("third"));
    assert_eq!(set.len(), 4);


    assert_eq!(set.get_index(0), Some(&"first"));
    assert_eq!(set.get_index(1), Some(&"second"));
    assert_eq!(set.get_index(2), Some(&"fourth"));
    assert_eq!(set.get_index(3), Some(&"fifth"));
}

#[test]
fn test_shift_remove_index_boundaries() {
    let mut set = IndexSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    set.insert(40);


    let removed = set.shift_remove_index(0);
    assert_eq!(removed, Some(10));
    assert_eq!(set.len(), 3);
    assert_eq!(set.get_index(0), Some(&20));


    let removed = set.shift_remove_index(2);
    assert_eq!(removed, Some(40));
    assert_eq!(set.len(), 2);
    assert_eq!(set.get_index(0), Some(&20));
    assert_eq!(set.get_index(1), Some(&30));


    let removed = set.shift_remove_index(5);
    assert_eq!(removed, None);
    assert_eq!(set.len(), 2);
}

#[test]
fn test_shift_remove_index_preserves_order() {
    let mut set = IndexSet::new();
    for i in 0..10 {
        set.insert(i * 10);
    }
    assert_eq!(set.len(), 10);


    let removed = set.shift_remove_index(3);
    assert_eq!(removed, Some(30));
    assert_eq!(set.len(), 9);


    let removed = set.shift_remove_index(5);
    assert_eq!(removed, Some(60));
    assert_eq!(set.len(), 8);


    assert_eq!(set.get_index(0), Some(&0));
    assert_eq!(set.get_index(1), Some(&10));
    assert_eq!(set.get_index(2), Some(&20));
    assert_eq!(set.get_index(3), Some(&40));
    assert_eq!(set.get_index(4), Some(&50));
}

#[test]
fn test_sort_then_binary_search_workflow() {
    let mut set = IndexSet::new();
    set.insert(42);
    set.insert(7);
    set.insert(99);
    set.insert(13);
    set.insert(55);
    set.insert(28);
    set.insert(3);
    set.insert(71);


    set.sort_unstable();


    assert_eq!(set.get_index(0), Some(&3));
    assert_eq!(set.get_index(7), Some(&99));


    assert_eq!(set.binary_search(&3), Ok(0));
    assert_eq!(set.binary_search(&42), Ok(4));
    assert_eq!(set.binary_search(&99), Ok(7));
    assert_eq!(set.binary_search(&50), Err(5));
    assert_eq!(set.binary_search(&100), Err(8));
}

#[test]
fn test_sort_remove_and_range_workflow() {
    let mut set = IndexSet::new();
    for i in (0..20).rev() {
        set.insert(i);
    }
    assert_eq!(set.len(), 20);

    set.sort_unstable();
    assert_eq!(set.get_index(0), Some(&0));
    assert_eq!(set.get_index(19), Some(&19));


    let slice = set.get_range(5..10).unwrap();
    assert_eq!(slice.len(), 5);
    assert_eq!(slice.first(), Some(&5));
    assert_eq!(slice.last(), Some(&9));


    let removed = set.shift_remove_index(0);
    assert_eq!(removed, Some(0));
    assert_eq!(set.get_index(0), Some(&1));


    let point = set.partition_point(|&x| x < 10);
    assert_eq!(point, 9);

    assert_eq!(set.len(), 19);
}

#[test]
fn test_empty_set_operations() {
    let mut set: IndexSet<i32> = IndexSet::new();

    set.sort_unstable();
    assert_eq!(set.len(), 0);

    set.sort_unstable_by(|a, b| a.cmp(b));
    assert_eq!(set.len(), 0);

    set.sort_by_cached_key(|x| *x);
    assert_eq!(set.len(), 0);

    let result = set.binary_search(&5);
    assert_eq!(result, Err(0));

    let point = set.partition_point(|&x| x < 5);
    assert_eq!(point, 0);

    let removed = set.shift_remove_index(0);
    assert_eq!(removed, None);

    let range = set.get_range(..);
    assert!(range.is_some());
    assert_eq!(range.unwrap().len(), 0);
}

#[test]
fn test_into_boxed_slice_empty() {
    let set: IndexSet<String> = IndexSet::new();
    let boxed = set.into_boxed_slice();
    assert_eq!(boxed.len(), 0);
    assert_eq!(boxed.first(), None);
    assert_eq!(boxed.last(), None);
    assert!(!boxed.iter().any(|x| *x == String::from("anything")));

    let iter_count = boxed.iter().count();
    assert_eq!(iter_count, 0);
    assert!(boxed.is_empty());
    assert_eq!(boxed.get_index(0), None::<&String>);
}

#[test]
fn test_sorted_unstable_by_into_iter_as_slice() {
    let mut set = IndexSet::new();
    set.insert(5);
    set.insert(2);
    set.insert(8);
    set.insert(1);
    set.insert(9);

    let mut iter = set.sorted_unstable_by(|a, b| a.cmp(b));


    let slice = iter.as_slice();
    assert_eq!(slice.len(), 5);
    assert_eq!(slice.first(), Some(&1));

    let first = iter.next();
    assert_eq!(first, Some(1));

    let slice = iter.as_slice();
    assert_eq!(slice.len(), 4);
    assert_eq!(slice.first(), Some(&2));

    let second = iter.next();
    assert_eq!(second, Some(2));

    let remaining: Vec<i32> = iter.collect();
    assert_eq!(remaining, vec![5, 8, 9]);
}