use bytes::{Bytes, BytesMut};
use bytes_utils::string::{Str, StrInner, StrMut};

#[test]
fn valid_and_invalid_utf8_workflow_covers_inner_into_inner_utf8_error_and_slice_ref() {
    let original = Bytes::from_static("prefix:αβγ:suffix".as_bytes());
    let text: Str = StrInner::<Bytes>::from_inner(original.clone())
        .expect("valid UTF-8 bytes should construct a string wrapper");

    assert_eq!(text, "prefix:αβγ:suffix");
    assert_eq!(StrInner::<Bytes>::inner(&text), &original);
    assert_eq!(&StrInner::<Bytes>::inner(&text)[..7], "prefix:".as_bytes());

    let middle: &str = &text[7..13];
    let sliced = StrInner::<Bytes>::slice_ref(&text, middle);

    assert_eq!(sliced, "αβγ");
    assert_eq!(&StrInner::<Bytes>::inner(&sliced)[..], "αβγ".as_bytes());

    let recovered = StrInner::<Bytes>::into_inner(text);
    assert_eq!(recovered, original);

    let invalid = Bytes::from_static(b"good\xffbad");
    let err = StrInner::<Bytes>::from_inner(invalid.clone())
        .expect_err("invalid UTF-8 must be rejected");

    let utf8 = err.utf8_error();
    assert_eq!(utf8.valid_up_to(), 4);
    assert_eq!(utf8.error_len(), Some(1));

    let recovered_invalid = err.into_inner();
    assert_eq!(recovered_invalid, invalid);
}

#[test]
fn mutable_build_modify_freeze_and_split_built_workflow() {
    let mut editable: StrMut = StrInner::<BytesMut>::from_inner(BytesMut::from(&b"start"[..]))
        .expect("ASCII seed is valid UTF-8");

    StrInner::<BytesMut>::push_str(&mut editable, "-");
    StrInner::<BytesMut>::push(&mut editable, 'λ');

    unsafe {
        let inner = StrInner::<BytesMut>::inner_mut(&mut editable);
        inner.extend_from_slice("-tail".as_bytes());
    }

    assert_eq!(editable, "start-λ-tail");
    assert_eq!(
        &StrInner::<BytesMut>::inner(&editable)[..],
        "start-λ-tail".as_bytes()
    );

    let frozen: Str = StrInner::<BytesMut>::freeze(editable);
    assert_eq!(frozen, "start-λ-tail");
    assert_eq!(
        &StrInner::<Bytes>::inner(&frozen)[..],
        "start-λ-tail".as_bytes()
    );

    let frozen_inner = StrInner::<Bytes>::into_inner(frozen);
    assert_eq!(frozen_inner, Bytes::from_static("start-λ-tail".as_bytes()));

    let mut builder: StrMut = StrInner::<BytesMut>::from_inner(BytesMut::from(&b"built-content"[..]))
        .expect("builder content is valid UTF-8");

    let built = StrInner::<BytesMut>::split_built(&mut builder);

    assert_eq!(built, "built-content");
    assert_eq!(builder, "");

    StrInner::<BytesMut>::push_str(&mut builder, "next");
    assert_eq!(builder, "next");
}