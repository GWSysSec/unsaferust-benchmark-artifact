use bytes_utils::string::StrInner;
use bytes::BytesMut;
use bytes::Buf;

#[test]
fn test_str_inner_into_inner_with_bytes_mut() {
    let inner_data = BytesMut::from("hello world");
    let str_inner = StrInner::<BytesMut>::try_from(inner_data.clone()).unwrap();

    let reference = str_inner.inner();
    assert_eq!(reference.len(), 11);
    assert_eq!(&reference[..], b"hello world");

    let recovered = str_inner.into_inner();
    assert_eq!(recovered.len(), 11);
    assert_eq!(&recovered[..], b"hello world");
    assert_eq!(recovered, BytesMut::from("hello world"));
    assert_ne!(recovered.len(), 0);
    assert_eq!(recovered.remaining(), 11);
    assert_eq!(&recovered[0..5], b"hello");
    assert_eq!(&recovered[6..11], b"world");
}

#[test]
fn test_str_inner_inner_ref_multiple_calls() {
    let data = BytesMut::from("integration test string");
    let str_inner = StrInner::<BytesMut>::try_from(data).unwrap();

    let r1 = str_inner.inner();
    let r2 = str_inner.inner();

    assert_eq!(r1.len(), 23);
    assert_eq!(r2.len(), 23);
    assert_eq!(&r1[..], &r2[..]);
    assert_eq!(&r1[..], b"integration test string");
    assert_eq!(r1.len(), r2.len());
    assert_ne!(r1.len(), 0);
    assert_eq!(&r1[0..11], b"integration");
    assert_eq!(&r1[12..16], b"test");
}

#[test]
fn test_str_inner_inner_mut_modification() {
    let data = BytesMut::from("mutable content here");
    let mut str_inner = StrInner::<BytesMut>::try_from(data).unwrap();

    let before_len = str_inner.inner().len();
    assert_eq!(before_len, 20);
    assert_eq!(&str_inner.inner()[..], b"mutable content here");

    let inner_mut = unsafe { str_inner.inner_mut() };
    assert_eq!(inner_mut.len(), 20);
    inner_mut.truncate(7);

    assert_eq!(str_inner.inner().len(), 7);
    assert_eq!(&str_inner.inner()[..], b"mutable");
    assert_ne!(str_inner.inner().len(), 20);
    assert_eq!(str_inner.inner().remaining(), 7);
    assert_ne!(&str_inner.inner()[..], b"mutable content here");
}

#[test]
fn test_str_inner_freeze_converts_to_immutable() {
    let data = BytesMut::from("freeze me please");
    let str_inner = StrInner::<BytesMut>::try_from(data).unwrap();

    assert_eq!(str_inner.inner().len(), 16);
    assert_eq!(&str_inner.inner()[..], b"freeze me please");

    let frozen = str_inner.freeze();

    let frozen_inner = frozen.inner();
    assert_eq!(frozen_inner.len(), 16);
    assert_eq!(&frozen_inner[..], b"freeze me please");
    assert_ne!(frozen_inner.len(), 0);
    assert_eq!(&frozen_inner[0..6], b"freeze");
    assert_eq!(&frozen_inner[7..9], b"me");
    assert_eq!(frozen_inner.remaining(), 16);
}

#[test]
fn test_str_inner_freeze_then_into_inner() {
    let data = BytesMut::from("round trip test data");
    let str_inner = StrInner::<BytesMut>::try_from(data).unwrap();

    assert_eq!(str_inner.inner().len(), 20);

    let frozen = str_inner.freeze();
    assert_eq!(frozen.inner().len(), 20);
    assert_eq!(&frozen.inner()[..], b"round trip test data");

    let recovered = frozen.into_inner();
    assert_eq!(recovered.len(), 20);
    assert_eq!(&recovered[..], b"round trip test data");
    assert_ne!(recovered.len(), 0);
    assert_eq!(&recovered[0..5], b"round");
    assert_eq!(&recovered[11..15], b"test");
    assert_eq!(recovered.remaining(), 20);
}

#[test]
fn test_str_inner_empty_string_operations() {
    let data = BytesMut::from("");
    let mut str_inner = StrInner::<BytesMut>::try_from(data).unwrap();

    assert_eq!(str_inner.inner().len(), 0);
    assert_eq!(&str_inner.inner()[..], b"");
    assert_eq!(str_inner.inner().remaining(), 0);

    let inner_mut = unsafe { str_inner.inner_mut() };
    assert_eq!(inner_mut.len(), 0);
    inner_mut.extend_from_slice(b"added");

    assert_eq!(str_inner.inner().len(), 5);
    assert_eq!(&str_inner.inner()[..], b"added");
    assert_ne!(str_inner.inner().len(), 0);

    let recovered = str_inner.into_inner();
    assert_eq!(recovered.len(), 5);
}

#[test]
fn test_str_inner_inner_mut_extend_then_freeze() {
    let data = BytesMut::from("base");
    let mut str_inner = StrInner::<BytesMut>::try_from(data).unwrap();

    assert_eq!(str_inner.inner().len(), 4);
    assert_eq!(&str_inner.inner()[..], b"base");

    let inner_mut = unsafe { str_inner.inner_mut() };
    inner_mut.extend_from_slice(b" extended");

    assert_eq!(str_inner.inner().len(), 13);
    assert_eq!(&str_inner.inner()[..], b"base extended");

    let frozen = str_inner.freeze();
    assert_eq!(frozen.inner().len(), 13);
    assert_eq!(&frozen.inner()[..], b"base extended");
    assert_ne!(frozen.inner().len(), 4);
    assert_eq!(&frozen.inner()[0..4], b"base");
}

#[test]
fn test_str_inner_large_content_into_inner() {
    let large_string: String = "abcdefghij".repeat(1000);
    let data = BytesMut::from(large_string.as_str());
    let str_inner = StrInner::<BytesMut>::try_from(data).unwrap();

    assert_eq!(str_inner.inner().len(), 10000);
    assert_eq!(&str_inner.inner()[0..10], b"abcdefghij");
    assert_eq!(&str_inner.inner()[9990..10000], b"abcdefghij");

    let recovered = str_inner.into_inner();
    assert_eq!(recovered.len(), 10000);
    assert_ne!(recovered.len(), 0);
    assert_eq!(&recovered[0..10], b"abcdefghij");
    assert_eq!(&recovered[5000..5010], b"abcdefghij");
    assert_eq!(recovered.remaining(), 10000);
    assert_eq!(&recovered[9990..10000], b"abcdefghij");
}