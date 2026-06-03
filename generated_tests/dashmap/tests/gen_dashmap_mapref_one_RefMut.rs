use dashmap::DashMap;
use dashmap::mapref::one::{Ref, RefMut, MappedRefMut};

#[test]
fn test_refmut_downgrade() {
    let map: DashMap<String, Vec<i32>> = DashMap::new();
    map.insert("a".to_string(), vec![1, 2, 3]);
    map.insert("b".to_string(), vec![4, 5, 6]);

    let mut rm: RefMut<String, Vec<i32>> = map.get_mut("a").expect("present");
    assert_eq!(rm.key(), "a");
    rm.value_mut().push(99);
    assert_eq!(rm.value().len(), 4);
    assert_eq!(rm.value()[3], 99);

    let r: Ref<String, Vec<i32>> = rm.downgrade();
    assert_eq!(r.key(), "a");
    assert_eq!(r.value().len(), 4);
    assert_eq!(r.value()[0], 1);
    assert_eq!(r.value()[3], 99);


    let other = map.get("b").expect("b present");
    assert_eq!(other.value().len(), 3);
    assert_eq!(other.value()[0], 4);
    drop(other);
    drop(r);


    let rm2 = map.get_mut("a").expect("still present");
    assert_eq!(rm2.value()[3], 99);
}

#[test]
fn test_refmut_map() {
    let map: DashMap<&'static str, (i64, String)> = DashMap::new();
    map.insert("k1", (10, "hello".to_string()));
    map.insert("k2", (20, "world".to_string()));

    let rm = map.get_mut("k1").expect("k1");
    assert_eq!(rm.value().0, 10);

    let mut mapped: MappedRefMut<&'static str, (i64, String), String> =
        rm.map(|v| &mut v.1);
    assert_eq!(mapped.key(), &"k1");
    assert_eq!(mapped.value(), "hello");
    mapped.value_mut().push_str(" there");
    assert_eq!(mapped.value(), "hello there");
    drop(mapped);

    let after = map.get("k1").expect("still present");
    assert_eq!(after.value().0, 10);
    assert_eq!(after.value().1, "hello there");
    drop(after);

    let untouched = map.get("k2").expect("k2");
    assert_eq!(untouched.value().0, 20);
    assert_eq!(untouched.value().1, "world");
}

#[test]
fn test_refmut_try_map_success_and_failure() {
    let map: DashMap<i32, Option<Vec<u8>>> = DashMap::new();
    map.insert(1, Some(vec![1, 2, 3]));
    map.insert(2, None);


    let rm_ok = map.get_mut(&1).expect("entry 1");
    let mapped_res = rm_ok.try_map(|v| v.as_mut());
    assert!(mapped_res.is_ok());
    let mut mapped = mapped_res.ok().expect("ok");
    assert_eq!(mapped.key(), &1);
    assert_eq!(mapped.value().len(), 3);
    mapped.value_mut().push(4);
    assert_eq!(mapped.value().len(), 4);
    assert_eq!(mapped.value()[3], 4);
    drop(mapped);


    let rm_err = map.get_mut(&2).expect("entry 2");
    let res = rm_err.try_map(|v| v.as_mut());
    assert!(res.is_err());
    let original = res.err().expect("err");
    assert_eq!(original.key(), &2);
    assert!(original.value().is_none());
    drop(original);


    let after = map.get(&1).expect("k1");
    let inner = after.value().as_ref().expect("some");
    assert_eq!(inner.len(), 4);
    assert_eq!(inner[3], 4);
}