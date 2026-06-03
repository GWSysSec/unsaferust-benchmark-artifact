use bytes_utils::string::StrMut;
use std::ops::Deref;

#[test]
fn test_str_mut_split_built_basic() {
    let mut s = StrMut::from("hello world");


    let as_str: &str = s.deref();
    assert_eq!(as_str, "hello world");
    assert_eq!(as_str.len(), 11);


    let split = s.split_built();


    let split_str: &str = split.deref();
    assert_eq!(split_str, "hello world");
    assert_eq!(split_str.len(), 11);


    let remaining: &str = s.deref();
    assert_eq!(remaining, "");
    assert_eq!(remaining.len(), 0);


    assert_ne!(split_str, remaining);
    assert_eq!(split_str.as_bytes(), b"hello world");
}

#[test]
fn test_str_mut_split_built_empty() {
    let mut s = StrMut::from("");


    let as_str: &str = s.deref();
    assert_eq!(as_str, "");
    assert_eq!(as_str.len(), 0);


    let split = s.split_built();


    let split_str: &str = split.deref();
    assert_eq!(split_str, "");
    assert_eq!(split_str.len(), 0);

    let remaining: &str = s.deref();
    assert_eq!(remaining, "");
    assert_eq!(remaining.len(), 0);


    assert_eq!(split_str, remaining);
    assert_eq!(split_str.as_bytes(), b"");
}

#[test]
fn test_str_mut_split_built_unicode() {
    let content = "héllo wörld 🌍";
    let mut s = StrMut::from(content);


    let as_str: &str = s.deref();
    assert_eq!(as_str, content);
    let char_count = as_str.chars().count();
    assert!(as_str.len() > char_count);
    assert_eq!(char_count, content.chars().count());

    let original_len = as_str.len();


    let split = s.split_built();

    let split_str: &str = split.deref();
    assert_eq!(split_str, content);
    assert_eq!(split_str.len(), original_len);
    assert_eq!(split_str.chars().count(), content.chars().count());


    let remaining: &str = s.deref();
    assert_eq!(remaining, "");
    assert_eq!(remaining.len(), 0);
}

#[test]
fn test_str_mut_split_built_multiple_splits() {
    let mut s = StrMut::from("first");


    let split1 = s.split_built();
    let s1: &str = split1.deref();
    assert_eq!(s1, "first");


    let r1: &str = s.deref();
    assert_eq!(r1, "");


    let split2 = s.split_built();
    let s2: &str = split2.deref();
    assert_eq!(s2, "");


    let r2: &str = s.deref();
    assert_eq!(r2, "");


    assert_eq!(s1, "first");
    assert_eq!(s2, "");
    assert_ne!(s1, s2);
    assert_eq!(s1.len(), 5);
    assert_eq!(s2.len(), 0);
}

#[test]
fn test_str_mut_split_built_large_content() {

    let large = "abcdefghij".repeat(1000);
    assert_eq!(large.len(), 10000);

    let mut s = StrMut::from(large.as_str());


    let as_str: &str = s.deref();
    assert_eq!(as_str.len(), 10000);
    assert_eq!(as_str, large.as_str());


    let split = s.split_built();

    let split_str: &str = split.deref();
    assert_eq!(split_str.len(), 10000);
    assert_eq!(split_str, large.as_str());


    let remaining: &str = s.deref();
    assert_eq!(remaining, "");
    assert_eq!(remaining.len(), 0);


    assert!(split_str.starts_with("abcdefghij"));
    assert!(split_str.ends_with("abcdefghij"));
}

#[test]
fn test_str_mut_split_built_preserves_valid_utf8() {

    let content = "α β γ δ ε ζ η θ ι κ";
    let mut s = StrMut::from(content);

    let as_str: &str = s.deref();
    assert_eq!(as_str, content);
    let char_count = as_str.chars().count();
    assert_eq!(char_count, content.chars().count());

    let split = s.split_built();
    let split_str: &str = split.deref();


    assert_eq!(split_str, content);
    assert_eq!(split_str.chars().count(), content.chars().count());
    assert!(split_str.is_char_boundary(0));
    assert!(split_str.is_char_boundary(split_str.len()));


    let remaining: &str = s.deref();
    assert_eq!(remaining.len(), 0);
    assert_eq!(remaining.chars().count(), 0);
}