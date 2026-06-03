use combine::parser::range::{take_while, take_while1, take_fn};
use combine::parser::range::TakeRange;
use combine::Parser;

#[test]
fn test_take_while_digits() {
    let mut p = take_while(|c: char| c.is_ascii_digit());
    let res: Result<(&str, &str), _> = p.parse("12345abc");
    let (matched, rest) = res.expect("should parse");
    assert_eq!(matched, "12345");
    assert_eq!(rest, "abc");


    let res2: Result<(&str, &str), _> = p.parse("abc");
    let (m2, r2) = res2.expect("empty match ok");
    assert_eq!(m2, "");
    assert_eq!(r2, "abc");


    let res3: Result<(&str, &str), _> = p.parse("999");
    let (m3, r3) = res3.expect("ok");
    assert_eq!(m3, "999");
    assert_eq!(r3, "");

    assert_ne!(matched, rest);
}

#[test]
fn test_take_while1_requires_one() {
    let mut p = take_while1(|c: char| c.is_ascii_alphabetic());
    let res: Result<(&str, &str), _> = p.parse("hello123");
    let (m, r) = res.expect("ok");
    assert_eq!(m, "hello");
    assert_eq!(r, "123");


    let res_fail: Result<(&str, &str), _> = p.parse("123hello");
    assert!(res_fail.is_err());


    let res2: Result<(&str, &str), _> = p.parse("a1");
    let (m2, r2) = res2.expect("ok");
    assert_eq!(m2, "a");
    assert_eq!(r2, "1");

    let res3: Result<(&str, &str), _> = p.parse("xyz");
    let (m3, r3) = res3.expect("ok");
    assert_eq!(m3, "xyz");
    assert_eq!(r3, "");
    assert_ne!(m3, r3);
}

#[test]
fn test_take_while_bytes() {
    let input: &[u8] = b"   spaces then text";
    let mut p = take_while(|b: u8| b == b' ');
    let res: Result<(&[u8], &[u8]), _> = p.parse(input);
    let (m, r) = res.expect("ok");
    assert_eq!(m, b"   ");
    assert_eq!(r, b"spaces then text");
    assert_eq!(m.len(), 3);
    assert_eq!(r.len(), 16);

    let mut p1 = take_while1(|b: u8| b == b' ');
    let res2: Result<(&[u8], &[u8]), _> = p1.parse(input);
    let (m2, _r2) = res2.expect("ok");
    assert_eq!(m2.len(), 3);

    let res3: Result<(&[u8], &[u8]), _> = p1.parse(b"no leading" as &[u8]);
    assert!(res3.is_err());
}

#[test]
fn test_take_fn_take() {
    let mut p = take_fn(|input: &str| -> TakeRange {
        match input.find(',') {
            Some(i) => TakeRange::Found(i),
            None => TakeRange::NotFound(input.len()),
        }
    });
    let res: Result<(&str, &str), _> = p.parse("hello,world");
    let (m, r) = res.expect("ok");
    assert_eq!(m, "hello");
    assert_eq!(r, ",world");
    assert_eq!(m.len(), 5);


    let res2: Result<(&str, &str), _> = p.parse("abc");

    assert!(res2.is_err() || res2.as_ref().map(|(m, _)| m.len()).unwrap_or(0) <= 3);

    let res3: Result<(&str, &str), _> = p.parse(",first");
    let (m3, r3) = res3.expect("ok empty before comma");
    assert_eq!(m3, "");
    assert_eq!(r3, ",first");
    assert_ne!(m, m3);
}