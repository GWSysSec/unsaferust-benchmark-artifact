use indexmap::IndexMap;
use indexmap::map::Slice;

fn build_sorted_map(n: usize) -> IndexMap<usize, String> {
    let mut map = IndexMap::new();
    for i in 0..n {
        map.insert(i * 10, format!("val_{}", i * 10));
    }
    map
}

#[test]
fn test_slice_new_mut_empty() {
    let empty: &mut Slice<String, i32> = Slice::new_mut();
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    assert_eq!(empty.first_mut(), None);
    assert_eq!(empty.last_mut(), None);
    assert!(empty.get_range(0..0).is_some());
    assert!(empty.get_range(0..1).is_none());
    assert!(empty.get_range_mut(0..0).is_some());
    assert!(empty.get_range_mut(0..1).is_none());
}

#[test]
fn test_slice_get_range_valid_and_invalid() {
    let map = build_sorted_map(5);
    let slice = map.as_slice();


    let sub = slice.get_range(1..3);
    assert!(sub.is_some());
    let sub = sub.unwrap();
    assert_eq!(sub.len(), 2);
    assert_eq!(sub[0], "val_10");
    assert_eq!(sub[1], "val_20");


    let full = slice.get_range(..);
    assert!(full.is_some());
    assert_eq!(full.unwrap().len(), 5);


    let empty = slice.get_range(2..2);
    assert!(empty.is_some());
    assert_eq!(empty.unwrap().len(), 0);


    let oob = slice.get_range(3..7);
    assert!(oob.is_none());
}

#[test]
fn test_slice_get_range_mut_modifications() {
    let mut map = build_sorted_map(6);
    let slice = map.as_mut_slice();


    let sub = slice.get_range_mut(2..5);
    assert!(sub.is_some());
    let sub = sub.unwrap();
    assert_eq!(sub.len(), 3);


    for (_, v) in sub.iter_mut() {
        *v = String::from("modified");
    }


    assert_eq!(slice[2], "modified");
    assert_eq!(slice[3], "modified");
    assert_eq!(slice[4], "modified");

    assert_eq!(slice[0], "val_0");
    assert_eq!(slice[1], "val_10");
    assert_eq!(slice[5], "val_50");
}

#[test]
fn test_slice_first_mut_and_last_mut() {
    let mut map = IndexMap::new();
    map.insert("alpha", 100);
    map.insert("beta", 200);
    map.insert("gamma", 300);

    let slice = map.as_mut_slice();


    let first = slice.first_mut();
    assert!(first.is_some());
    let (k, v) = first.unwrap();
    assert_eq!(*k, "alpha");
    assert_eq!(*v, 100);
    *v = 999;


    let last = slice.last_mut();
    assert!(last.is_some());
    let (k, v) = last.unwrap();
    assert_eq!(*k, "gamma");
    assert_eq!(*v, 300);
    *v = 888;


    assert_eq!(map["alpha"], 999);
    assert_eq!(map["gamma"], 888);
    assert_eq!(map["beta"], 200);
}

#[test]
fn test_slice_split_at() {
    let map = build_sorted_map(5);
    let slice = map.as_slice();

    let (left, right) = slice.split_at(2);
    assert_eq!(left.len(), 2);
    assert_eq!(right.len(), 3);


    assert_eq!(left[0], "val_0");
    assert_eq!(left[1], "val_10");


    assert_eq!(right[0], "val_20");
    assert_eq!(right[1], "val_30");
    assert_eq!(right[2], "val_40");


    let (l0, r0) = slice.split_at(0);
    assert_eq!(l0.len(), 0);
    assert_eq!(r0.len(), 5);


    let (lend, rend) = slice.split_at(5);
    assert_eq!(lend.len(), 5);
    assert_eq!(rend.len(), 0);
}

#[test]
fn test_slice_split_at_mut() {
    let mut map = build_sorted_map(4);
    let slice = map.as_mut_slice();

    let (left, right) = slice.split_at_mut(2);
    assert_eq!(left.len(), 2);
    assert_eq!(right.len(), 2);


    for (_, v) in left.iter_mut() {
        *v = String::from("left");
    }

    for (_, v) in right.iter_mut() {
        *v = String::from("right");
    }


    assert_eq!(map[&0], "left");
    assert_eq!(map[&10], "left");
    assert_eq!(map[&20], "right");
    assert_eq!(map[&30], "right");
}

#[test]
fn test_slice_split_first_and_split_last() {
    let map = build_sorted_map(4);
    let slice = map.as_slice();


    let result = slice.split_first();
    assert!(result.is_some());
    let ((k, v), rest) = result.unwrap();
    assert_eq!(*k, 0);
    assert_eq!(v, "val_0");
    assert_eq!(rest.len(), 3);
    assert_eq!(rest[0], "val_10");


    let result = slice.split_last();
    assert!(result.is_some());
    let ((k, v), rest) = result.unwrap();
    assert_eq!(*k, 30);
    assert_eq!(v, "val_30");
    assert_eq!(rest.len(), 3);
    assert_eq!(rest[2], "val_20");
}

#[test]
fn test_slice_split_first_mut_and_split_last_mut() {
    let mut map = build_sorted_map(3);
    let slice = map.as_mut_slice();


    let result = slice.split_first_mut();
    assert!(result.is_some());
    let ((k, v), rest) = result.unwrap();
    assert_eq!(*k, 0);
    assert_eq!(*v, "val_0");
    *v = String::from("first_modified");
    assert_eq!(rest.len(), 2);


    assert_eq!(map[&0], "first_modified");

    let slice = map.as_mut_slice();

    let result = slice.split_last_mut();
    assert!(result.is_some());
    let ((k, v), rest) = result.unwrap();
    assert_eq!(*k, 20);
    assert_eq!(*v, "val_20");
    *v = String::from("last_modified");
    assert_eq!(rest.len(), 2);

    assert_eq!(map[&20], "last_modified");
}

#[test]
fn test_slice_split_first_last_on_empty() {
    let map: IndexMap<i32, i32> = IndexMap::new();
    let slice = map.as_slice();

    assert!(slice.split_first().is_none());
    assert!(slice.split_last().is_none());

    let mut map2: IndexMap<i32, i32> = IndexMap::new();
    let slice_mut = map2.as_mut_slice();
    assert!(slice_mut.split_first_mut().is_none());
    assert!(slice_mut.split_last_mut().is_none());


    let mut single = IndexMap::new();
    single.insert(42, "only");
    let s = single.as_slice();
    let ((k, v), rest) = s.split_first().unwrap();
    assert_eq!(*k, 42);
    assert_eq!(*v, "only");
    assert_eq!(rest.len(), 0);

    let ((k2, v2), rest2) = s.split_last().unwrap();
    assert_eq!(*k2, 42);
    assert_eq!(*v2, "only");
    assert_eq!(rest2.len(), 0);
}

#[test]
fn test_slice_binary_search_by() {

    let mut map = IndexMap::new();
    for i in 0..10 {
        map.insert(i * 5, format!("v{}", i * 5));
    }

    let slice = map.as_slice();


    let result = slice.binary_search_by(|k, _| k.cmp(&20));
    assert_eq!(result, Ok(4));


    let result = slice.binary_search_by(|k, _| k.cmp(&0));
    assert_eq!(result, Ok(0));


    let result = slice.binary_search_by(|k, _| k.cmp(&45));
    assert_eq!(result, Ok(9));


    let result = slice.binary_search_by(|k, _| k.cmp(&12));
    assert_eq!(result, Err(3));


    let result = slice.binary_search_by(|k, _| k.cmp(&-1i32));
    assert_eq!(result, Err(0));


    let result = slice.binary_search_by(|k, _| k.cmp(&100));
    assert_eq!(result, Err(10));


    let result = slice.binary_search_by(|k, _| k.cmp(&7));
    assert_eq!(result, Err(2));


    let result = slice.binary_search_by(|k, _| k.cmp(&42));
    assert_eq!(result, Err(9));
}

#[test]
fn test_slice_binary_search_by_key() {

    let mut map = IndexMap::new();
    map.insert("a", 10);
    map.insert("b", 20);
    map.insert("c", 30);
    map.insert("d", 40);
    map.insert("e", 50);

    let slice = map.as_slice();


    let result = slice.binary_search_by_key(&30, |_, v| *v);
    assert_eq!(result, Ok(2));

    let result = slice.binary_search_by_key(&10, |_, v| *v);
    assert_eq!(result, Ok(0));

    let result = slice.binary_search_by_key(&50, |_, v| *v);
    assert_eq!(result, Ok(4));


    let result = slice.binary_search_by_key(&25, |_, v| *v);
    assert_eq!(result, Err(2));

    let result = slice.binary_search_by_key(&5, |_, v| *v);
    assert_eq!(result, Err(0));

    let result = slice.binary_search_by_key(&55, |_, v| *v);
    assert_eq!(result, Err(5));

    let result = slice.binary_search_by_key(&35, |_, v| *v);
    assert_eq!(result, Err(3));

    let result = slice.binary_search_by_key(&45, |_, v| *v);
    assert_eq!(result, Err(4));
}

#[test]
fn test_slice_partition_point() {
    let mut map = IndexMap::new();
    for i in 0..8 {
        map.insert(i * 3, i * 3 * 2);
    }


    let slice = map.as_slice();


    let pp = slice.partition_point(|k, _| *k < 10);
    assert_eq!(pp, 4);


    let pp = slice.partition_point(|k, _| *k < 100);
    assert_eq!(pp, 8);


    let pp = slice.partition_point(|_, _| false);
    assert_eq!(pp, 0);


    let pp = slice.partition_point(|_, v| *v < 24);
    assert_eq!(pp, 4);


    let pp = slice.partition_point(|k, _| *k <= 9);
    assert_eq!(pp, 4);

    let pp = slice.partition_point(|k, _| *k < 0);
    assert_eq!(pp, 0);

    let pp = slice.partition_point(|k, _| *k <= 21);
    assert_eq!(pp, 8);

    let pp = slice.partition_point(|k, _| *k < 21);
    assert_eq!(pp, 7);
}

#[test]
fn test_slice_combined_workflow() {
    let mut map = IndexMap::new();
    for i in 0..10 {
        map.insert(i, (i as f64) * 1.5);
    }

    let slice = map.as_mut_slice();


    let sub = slice.get_range_mut(3..7).unwrap();
    assert_eq!(sub.len(), 4);
    for (_, v) in sub.iter_mut() {
        *v *= 2.0;
    }


    let slice = map.as_slice();
    let (left, right) = slice.split_at(3);
    assert_eq!(left.len(), 3);

    assert_eq!(map[&0], 0.0);
    assert_eq!(map[&2], 3.0);

    assert_eq!(map[&3], 9.0);
    assert_eq!(map[&6], 18.0);

    assert_eq!(map[&7], 10.5);
    assert_eq!(right.len(), 7);


    let found = slice.binary_search_by(|k, _| k.cmp(&5));
    assert_eq!(found, Ok(5));
}

#[test]
fn test_new_mut_slice_operations() {
    let empty: &mut Slice<u32, u32> = Slice::new_mut();
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());


    let (l, r) = empty.split_at(0);
    assert_eq!(l.len(), 0);
    assert_eq!(r.len(), 0);


    let range = empty.get_range(..);
    assert!(range.is_some());
    assert_eq!(range.unwrap().len(), 0);


    let pp = empty.partition_point(|_, _| true);
    assert_eq!(pp, 0);

    let pp2 = empty.partition_point(|_, _| false);
    assert_eq!(pp2, 0);


    let result = empty.binary_search_by(|k, _| k.cmp(&42));
    assert_eq!(result, Err(0));
}