use serde_yaml::{Mapping, Value};

fn parse_map(yaml: &str) -> Mapping {
    serde_yaml::from_str::<Mapping>(yaml).expect("valid yaml mapping")
}

fn parse_val(yaml: &str) -> Value {
    serde_yaml::from_str::<Value>(yaml).expect("valid yaml value")
}

#[test]
fn test_with_capacity_reserve_shrink_cycle() {
    let mut m = Mapping::with_capacity(128);
    assert_eq!(m.is_empty(), true);
    let c1 = m.capacity();
    assert!(c1 >= 128, "with_capacity did not honor request: got {}", c1);

    m.reserve(500);
    let c2 = m.capacity();
    assert!(c2 >= 500, "capacity after reserve({}) was {}", 500, c2);
    assert!(c2 >= c1);

    m.shrink_to_fit();
    let c3 = m.capacity();
    assert!(c3 <= c2, "shrink_to_fit grew capacity: {} -> {}", c2, c3);
    assert_eq!(m.is_empty(), true);

    let m0 = Mapping::with_capacity(0);
    assert_eq!(m0.is_empty(), true);
    assert!(m0.capacity() <= c1);


    let before = m.capacity();
    m.reserve(1);
    assert!(m.capacity() >= before);
}

#[test]
fn test_contains_key_and_get_multiple_types() {
    let m = parse_map("alpha: 1\nbeta: 2\ngamma: 3\nactive: true\nname: zed");

    assert_eq!(m.is_empty(), false);
    assert_eq!(m.contains_key("alpha"), true);
    assert_eq!(m.contains_key("beta"), true);
    assert_eq!(m.contains_key("gamma"), true);
    assert_eq!(m.contains_key("missing"), false);
    assert_eq!(m.contains_key("ALPHA"), false);

    let alpha = m.get("alpha").expect("alpha present");
    assert_eq!(alpha, &parse_val("1"));

    let name = m.get("name").expect("name present");
    assert_eq!(name, &parse_val("zed"));

    let active = m.get("active").expect("active present");
    assert_eq!(active, &parse_val("true"));

    assert_eq!(m.get("nope").is_none(), true);


    let k: Value = parse_val("beta");
    assert_eq!(m.contains_key(&k), true);
    let k_missing: Value = parse_val("ghost");
    assert_eq!(m.contains_key(&k_missing), false);
}

#[test]
fn test_get_mut_modifies_entries() {
    let mut m = parse_map("count: 10\nlabel: initial\nflag: false");

    assert_eq!(m.is_empty(), false);
    let before_count = m.get("count").cloned().expect("has count");
    assert_eq!(before_count, parse_val("10"));

    {
        let count_ref = m.get_mut("count").expect("count mut");
        *count_ref = parse_val("999");
    }

    let after_count = m.get("count").expect("still there");
    assert_eq!(after_count, &parse_val("999"));
    assert_ne!(after_count, &before_count);

    {
        let label_ref = m.get_mut("label").expect("label mut");
        *label_ref = parse_val("updated");
    }
    assert_eq!(m.get("label").unwrap(), &parse_val("updated"));

    assert_eq!(m.get_mut("missing").is_none(), true);
    assert_eq!(m.get("flag").unwrap(), &parse_val("false"));
    assert_eq!(m.contains_key("count"), true);
}

#[test]
fn test_shift_remove_preserves_order() {
    let mut m = parse_map("a: 1\nb: 2\nc: 3\nd: 4\ne: 5");
    let initial_capacity = m.capacity();
    assert!(initial_capacity >= 5);
    assert_eq!(m.is_empty(), false);
    assert_eq!(m.contains_key("b"), true);
    assert_eq!(m.contains_key("c"), true);

    let removed = m.shift_remove("c").expect("c was present");
    assert_eq!(removed, parse_val("3"));
    assert_eq!(m.contains_key("c"), false);
    assert_eq!(m.contains_key("a"), true);
    assert_eq!(m.contains_key("e"), true);

    let (k, v) = m.shift_remove_entry("b").expect("b present");
    assert_eq!(k, parse_val("b"));
    assert_eq!(v, parse_val("2"));
    assert_eq!(m.contains_key("b"), false);

    assert_eq!(m.shift_remove("zzz").is_none(), true);
    assert_eq!(m.shift_remove_entry("zzz").is_none(), true);


    let out = serde_yaml::to_string(&m).expect("to_string");
    let a_idx = out.find("a:").expect("has a");
    let d_idx = out.find("d:").expect("has d");
    let e_idx = out.find("e:").expect("has e");
    assert!(a_idx < d_idx, "a must precede d in {:?}", out);
    assert!(d_idx < e_idx, "d must precede e in {:?}", out);
}

#[test]
fn test_swap_remove_and_remove_entry() {
    let mut m = parse_map("x: 10\ny: 20\nz: 30\nw: 40");
    assert_eq!(m.is_empty(), false);
    assert_eq!(m.contains_key("y"), true);
    assert_eq!(m.contains_key("w"), true);

    let (k, v) = m.swap_remove_entry("y").expect("y present");
    assert_eq!(k, parse_val("y"));
    assert_eq!(v, parse_val("20"));
    assert_eq!(m.contains_key("y"), false);
    assert_eq!(m.contains_key("x"), true);
    assert_eq!(m.contains_key("z"), true);
    assert_eq!(m.contains_key("w"), true);

    assert_eq!(m.swap_remove_entry("missing").is_none(), true);

    let (k2, v2) = m.remove_entry("x").expect("x present");
    assert_eq!(k2, parse_val("x"));
    assert_eq!(v2, parse_val("10"));
    assert_eq!(m.contains_key("x"), false);
    assert_eq!(m.remove_entry("nope").is_none(), true);


    assert_eq!(m.is_empty(), false);
    assert_eq!(m.get("z").unwrap(), &parse_val("30"));
    assert_eq!(m.get("w").unwrap(), &parse_val("40"));
}

#[test]
fn test_retain_keeps_filtered_entries() {
    let mut m = parse_map("one: 1\ntwo: 2\nthree: 3\nfour: 4\nfive: 5\nsix: 6");
    assert_eq!(m.is_empty(), false);
    let before_cap = m.capacity();
    assert!(before_cap >= 6);


    m.retain(|_k, v| {
        let n: i64 = serde_yaml::from_value(v.clone()).unwrap_or(-1);
        n % 2 == 0
    });

    assert_eq!(m.contains_key("two"), true);
    assert_eq!(m.contains_key("four"), true);
    assert_eq!(m.contains_key("six"), true);
    assert_eq!(m.contains_key("one"), false);
    assert_eq!(m.contains_key("three"), false);
    assert_eq!(m.contains_key("five"), false);
    assert_eq!(m.is_empty(), false);

    m.retain(|_, _| true);
    assert_eq!(m.contains_key("two"), true);
    assert_eq!(m.contains_key("four"), true);
    assert_eq!(m.contains_key("six"), true);

    m.retain(|_, _| false);
    assert_eq!(m.is_empty(), true);
    assert_eq!(m.contains_key("two"), false);
}

#[test]
fn test_iter_mut_bulk_update() {
    let mut m = parse_map("a: 1\nb: 2\nc: 3\nd: 4");
    assert_eq!(m.is_empty(), false);

    let count = m.iter_mut().len();
    assert_eq!(count, 4);


    for (_k, v) in m.iter_mut() {
        let n: i64 = serde_yaml::from_value(v.clone()).expect("int value");
        *v = parse_val(&format!("{}", n * 2));
    }

    assert_eq!(m.get("a").unwrap(), &parse_val("2"));
    assert_eq!(m.get("b").unwrap(), &parse_val("4"));
    assert_eq!(m.get("c").unwrap(), &parse_val("6"));
    assert_eq!(m.get("d").unwrap(), &parse_val("8"));
    assert_eq!(m.contains_key("a"), true);
    assert_eq!(m.contains_key("missing"), false);


    assert_eq!(m.iter_mut().len(), 4);
}

#[test]
fn test_clear_then_reuse_via_parse() {
    let mut m = parse_map("x: hello\ny: world\nz: !!str 42");
    assert_eq!(m.is_empty(), false);
    assert_eq!(m.contains_key("x"), true);
    assert_eq!(m.contains_key("z"), true);
    let cap_before = m.capacity();
    assert!(cap_before >= 3);

    m.clear();
    assert_eq!(m.is_empty(), true);
    assert_eq!(m.contains_key("x"), false);
    assert_eq!(m.get("x").is_none(), true);
    assert_eq!(m.get_mut("y").is_none(), true);


    assert!(m.capacity() >= cap_before.min(3));

    m.shrink_to_fit();
    let cap_after = m.capacity();
    assert!(cap_after <= cap_before);


    let m2 = parse_map("fresh: 1\ndata: 2");
    assert_eq!(m2.contains_key("fresh"), true);
    assert_eq!(m2.contains_key("x"), false);
    assert_eq!(m.is_empty(), true);
}

#[test]
fn test_remove_entry_vs_shift_remove_semantics() {

    let mut a = parse_map("p: 1\nq: 2\nr: 3\ns: 4");
    let mut b = parse_map("p: 1\nq: 2\nr: 3\ns: 4");

    assert_eq!(a.is_empty(), false);
    assert_eq!(b.is_empty(), false);
    assert_eq!(a.capacity(), a.capacity());

    let ra = a.remove_entry("q").expect("q in a");
    assert_eq!(ra.0, parse_val("q"));
    assert_eq!(ra.1, parse_val("2"));
    assert_eq!(a.contains_key("q"), false);

    let rb = b.shift_remove_entry("q").expect("q in b");
    assert_eq!(rb.0, parse_val("q"));
    assert_eq!(rb.1, parse_val("2"));
    assert_eq!(b.contains_key("q"), false);


    for key in ["p", "r", "s"].iter() {
        assert_eq!(a.contains_key(*key), true, "a lost {}", key);
        assert_eq!(b.contains_key(*key), true, "b lost {}", key);
    }


    let v = a.shift_remove("p").expect("p in a");
    assert_eq!(v, parse_val("1"));
    assert_eq!(a.contains_key("p"), false);
    assert_eq!(a.iter_mut().len(), 2);
}