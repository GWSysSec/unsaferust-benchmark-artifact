use bytes::{Buf, Bytes};
use bytes_utils::SegmentedBuf;

#[test]
fn segmented_module_into_inner_returns_empty_deque_for_new_buffer() {
    let segmented: SegmentedBuf<Bytes> = SegmentedBuf::new();

    assert_eq!(segmented.segments(), 0);
    assert_eq!(Buf::remaining(&segmented), 0);

    let inner = SegmentedBuf::into_inner(segmented);

    assert!(inner.is_empty());
    assert_eq!(inner.len(), 0);
}

#[test]
fn segmented_module_into_inner_preserves_pushed_segments_in_fifo_order() {
    let first = Bytes::from_static(b"one");
    let second = Bytes::from_static(b"two");
    let third = Bytes::from_static(b"three");

    let mut segmented = SegmentedBuf::new();
    segmented.push(first.clone());
    segmented.push(second.clone());
    segmented.push(third.clone());

    assert_eq!(segmented.segments(), 3);
    assert_eq!(
        Buf::remaining(&segmented),
        first.len() + second.len() + third.len()
    );

    let mut inner = SegmentedBuf::into_inner(segmented);

    assert_eq!(inner.len(), 3);
    assert_eq!(inner.pop_front(), Some(first));
    assert_eq!(inner.pop_front(), Some(second));
    assert_eq!(inner.pop_front(), Some(third));
    assert!(inner.is_empty());
}

#[test]
fn segmented_module_into_inner_returns_only_unconsumed_segments_after_advancing() {
    let first = Bytes::from_static(b"alpha");
    let second = Bytes::from_static(b"bravo");
    let third = Bytes::from_static(b"charlie");

    let mut segmented = SegmentedBuf::new();
    segmented.push(first);
    segmented.push(second.clone());
    segmented.push(third.clone());

    assert_eq!(segmented.segments(), 3);
    assert_eq!(Buf::chunk(&segmented), b"alpha");

    Buf::advance(&mut segmented, 5);

    assert_eq!(segmented.segments(), 2);
    assert_eq!(Buf::chunk(&segmented), b"bravo");
    assert_eq!(Buf::remaining(&segmented), second.len() + third.len());

    let mut inner = SegmentedBuf::into_inner(segmented);

    assert_eq!(inner.len(), 2);
    assert_eq!(inner.pop_front(), Some(second));
    assert_eq!(inner.pop_front(), Some(third));
    assert!(inner.is_empty());
}

#[test]
fn segmented_module_into_inner_keeps_partially_consumed_front_segment() {
    let first = Bytes::from_static(b"abcdef");
    let second = Bytes::from_static(b"ghij");

    let mut segmented = SegmentedBuf::new();
    segmented.push(first);
    segmented.push(second.clone());

    assert_eq!(Buf::chunk(&segmented), b"abcdef");

    Buf::advance(&mut segmented, 2);

    assert_eq!(segmented.segments(), 2);
    assert_eq!(Buf::chunk(&segmented), b"cdef");
    assert_eq!(Buf::remaining(&segmented), 8);

    let mut inner = SegmentedBuf::into_inner(segmented);

    assert_eq!(inner.len(), 2);
    assert_eq!(inner.pop_front(), Some(Bytes::from_static(b"cdef")));
    assert_eq!(inner.pop_front(), Some(second));
    assert!(inner.is_empty());
}