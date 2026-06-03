use combine::stream::easy::{Errors, Info};

#[test]
fn test_errors_set_expected_attaches_expected_info() {
    let mut e: Errors<char, &str, i32> = Errors {
        position: 10,
        errors: Vec::new(),
    };
    assert_eq!(e.position, 10);
    assert_eq!(e.errors.len(), 0);

    e.set_expected(Info::Token('x'));
    assert_eq!(e.position, 10);
    assert!(!e.errors.is_empty());

    let after_first = e.errors.len();
    e.set_expected(Info::Token('y'));
    assert_eq!(e.position, 10);
    assert!(!e.errors.is_empty());

    assert_eq!(e.errors.len(), after_first);

    let mut e2: Errors<char, &str, usize> = Errors {
        position: 0,
        errors: Vec::new(),
    };
    e2.set_expected(Info::Range("number"));
    assert_eq!(e2.position, 0);
    assert!(!e2.errors.is_empty());
}

#[test]
fn test_errors_map_position_transforms_position_only() {
    let e: Errors<char, &str, i32> = Errors {
        position: 42,
        errors: Vec::new(),
    };
    let e2: Errors<char, &str, String> = e.map_position(|p| format!("pos={}", p));
    assert_eq!(e2.position, "pos=42");
    assert_eq!(e2.errors.len(), 0);

    let e3: Errors<char, &str, i32> = Errors {
        position: 100,
        errors: Vec::new(),
    };
    let e4: Errors<char, &str, i64> = e3.map_position(|p| p as i64 * 2);
    assert_eq!(e4.position, 200i64);
    assert_eq!(e4.errors.len(), 0);

    let e5: Errors<char, &str, usize> = Errors {
        position: 0,
        errors: Vec::new(),
    };
    let e6 = e5.map_position(|p| p + 1);
    assert_eq!(e6.position, 1usize);
    assert_eq!(e6.errors.len(), 0);

    let e7: Errors<char, &str, i32> = Errors {
        position: -5,
        errors: Vec::new(),
    };
    let e8 = e7.map_position(|p| p.abs());
    assert_eq!(e8.position, 5i32);
    assert_eq!(e8.errors.len(), 0);
}

#[test]
fn test_errors_map_token_changes_token_type() {
    let e: Errors<char, &str, i32> = Errors {
        position: 0,
        errors: Vec::new(),
    };
    let e2: Errors<u32, &str, i32> = e.map_token(|c| c as u32);
    assert_eq!(e2.position, 0);
    assert_eq!(e2.errors.len(), 0);

    let e3: Errors<char, &str, i32> = Errors {
        position: 5,
        errors: Vec::new(),
    };
    let e4: Errors<String, &str, i32> = e3.map_token(|c| c.to_string());
    assert_eq!(e4.position, 5);
    assert_eq!(e4.errors.len(), 0);

    let e5: Errors<u8, &str, i32> = Errors {
        position: 1,
        errors: Vec::new(),
    };
    let e6: Errors<i32, &str, i32> = e5.map_token(|b| b as i32);
    assert_eq!(e6.position, 1);
    assert_eq!(e6.errors.len(), 0);

    let e7: Errors<char, &str, usize> = Errors {
        position: 99,
        errors: Vec::new(),
    };
    let e8: Errors<u8, &str, usize> = e7.map_token(|c| c as u8);
    assert_eq!(e8.position, 99);
    assert_eq!(e8.errors.len(), 0);
}

#[test]
fn test_errors_map_range_changes_range_type() {
    let e: Errors<char, &str, i32> = Errors {
        position: 0,
        errors: Vec::new(),
    };
    let e2: Errors<char, String, i32> = e.map_range(|r: &str| r.to_string());
    assert_eq!(e2.position, 0);
    assert_eq!(e2.errors.len(), 0);

    let e3: Errors<char, &str, i32> = Errors {
        position: 7,
        errors: Vec::new(),
    };
    let e4: Errors<char, usize, i32> = e3.map_range(|r: &str| r.len());
    assert_eq!(e4.position, 7);
    assert_eq!(e4.errors.len(), 0);

    let e5: Errors<char, &str, usize> = Errors {
        position: 3,
        errors: Vec::new(),
    };
    let e6: Errors<char, usize, usize> = e5.map_range(|r: &str| r.len() * 2);
    assert_eq!(e6.position, 3);
    assert_eq!(e6.errors.len(), 0);

    let e7: Errors<char, &str, i32> = Errors {
        position: -1,
        errors: Vec::new(),
    };
    let e8: Errors<char, String, i32> = e7.map_range(|r: &str| format!("range={}", r));
    assert_eq!(e8.position, -1);
    assert_eq!(e8.errors.len(), 0);
}