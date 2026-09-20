use bytes::{Bytes, BytesMut};
use bytes_utils::string::StrInner;

#[test]
fn utf8_error_reports_invalid_byte_and_into_inner_returns_original_bytes() {
    let original = Bytes::from_static(b"abc\xffdef");
    let result = StrInner::<Bytes>::from_inner(original.clone());

    assert!(result.is_err(), "invalid UTF-8 must be rejected");

    let error = match result {
        Ok(_) => panic!("invalid UTF-8 unexpectedly parsed successfully"),
        Err(error) => error,
    };

    let utf8_error = error.utf8_error();
    assert_eq!(utf8_error.valid_up_to(), 3);
    assert_eq!(utf8_error.error_len(), Some(1));

    let recovered = error.into_inner();
    assert_eq!(recovered, original);
    assert_eq!(&recovered[..3], b"abc");
    assert_eq!(&recovered[4..], b"def");

    let repaired = Bytes::from_static(b"abc-def");
    let parsed = StrInner::<Bytes>::from_inner(repaired.clone())
        .expect("repaired bytes should be valid UTF-8");

    assert_eq!(parsed.inner(), &repaired);
    assert_eq!(parsed.into_inner(), repaired);
}

#[test]
fn incomplete_multibyte_sequence_can_be_recovered_repaired_and_reparsed() {
    let invalid = BytesMut::from(&b"prefix \xe2\x82"[..]);
    let result = StrInner::<BytesMut>::from_inner(invalid);

    assert!(
        result.is_err(),
        "truncated multibyte UTF-8 sequence must be rejected"
    );

    let error = match result {
        Ok(_) => panic!("truncated UTF-8 unexpectedly parsed successfully"),
        Err(error) => error,
    };

    let utf8_error = error.utf8_error();
    assert_eq!(utf8_error.valid_up_to(), "prefix ".len());
    assert_eq!(utf8_error.error_len(), None);

    let mut recovered = error.into_inner();
    assert_eq!(&recovered[..], b"prefix \xe2\x82");

    recovered.truncate(utf8_error.valid_up_to());
    recovered.extend_from_slice("€ suffix".as_bytes());

    let parsed = StrInner::<BytesMut>::from_inner(recovered)
        .expect("repaired mutable buffer should be valid UTF-8");

    assert_eq!(parsed, "prefix € suffix");
    assert_eq!(&parsed.inner()[..], "prefix € suffix".as_bytes());

    let extracted = parsed.into_inner();
    assert_eq!(&extracted[..], "prefix € suffix".as_bytes());
}