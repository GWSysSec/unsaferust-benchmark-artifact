use combine::Parser;
use combine::parser::token::{
    none_of, one_of, produce, satisfy, satisfy_map, tokens, tokens_cmp,
};

#[test]
fn test_satisfy_str_uppercase() {
    let mut p = satisfy(|c: char| c.is_ascii_uppercase());

    let r1 = p.parse("Hello");
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, 'H');
    assert_eq!(rest1, "ello");

    let r2 = p.parse("hello");
    assert!(r2.is_err());

    let r3 = p.parse("");
    assert!(r3.is_err());

    let r4 = p.parse("X");
    let (v4, rest4) = r4.unwrap();
    assert_eq!(v4, 'X');
    assert_eq!(rest4, "");

    let r5 = p.parse("ABC");
    let (v5, rest5) = r5.unwrap();
    assert_eq!(v5, 'A');
    assert_eq!(rest5, "BC");

    let r6 = p.parse("1A");
    assert!(r6.is_err());
}

#[test]
fn test_satisfy_bytes_stream() {
    let mut p = satisfy(|b: u8| b == b'x' || b == b'y');

    let input1: &[u8] = b"xhello";
    let r1 = p.parse(input1);
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, b'x');
    assert_eq!(rest1, b"hello" as &[u8]);

    let input2: &[u8] = b"yworld";
    let r2 = p.parse(input2);
    let (v2, rest2) = r2.unwrap();
    assert_eq!(v2, b'y');
    assert_eq!(rest2, b"world" as &[u8]);

    let input3: &[u8] = b"zfail";
    let r3 = p.parse(input3);
    assert!(r3.is_err());

    let input4: &[u8] = b"";
    let r4 = p.parse(input4);
    assert!(r4.is_err());

    let input5: &[u8] = b"y";
    let r5 = p.parse(input5);
    let (v5, rest5) = r5.unwrap();
    assert_eq!(v5, b'y');
    assert_eq!(rest5.len(), 0);
}

#[test]
fn test_satisfy_map_hex_digit_conversion() {
    let mut p = satisfy_map(|c: char| {
        if c.is_ascii_hexdigit() {
            c.to_digit(16)
        } else {
            None
        }
    });

    let r1 = p.parse("A123");
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, 10u32);
    assert_eq!(rest1, "123");

    let r2 = p.parse("f12");
    let (v2, rest2) = r2.unwrap();
    assert_eq!(v2, 15u32);
    assert_eq!(rest2, "12");

    let r3 = p.parse("5xy");
    let (v3, rest3) = r3.unwrap();
    assert_eq!(v3, 5u32);
    assert_eq!(rest3, "xy");

    let r4 = p.parse("G12");
    assert!(r4.is_err());

    let r5 = p.parse("");
    assert!(r5.is_err());

    let r6 = p.parse("0");
    let (v6, rest6) = r6.unwrap();
    assert_eq!(v6, 0u32);
    assert_eq!(rest6, "");

    let r7 = p.parse("eZ");
    let (v7, rest7) = r7.unwrap();
    assert_eq!(v7, 14u32);
    assert_eq!(rest7, "Z");
}

#[test]
fn test_one_of_vowels() {
    let vowels: Vec<char> = "aeiou".chars().collect();
    let mut p = one_of(vowels.clone());

    let r1 = p.parse("apple");
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, 'a');
    assert_eq!(rest1, "pple");

    let r2 = p.parse("egg");
    let (v2, rest2) = r2.unwrap();
    assert_eq!(v2, 'e');
    assert_eq!(rest2, "gg");

    let r3 = p.parse("umbrella");
    let (v3, rest3) = r3.unwrap();
    assert_eq!(v3, 'u');
    assert_eq!(rest3, "mbrella");

    let r4 = p.parse("xyz");
    assert!(r4.is_err());

    let r5 = p.parse("");
    assert!(r5.is_err());

    let r6 = p.parse("bee");
    assert!(r6.is_err());

    assert_eq!(vowels.len(), 5);
    assert_eq!(vowels[0], 'a');
    assert_eq!(vowels[4], 'u');
}

#[test]
fn test_one_of_bytes_digits() {
    let digits: Vec<u8> = vec![b'0', b'1', b'2', b'3'];
    let mut p = one_of(digits.clone());

    let input1: &[u8] = b"0xyz";
    let r1 = p.parse(input1);
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, b'0');
    assert_eq!(rest1, b"xyz" as &[u8]);

    let input2: &[u8] = b"3abc";
    let r2 = p.parse(input2);
    let (v2, rest2) = r2.unwrap();
    assert_eq!(v2, b'3');
    assert_eq!(rest2, b"abc" as &[u8]);

    let input3: &[u8] = b"5xyz";
    let r3 = p.parse(input3);
    assert!(r3.is_err());

    let input4: &[u8] = b"";
    let r4 = p.parse(input4);
    assert!(r4.is_err());

    let input5: &[u8] = b"9aa";
    let r5 = p.parse(input5);
    assert!(r5.is_err());

    assert_eq!(digits.len(), 4);
}

#[test]
fn test_none_of_whitespace() {
    let ws: Vec<char> = vec![' ', '\t', '\n'];
    let mut p = none_of(ws.clone());

    let r1 = p.parse("hello world");
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, 'h');
    assert_eq!(rest1, "ello world");

    let r2 = p.parse(" space");
    assert!(r2.is_err());

    let r3 = p.parse("\ttab");
    assert!(r3.is_err());

    let r4 = p.parse("\nnewline");
    assert!(r4.is_err());

    let r5 = p.parse("");
    assert!(r5.is_err());

    let r6 = p.parse("Xyz");
    let (v6, rest6) = r6.unwrap();
    assert_eq!(v6, 'X');
    assert_eq!(rest6, "yz");

    let r7 = p.parse("1 more");
    let (v7, rest7) = r7.unwrap();
    assert_eq!(v7, '1');
    assert_eq!(rest7, " more");

    assert_eq!(ws.len(), 3);
}

#[test]
fn test_none_of_bytes() {
    let forbidden: Vec<u8> = vec![b'\0', b'\r', b'\n'];
    let mut p = none_of(forbidden.clone());

    let input1: &[u8] = b"abc";
    let r1 = p.parse(input1);
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, b'a');
    assert_eq!(rest1, b"bc" as &[u8]);

    let input2: &[u8] = b"\nbad";
    let r2 = p.parse(input2);
    assert!(r2.is_err());

    let input3: &[u8] = b"\0null";
    let r3 = p.parse(input3);
    assert!(r3.is_err());

    let input4: &[u8] = b"";
    let r4 = p.parse(input4);
    assert!(r4.is_err());

    let input5: &[u8] = b"Z";
    let r5 = p.parse(input5);
    let (v5, rest5) = r5.unwrap();
    assert_eq!(v5, b'Z');
    assert_eq!(rest5.len(), 0);

    assert_eq!(forbidden.len(), 3);
}

#[test]
fn test_tokens_cmp_case_insensitive() {
    let expected: Vec<char> = "Hello".chars().collect();
    let mut p = tokens_cmp(expected.clone(), |l: char, r: char| {
        l.eq_ignore_ascii_case(&r)
    });

    let r1 = p.parse("HELLO world");
    assert!(r1.is_ok());
    let (_, rest1) = r1.unwrap();
    assert_eq!(rest1, " world");

    let r2 = p.parse("hello there");
    assert!(r2.is_ok());
    let (_, rest2) = r2.unwrap();
    assert_eq!(rest2, " there");

    let r3 = p.parse("HeLLo!");
    let (_, rest3) = r3.unwrap();
    assert_eq!(rest3, "!");

    let r4 = p.parse("world");
    assert!(r4.is_err());

    let r5 = p.parse("hell");
    assert!(r5.is_err());

    let r6 = p.parse("");
    assert!(r6.is_err());

    let r7 = p.parse("Hello");
    let (_, rest7) = r7.unwrap();
    assert_eq!(rest7, "");

    assert_eq!(expected.len(), 5);
    assert_eq!(expected[0], 'H');
}

#[test]
fn test_tokens_strict_equality() {
    let expected: Vec<char> = "abc".chars().collect();
    let mut p = tokens(|l: char, r: char| l == r, "expected abc", expected.clone());

    let r1 = p.parse("abcdef");
    assert!(r1.is_ok());
    let (_, rest1) = r1.unwrap();
    assert_eq!(rest1, "def");

    let r2 = p.parse("ABC");
    assert!(r2.is_err());

    let r3 = p.parse("abX");
    assert!(r3.is_err());

    let r4 = p.parse("");
    assert!(r4.is_err());

    let r5 = p.parse("abc");
    let (_, rest5) = r5.unwrap();
    assert_eq!(rest5, "");

    let r6 = p.parse("ab");
    assert!(r6.is_err());

    assert_eq!(expected, vec!['a', 'b', 'c']);
    assert_eq!(expected.len(), 3);
}

#[test]
fn test_tokens_on_bytes() {
    let expected: Vec<u8> = b"GET ".to_vec();
    let mut p = tokens(|l: u8, r: u8| l == r, "GET method", expected.clone());

    let input1: &[u8] = b"GET /path HTTP/1.1";
    let r1 = p.parse(input1);
    assert!(r1.is_ok());
    let (_, rest1) = r1.unwrap();
    assert_eq!(rest1, b"/path HTTP/1.1" as &[u8]);

    let input2: &[u8] = b"POST /path";
    let r2 = p.parse(input2);
    assert!(r2.is_err());

    let input3: &[u8] = b"GE";
    let r3 = p.parse(input3);
    assert!(r3.is_err());

    let input4: &[u8] = b"";
    let r4 = p.parse(input4);
    assert!(r4.is_err());

    let input5: &[u8] = b"GET ";
    let r5 = p.parse(input5);
    let (_, rest5) = r5.unwrap();
    assert_eq!(rest5.len(), 0);

    assert_eq!(expected.len(), 4);
    assert_eq!(expected[0], b'G');
}

#[test]
fn test_produce_constant_int() {
    let mut p = produce(|| 42i32);

    let r1 = p.parse("hello");
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, 42);
    assert_eq!(rest1, "hello");

    let r2 = p.parse("");
    assert!(r2.is_ok());
    let (v2, rest2) = r2.unwrap();
    assert_eq!(v2, 42);
    assert_eq!(rest2, "");

    let r3 = p.parse("xyz");
    let (v3, rest3) = r3.unwrap();
    assert_eq!(v3, 42);
    assert_eq!(rest3, "xyz");

    let r4 = p.parse("1234567890");
    let (v4, rest4) = r4.unwrap();
    assert_eq!(v4, 42);
    assert_eq!(rest4.len(), 10);
}

#[test]
fn test_produce_string_value() {
    let mut p = produce(|| String::from("default"));

    let r1 = p.parse("abc");
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, "default");
    assert_eq!(rest1, "abc");
    assert_eq!(v1.len(), 7);

    let r2 = p.parse("");
    let (v2, rest2) = r2.unwrap();
    assert_eq!(v2, "default");
    assert_eq!(rest2, "");

    let r3 = p.parse("xxx");
    let (v3, rest3) = r3.unwrap();
    assert_eq!(v3.chars().count(), 7);
    assert_eq!(rest3, "xxx");
    assert_ne!(v3, "different");
}

#[test]
fn test_produce_on_byte_stream() {
    let mut p = produce(|| vec![1u8, 2, 3]);

    let input1: &[u8] = b"hello";
    let r1 = p.parse(input1);
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, vec![1u8, 2, 3]);
    assert_eq!(rest1, b"hello" as &[u8]);
    assert_eq!(v1.len(), 3);

    let input2: &[u8] = b"";
    let r2 = p.parse(input2);
    let (v2, rest2) = r2.unwrap();
    assert_eq!(v2.len(), 3);
    assert_eq!(rest2.len(), 0);
    assert_eq!(v2[0], 1);
    assert_eq!(v2[2], 3);
}

#[test]
fn test_one_of_empty_set() {
    let empty: Vec<char> = Vec::new();
    let mut p = one_of(empty.clone());

    let r1 = p.parse("anything");
    assert!(r1.is_err());

    let r2 = p.parse("");
    assert!(r2.is_err());

    let r3 = p.parse("a");
    assert!(r3.is_err());

    let r4 = p.parse("xyz");
    assert!(r4.is_err());

    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
}

#[test]
fn test_none_of_empty_set_accepts_anything() {
    let empty: Vec<char> = Vec::new();
    let mut p = none_of(empty.clone());

    let r1 = p.parse("abc");
    assert!(r1.is_ok());
    let (v1, rest1) = r1.unwrap();
    assert_eq!(v1, 'a');
    assert_eq!(rest1, "bc");

    let r2 = p.parse("");
    assert!(r2.is_err());

    let r3 = p.parse("\n");
    let (v3, rest3) = r3.unwrap();
    assert_eq!(v3, '\n');
    assert_eq!(rest3, "");

    let r4 = p.parse("Z");
    let (v4, rest4) = r4.unwrap();
    assert_eq!(v4, 'Z');
    assert_eq!(rest4, "");

    assert!(empty.is_empty());
}