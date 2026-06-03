use bytes::{Buf, Bytes};
use bytes_utils::SegmentedBuf;

#[test]
fn into_inner_returns_empty_deque_for_new_segmented_buffer() {
    let segmented: SegmentedBuf<Bytes> = SegmentedBuf::new();

    assert_eq!(segmented.segments(), 0);
    assert_eq!(bytes::Buf::remaining(&segmented), 0);

    let inner = segmented.into_inner();
    assert!(inner.is_empty());
    assert_eq!(inner.len(), 0);
}

#[test]
fn into_inner_preserves_unconsumed_buffers_in_original_order() {
    let first = Bytes::from_static(b"alpha");
    let second = Bytes::from_static(b"beta");
    let third = Bytes::from_static(b"gamma");

    let mut segmented = SegmentedBuf::new();
    segmented.push(first.clone());
    segmented.push(second.clone());
    segmented.push(third.clone());

    assert_eq!(segmented.segments(), 3);
    assert_eq!(bytes::Buf::remaining(&segmented), first.len() + second.len() + third.len());

    let mut inner = segmented.into_inner();

    assert_eq!(inner.len(), 3);
    assert_eq!(inner.pop_front(), Some(first));
    assert_eq!(inner.pop_front(), Some(second));
    assert_eq!(inner.pop_front(), Some(third));
    assert!(inner.is_empty());
}

#[test]
fn into_inner_returns_only_yet_unconsumed_data_after_reads_cross_segments() {
    let mut segmented = SegmentedBuf::new();
    segmented.push(Bytes::from_static(b"hello"));
    segmented.push(Bytes::from_static(b", "));
    segmented.push(Bytes::from_static(b"world"));
    segmented.push(Bytes::from_static(b"!"));

    assert_eq!(segmented.segments(), 4);
    assert_eq!(bytes::Buf::remaining(&segmented), 13);

    assert_eq!(bytes::Buf::get_u8(&mut segmented), b'h');
    bytes::Buf::advance(&mut segmented, 4);

    let punctuation = bytes::Buf::copy_to_bytes(&mut segmented, 2);
    assert_eq!(punctuation, Bytes::from_static(b", "));
    assert_eq!(bytes::Buf::remaining(&segmented), 6);

    let mut inner = segmented.into_inner();

    assert_eq!(inner.len(), 2);
    assert_eq!(inner.pop_front(), Some(Bytes::from_static(b"world")));
    assert_eq!(inner.pop_front(), Some(Bytes::from_static(b"!")));
    assert!(inner.is_empty());
}

#[test]
fn into_inner_keeps_partially_consumed_front_buffer_with_remaining_suffix() {
    let mut segmented = SegmentedBuf::new();
    segmented.push(Bytes::from_static(b"abcdef"));
    segmented.push(Bytes::from_static(b"gh"));
    segmented.push(Bytes::from_static(b"ijkl"));

    assert_eq!(bytes::Buf::remaining(&segmented), 12);

    bytes::Buf::advance(&mut segmented, 2);
    let copied = bytes::Buf::copy_to_bytes(&mut segmented, 3);

    assert_eq!(copied, Bytes::from_static(b"cde"));
    assert_eq!(bytes::Buf::remaining(&segmented), 7);

    let mut inner = segmented.into_inner();

    assert_eq!(inner.len(), 3);
    assert_eq!(inner.pop_front(), Some(Bytes::from_static(b"f")));
    assert_eq!(inner.pop_front(), Some(Bytes::from_static(b"gh")));
    assert_eq!(inner.pop_front(), Some(Bytes::from_static(b"ijkl")));
    assert!(inner.is_empty());
}