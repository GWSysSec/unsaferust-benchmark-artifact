
use indexmap::IndexSet;
use std::collections::hash_map::RandomState;

#[test]
fn test_hasher_returns_reference() {
    let set: IndexSet<i32, RandomState> = IndexSet::new();
    let _hasher = set.hasher();

    let set2: IndexSet<i32> = IndexSet::with_hasher(RandomState::new());
    let _h2 = set2.hasher();
    assert_eq!(set.len(), 0);
    assert_eq!(set2.len(), 0);
    assert!(set.is_empty());
    assert!(set2.is_empty());

    let mut set3: IndexSet<String> = IndexSet::new();
    set3.insert("hello".to_string());
    set3.insert("world".to_string());
    let _h3 = set3.hasher();
    assert_eq!(set3.len(), 2);
    assert!(set3.contains("hello"));
    assert!(set3.contains("world"));
}

#[test]
fn test_clear_removes_all_elements() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    set.insert(40);

    assert_eq!(set.len(), 4);
    assert!(set.contains(&10));
    assert!(set.contains(&20));
    assert!(set.contains(&30));
    assert!(set.contains(&40));

    set.clear();

    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
    assert!(!set.contains(&10));
    assert!(!set.contains(&20));
    assert!(!set.contains(&30));
    assert!(!set.contains(&40));
}

#[test]
fn test_truncate_shortens_set() {
    let mut set: IndexSet<i32> = IndexSet::new();
    for i in 0..10 {
        set.insert(i);
    }

    assert_eq!(set.len(), 10);
    assert!(set.contains(&9));

    set.truncate(5);

    assert_eq!(set.len(), 5);
    assert!(set.contains(&0));
    assert!(set.contains(&4));
    assert!(!set.contains(&5));
    assert!(!set.contains(&9));


    set.truncate(0);
    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
}

#[test]
fn test_truncate_larger_than_len_is_noop() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    assert_eq!(set.len(), 3);
    set.truncate(100);
    assert_eq!(set.len(), 3);
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));

    assert_eq!(set.get_index(0), Some(&1));
    assert_eq!(set.get_index(1), Some(&2));
    assert_eq!(set.get_index(2), Some(&3));
}

#[test]
fn test_split_off_divides_set() {
    let mut set: IndexSet<&str> = IndexSet::new();
    set.insert("a");
    set.insert("b");
    set.insert("c");
    set.insert("d");
    set.insert("e");

    assert_eq!(set.len(), 5);

    let split = set.split_off(3);

    assert_eq!(set.len(), 3);
    assert_eq!(split.len(), 2);

    assert!(set.contains("a"));
    assert!(set.contains("b"));
    assert!(set.contains("c"));
    assert!(!set.contains("d"));
    assert!(!set.contains("e"));

    assert!(!split.contains("a"));
    assert!(split.contains("d"));
    assert!(split.contains("e"));
}

#[test]
fn test_split_off_at_zero() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(100);
    set.insert(200);
    set.insert(300);

    let split = set.split_off(0);

    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
    assert_eq!(split.len(), 3);
    assert!(split.contains(&100));
    assert!(split.contains(&200));
    assert!(split.contains(&300));
    assert_eq!(split.get_index(0), Some(&100));
    assert_eq!(split.get_index(2), Some(&300));
}

#[test]
fn test_reserve_exact() {
    let mut set: IndexSet<i32> = IndexSet::new();
    assert_eq!(set.len(), 0);

    set.reserve_exact(100);
    let cap = set.capacity();
    assert!(cap >= 100);


    for i in 0..50 {
        set.insert(i);
    }
    assert_eq!(set.len(), 50);
    assert!(set.contains(&0));
    assert!(set.contains(&49));
    assert!(!set.contains(&50));
    assert!(set.capacity() >= 100);
}

#[test]
fn test_try_reserve_exact_success() {
    let mut set: IndexSet<i32> = IndexSet::new();
    let result = set.try_reserve_exact(50);
    assert!(result.is_ok());
    assert!(set.capacity() >= 50);

    for i in 0..50 {
        set.insert(i);
    }
    assert_eq!(set.len(), 50);
    assert!(set.contains(&0));
    assert!(set.contains(&49));


    let result2 = set.try_reserve_exact(10);
    assert!(result2.is_ok());
    assert!(set.capacity() >= 60);
}

#[test]
fn test_try_reserve_exact_overflow() {
    let mut set: IndexSet<i32> = IndexSet::new();

    let result = set.try_reserve_exact(usize::MAX / 2);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let cloned_err = err.clone();

    let err_str = format!("{:?}", cloned_err);
    assert!(!err_str.is_empty());

    set.insert(42);
    assert_eq!(set.len(), 1);
    assert!(set.contains(&42));
}

#[test]
fn test_splice_replace_range() {
    let mut set: IndexSet<i32> = IndexSet::new();
    for i in 0..5 {
        set.insert(i * 10);
    }

    assert_eq!(set.len(), 5);
    assert_eq!(set.get_index(0), Some(&0));
    assert_eq!(set.get_index(4), Some(&40));


    let removed: Vec<i32> = set.splice(1..3, vec![100, 200, 300]).collect();

    assert_eq!(removed.len(), 2);
    assert_eq!(removed[0], 10);
    assert_eq!(removed[1], 20);


    assert!(set.contains(&0));
    assert!(set.contains(&100));
    assert!(set.contains(&200));
    assert!(set.contains(&300));
    assert!(set.contains(&30));
    assert!(!set.contains(&10));
    assert!(!set.contains(&20));
}

#[test]
fn test_splice_empty_replacement() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);
    set.insert(4);
    set.insert(5);

    assert_eq!(set.len(), 5);


    let removed: Vec<i32> = set.splice(1..4, std::iter::empty()).collect();

    assert_eq!(removed.len(), 3);
    assert_eq!(removed[0], 2);
    assert_eq!(removed[1], 3);
    assert_eq!(removed[2], 4);
    assert_eq!(set.len(), 2);
    assert!(set.contains(&1));
    assert!(set.contains(&5));
}

#[test]
fn test_append_merges_sets() {
    let mut set1: IndexSet<i32> = IndexSet::new();
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);

    let mut set2: IndexSet<i32> = IndexSet::new();
    set2.insert(3);
    set2.insert(4);
    set2.insert(5);

    assert_eq!(set1.len(), 3);
    assert_eq!(set2.len(), 3);

    set1.append(&mut set2);


    assert_eq!(set1.len(), 5);
    assert!(set1.contains(&1));
    assert!(set1.contains(&2));
    assert!(set1.contains(&3));
    assert!(set1.contains(&4));
    assert!(set1.contains(&5));


    assert_eq!(set2.len(), 0);
    assert!(set2.is_empty());
}

#[test]
fn test_take_removes_and_returns_value() {
    let mut set: IndexSet<String> = IndexSet::new();
    set.insert("alpha".to_string());
    set.insert("beta".to_string());
    set.insert("gamma".to_string());
    set.insert("delta".to_string());

    assert_eq!(set.len(), 4);
    assert!(set.contains("beta"));

    let taken = set.take("beta");
    assert_eq!(taken, Some("beta".to_string()));
    assert_eq!(set.len(), 3);
    assert!(!set.contains("beta"));


    let not_found = set.take("omega");
    assert_eq!(not_found, None);
    assert_eq!(set.len(), 3);


    assert!(set.contains("alpha"));
    assert!(set.contains("gamma"));
    assert!(set.contains("delta"));
}

#[test]
fn test_swap_take_removes_value() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    set.insert(40);
    set.insert(50);

    assert_eq!(set.len(), 5);
    assert_eq!(set.get_index(1), Some(&20));


    let taken = set.swap_take(&20);
    assert_eq!(taken, Some(20));
    assert_eq!(set.len(), 4);
    assert!(!set.contains(&20));


    assert_eq!(set.get_index(1), Some(&50));
    assert!(set.contains(&10));
    assert!(set.contains(&30));
    assert!(set.contains(&40));
    assert!(set.contains(&50));


    let missing = set.swap_take(&999);
    assert_eq!(missing, None);
}

#[test]
fn test_shift_take_removes_value_preserving_order() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    set.insert(40);
    set.insert(50);

    assert_eq!(set.len(), 5);


    let taken = set.shift_take(&20);
    assert_eq!(taken, Some(20));
    assert_eq!(set.len(), 4);
    assert!(!set.contains(&20));


    assert_eq!(set.get_index(0), Some(&10));
    assert_eq!(set.get_index(1), Some(&30));
    assert_eq!(set.get_index(2), Some(&40));
    assert_eq!(set.get_index(3), Some(&50));


    let missing = set.shift_take(&999);
    assert_eq!(missing, None);
    assert_eq!(set.len(), 4);
}

#[test]
fn test_retain_keeps_matching_elements() {
    let mut set: IndexSet<i32> = IndexSet::new();
    for i in 0..10 {
        set.insert(i);
    }

    assert_eq!(set.len(), 10);


    set.retain(|&x| x % 2 == 0);

    assert_eq!(set.len(), 5);
    assert!(set.contains(&0));
    assert!(set.contains(&2));
    assert!(set.contains(&4));
    assert!(set.contains(&6));
    assert!(set.contains(&8));
    assert!(!set.contains(&1));
    assert!(!set.contains(&3));
    assert!(!set.contains(&5));
    assert!(!set.contains(&7));
    assert!(!set.contains(&9));
}

#[test]
fn test_retain_removes_all() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    assert_eq!(set.len(), 3);

    set.retain(|_| false);

    assert_eq!(set.len(), 0);
    assert!(set.is_empty());
    assert!(!set.contains(&1));
    assert!(!set.contains(&2));
    assert!(!set.contains(&3));

    assert!(set.capacity() >= 3);
}

#[test]
fn test_sort_orders_elements() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(50);
    set.insert(10);
    set.insert(40);
    set.insert(20);
    set.insert(30);

    assert_eq!(set.get_index(0), Some(&50));
    assert_eq!(set.get_index(1), Some(&10));

    set.sort();

    assert_eq!(set.len(), 5);
    assert_eq!(set.get_index(0), Some(&10));
    assert_eq!(set.get_index(1), Some(&20));
    assert_eq!(set.get_index(2), Some(&30));
    assert_eq!(set.get_index(3), Some(&40));
    assert_eq!(set.get_index(4), Some(&50));
}

#[test]
fn test_sort_by_custom_comparator() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(1);
    set.insert(5);
    set.insert(3);
    set.insert(2);
    set.insert(4);


    set.sort_by(|a, b| b.cmp(a));

    assert_eq!(set.len(), 5);
    assert_eq!(set.get_index(0), Some(&5));
    assert_eq!(set.get_index(1), Some(&4));
    assert_eq!(set.get_index(2), Some(&3));
    assert_eq!(set.get_index(3), Some(&2));
    assert_eq!(set.get_index(4), Some(&1));
}

#[test]
fn test_sorted_by_returns_sorted_iter() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(30);
    set.insert(10);
    set.insert(50);
    set.insert(20);
    set.insert(40);

    assert_eq!(set.get_index(0), Some(&30));


    let sorted_items: Vec<i32> = set.sorted_by(|a, b| a.cmp(b)).collect();

    assert_eq!(sorted_items.len(), 5);
    assert_eq!(sorted_items[0], 10);
    assert_eq!(sorted_items[1], 20);
    assert_eq!(sorted_items[2], 30);
    assert_eq!(sorted_items[3], 40);
    assert_eq!(sorted_items[4], 50);
}

#[test]
fn test_sorted_by_descending() {
    let mut set: IndexSet<String> = IndexSet::new();
    set.insert("cherry".to_string());
    set.insert("apple".to_string());
    set.insert("banana".to_string());
    set.insert("date".to_string());

    let sorted: Vec<String> = set.sorted_by(|a, b| b.cmp(a)).collect();

    assert_eq!(sorted.len(), 4);
    assert_eq!(sorted[0], "date");
    assert_eq!(sorted[1], "cherry");
    assert_eq!(sorted[2], "banana");
    assert_eq!(sorted[3], "apple");
}

#[test]
fn test_combined_workflow_clear_and_reuse() {
    let mut set: IndexSet<i32> = IndexSet::new();
    for i in 0..20 {
        set.insert(i);
    }
    assert_eq!(set.len(), 20);

    set.clear();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);


    for i in 100..110 {
        set.insert(i);
    }
    assert_eq!(set.len(), 10);
    assert!(set.contains(&100));
    assert!(set.contains(&109));
    assert!(!set.contains(&0));
    assert!(!set.contains(&19));
}

#[test]
fn test_combined_workflow_truncate_sort_retain() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(99);
    set.insert(3);
    set.insert(55);
    set.insert(7);
    set.insert(42);
    set.insert(1);
    set.insert(88);
    set.insert(15);

    assert_eq!(set.len(), 8);


    set.sort();
    assert_eq!(set.get_index(0), Some(&1));
    assert_eq!(set.get_index(7), Some(&99));


    set.truncate(6);
    assert_eq!(set.len(), 6);
    assert!(!set.contains(&88));
    assert!(!set.contains(&99));


    set.retain(|&x| x > 5);
    assert!(!set.contains(&1));
    assert!(!set.contains(&3));
    assert!(set.contains(&7));
    assert!(set.contains(&15));
    assert!(set.contains(&42));
    assert!(set.contains(&55));
    assert_eq!(set.len(), 4);
}

#[test]
fn test_combined_split_off_and_append() {
    let mut set: IndexSet<i32> = IndexSet::new();
    for i in 0..10 {
        set.insert(i);
    }


    let mut second_half = set.split_off(5);
    assert_eq!(set.len(), 5);
    assert_eq!(second_half.len(), 5);


    second_half.insert(100);
    second_half.insert(200);
    assert_eq!(second_half.len(), 7);


    set.append(&mut second_half);
    assert_eq!(set.len(), 12);
    assert!(second_half.is_empty());
    assert!(set.contains(&0));
    assert!(set.contains(&9));
    assert!(set.contains(&100));
    assert!(set.contains(&200));
}

#[test]
fn test_swap_take_vs_shift_take_ordering() {

    let mut set_swap: IndexSet<i32> = IndexSet::new();
    let mut set_shift: IndexSet<i32> = IndexSet::new();

    for i in 1..=5 {
        set_swap.insert(i * 10);
        set_shift.insert(i * 10);
    }


    let sw = set_swap.swap_take(&20);
    assert_eq!(sw, Some(20));

    assert_eq!(set_swap.get_index(0), Some(&10));
    assert_eq!(set_swap.get_index(1), Some(&50));
    assert_eq!(set_swap.get_index(2), Some(&30));
    assert_eq!(set_swap.get_index(3), Some(&40));


    let sh = set_shift.shift_take(&20);
    assert_eq!(sh, Some(20));

    assert_eq!(set_shift.get_index(0), Some(&10));
    assert_eq!(set_shift.get_index(1), Some(&30));
    assert_eq!(set_shift.get_index(2), Some(&40));
    assert_eq!(set_shift.get_index(3), Some(&50));
}

#[test]
fn test_splice_with_duplicates_in_replacement() {
    let mut set: IndexSet<i32> = IndexSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);
    set.insert(4);
    set.insert(5);

    assert_eq!(set.len(), 5);



    let removed: Vec<i32> = set.splice(1..3, vec![10, 10, 1, 20]).collect();

    assert_eq!(removed.len(), 2);
    assert_eq!(removed[0], 2);
    assert_eq!(removed[1], 3);


    assert!(set.contains(&1));
    assert!(set.contains(&10));
    assert!(set.contains(&20));
    assert!(set.contains(&4));
    assert!(set.contains(&5));
    assert!(!set.contains(&2));
    assert!(!set.contains(&3));
}

#[test]
fn test_sort_by_string_length() {
    let mut set: IndexSet<String> = IndexSet::new();
    set.insert("hi".to_string());
    set.insert("hello".to_string());
    set.insert("hey".to_string());
    set.insert("a".to_string());
    set.insert("wonderful".to_string());


    set.sort_by(|a, b| a.len().cmp(&b.len()));

    assert_eq!(set.get_index(0), Some(&"a".to_string()));
    assert_eq!(set.get_index(1), Some(&"hi".to_string()));
    assert_eq!(set.get_index(2), Some(&"hey".to_string()));
    assert_eq!(set.get_index(3), Some(&"hello".to_string()));
    assert_eq!(set.get_index(4), Some(&"wonderful".to_string()));
    assert_eq!(set.len(), 5);
    assert!(set.contains("a"));
    assert!(set.contains("wonderful"));
}