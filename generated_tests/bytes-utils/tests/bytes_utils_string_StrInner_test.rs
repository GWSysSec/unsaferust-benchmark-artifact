use bytes::{Bytes, BytesMut};
use bytes_utils::string::{Str, StrInner, StrMut};

#[test]
fn from_inner_inner_and_into_inner_preserve_valid_bytes_and_report_invalid_utf8() {
    let original = Bytes::from_static(b"hello, \xce\xbc-world");
    let text: Str = StrInner::from_inner(original.clone()).expect("valid UTF-8 should be accepted");

    assert_eq!(text, "hello, μ-world");
    assert_eq!(text.inner(), &original);
    assert_eq!(&text.inner()[..], b"hello, \xce\xbc-world");

    let extracted = text.into_inner();
    assert_eq!(extracted, original);

    let invalid = Bytes::from_static(b"abc\xffdef");
    let error = StrInner::<Bytes>::from_inner(invalid.clone()).expect_err("invalid UTF-8 must fail");

    assert_eq!(error.utf8_error().valid_up_to(), 3);
    assert_eq!(error.into_inner(), invalid);
}

#[test]
fn unchecked_construction_inner_mut_and_freeze_support_mutable_build_then_shareable_string() {
    let mut mutable: StrMut = unsafe {
        StrInner::from_inner_unchecked(BytesMut::from(&b"seed"[..]))
    };

    assert_eq!(mutable, "seed");

    unsafe {
        let inner = mutable.inner_mut();
        inner.extend_from_slice(b"-grown");
    }

    assert_eq!(mutable, "seed-grown");
    assert_eq!(&mutable.inner()[..], b"seed-grown");

    let frozen: Str = mutable.freeze();

    assert_eq!(frozen, "seed-grown");
    assert_eq!(&frozen.inner()[..], b"seed-grown");

    let frozen_inner = frozen.into_inner();
    assert_eq!(&frozen_inner[..], b"seed-grown");
}

#[test]
fn from_static_and_slice_ref_extract_substrings_without_changing_original() {
    let owned: Str = StrInner::from_static("Hello World");

    assert_eq!(owned, "Hello World");
    assert_eq!(&owned.inner()[..], b"Hello World");

    let borrowed_mid: &str = &owned[2..5];
    let mid = owned.slice_ref(borrowed_mid);

    assert_eq!(mid, "llo");
    assert_eq!(owned, "Hello World");

    let borrowed_tail: &str = &owned[6..];
    let tail = owned.slice_ref(borrowed_tail);

    assert_eq!(tail, "World");
    assert_eq!(tail.into_inner(), Bytes::from_static(b"World"));
}

#[test]
fn split_built_returns_completed_prefix_and_keeps_builder_reusable() {
    let mut builder: StrMut =
        StrInner::from_inner(BytesMut::with_capacity(64)).expect("empty buffer is valid UTF-8");

    builder.push_str("first");
    builder.push('-');
    builder.push_str("message");

    assert_eq!(builder, "first-message");

    let first = builder.split_built();

    assert_eq!(first, "first-message");
    assert_eq!(builder, "");

    builder.push_str("second");
    builder.push(':');
    builder.push('✓');

    assert_eq!(builder, "second:✓");

    let second = builder.split_built();

    assert_eq!(second, "second:✓");
    assert_eq!(builder, "");

    let first_bytes = first.into_inner();
    let second_bytes = second.into_inner();

    assert_eq!(&first_bytes[..], "first-message".as_bytes());
    assert_eq!(&second_bytes[..], "second:✓".as_bytes());
}