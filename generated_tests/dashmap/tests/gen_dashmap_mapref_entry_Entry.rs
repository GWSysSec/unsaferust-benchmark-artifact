use dashmap::DashMap;

#[test]
fn test_entry_or_insert_and_and_modify() {
    let map: DashMap<String, i64> = DashMap::new();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());


    {
        let mut r = map.entry("a".to_string()).or_insert(10);
        assert_eq!(*r.value(), 10);
        *r.value_mut() += 5;
    }
    assert_eq!(map.len(), 1);
    assert_eq!(*map.get("a").unwrap().value(), 15);


    {
        let r = map.entry("a".to_string()).or_insert(999);
        assert_eq!(*r.value(), 15);
    }
    assert_eq!(*map.get("a").unwrap().value(), 15);


    map.entry("a".to_string())
        .and_modify(|v| *v *= 2)
        .or_insert(0);
    assert_eq!(*map.get("a").unwrap().value(), 30);


    let mut called = false;
    map.entry("b".to_string())
        .and_modify(|v| {
            called = true;
            *v += 1;
        })
        .or_insert(7);
    assert_eq!(called, false);
    assert_eq!(*map.get("b").unwrap().value(), 7);
    assert_eq!(map.len(), 2);
}

#[test]
fn test_entry_or_insert_with_and_or_default() {
    let map: DashMap<&'static str, Vec<i32>> = DashMap::new();
    assert_eq!(map.len(), 0);


    {
        let r = map.entry("nums").or_insert_with(|| vec![1, 2, 3]);
        assert_eq!(r.value().len(), 3);
        assert_eq!(r.value()[0], 1);
        assert_eq!(r.value()[2], 3);
    }
    assert_eq!(map.len(), 1);


    let mut closure_called = false;
    {
        let r = map.entry("nums").or_insert_with(|| {
            closure_called = true;
            vec![99]
        });
        assert_eq!(r.value().len(), 3);
    }
    assert_eq!(closure_called, false);


    let map2: DashMap<i32, String> = DashMap::new();
    {
        let r = map2.entry(42).or_default();
        assert_eq!(r.value(), &String::new());
        assert_eq!(r.value().len(), 0);
    }
    assert_eq!(map2.len(), 1);
    assert!(map2.contains_key(&42));


    map2.insert(7, "hello".to_string());
    {
        let r = map2.entry(7).or_default();
        assert_eq!(r.value(), "hello");
        assert_eq!(r.value().len(), 5);
    }
    assert_eq!(map2.len(), 2);
}

#[test]
fn test_entry_or_try_insert_with_success_and_failure() {
    let map: DashMap<String, i32> = DashMap::new();


    let res: Result<_, &'static str> = map
        .entry("ok".to_string())
        .or_try_insert_with(|| Ok(100));
    assert!(res.is_ok());
    {
        let r = res.unwrap();
        assert_eq!(*r.value(), 100);
    }
    assert_eq!(map.len(), 1);
    assert_eq!(*map.get("ok").unwrap().value(), 100);


    let res2: Result<_, &'static str> = map
        .entry("bad".to_string())
        .or_try_insert_with(|| Err("nope"));
    assert!(res2.is_err());
    assert_eq!(res2.err().unwrap(), "nope");
    assert_eq!(map.len(), 1);
    assert!(!map.contains_key("bad"));


    let mut called = false;
    let res3: Result<_, &'static str> = map.entry("ok".to_string()).or_try_insert_with(|| {
        called = true;
        Ok(0)
    });
    assert!(res3.is_ok());
    assert_eq!(called, false);
    {
        let r = res3.unwrap();
        assert_eq!(*r.value(), 100);
    }
    assert_eq!(map.len(), 1);
}

#[test]
fn test_entry_into_key() {
    let map: DashMap<String, u32> = DashMap::new();
    map.insert("present".to_string(), 1);
    assert_eq!(map.len(), 1);


    let k1 = map.entry("absent".to_string()).into_key();
    assert_eq!(k1, "absent");
    assert_eq!(k1.len(), 6);

    assert_eq!(map.len(), 1);
    assert!(!map.contains_key("absent"));


    let k2 = map.entry("present".to_string()).into_key();
    assert_eq!(k2, "present");
    assert_eq!(k2.len(), 7);
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("present"));
}

#[test]
fn test_entry_combined_workflow() {

    let counts: DashMap<&'static str, u64> = DashMap::new();
    let words = ["a", "b", "a", "c", "a", "b", "d"];

    for w in &words {
        counts
            .entry(*w)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }

    assert_eq!(counts.len(), 4);
    assert_eq!(*counts.get("a").unwrap().value(), 3);
    assert_eq!(*counts.get("b").unwrap().value(), 2);
    assert_eq!(*counts.get("c").unwrap().value(), 1);
    assert_eq!(*counts.get("d").unwrap().value(), 1);


    let parse_value = |s: &str| -> Result<u64, std::num::ParseIntError> { s.parse::<u64>() };

    let res_ok = counts
        .entry("e")
        .or_try_insert_with(|| parse_value("42"));
    assert!(res_ok.is_ok());
    drop(res_ok);
    assert_eq!(*counts.get("e").unwrap().value(), 42);

    let res_err = counts
        .entry("f")
        .or_try_insert_with(|| parse_value("not-a-number"));
    assert!(res_err.is_err());
    assert!(!counts.contains_key("f"));
    assert_eq!(counts.len(), 5);


    {
        let r = counts.entry("g").or_default();
        assert_eq!(*r.value(), 0u64);
    }
    assert_eq!(counts.len(), 6);
    assert_eq!(*counts.get("g").unwrap().value(), 0);
}