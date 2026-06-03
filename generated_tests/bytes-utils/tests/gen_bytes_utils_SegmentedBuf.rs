
use bytes_utils::SegmentedBuf;
use bytes::Buf;
use std::collections::VecDeque;

#[test]
fn test_segmented_buf_into_inner_empty() {
    let buf: SegmentedBuf<&[u8]> = SegmentedBuf::new();


    assert_eq!(buf.remaining(), 0);

    let inner: VecDeque<&[u8]> = buf.into_inner();
    assert_eq!(inner.len(), 0);
    assert!(inner.is_empty());
    assert_eq!(inner.capacity(), 0);


    let mut rebuilt: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    for item in inner.iter() {
        rebuilt.push(*item);
    }
    assert_eq!(rebuilt.remaining(), 0);
    assert_eq!(rebuilt.chunk(), &[] as &[u8]);
    assert!(!rebuilt.has_remaining());
}

#[test]
fn test_segmented_buf_into_inner_single_segment() {
    let mut buf: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    let data: &[u8] = b"hello world";
    buf.push(data);


    assert_eq!(buf.remaining(), 11);
    assert!(buf.has_remaining());
    assert_eq!(buf.chunk(), b"hello world");

    let inner: VecDeque<&[u8]> = buf.into_inner();
    assert_eq!(inner.len(), 1);
    assert_eq!(inner[0], b"hello world");
    assert_eq!(inner[0].len(), 11);
    assert_eq!(inner.front().unwrap(), &b"hello world".as_slice());
    assert_eq!(inner.back().unwrap(), &b"hello world".as_slice());
}

#[test]
fn test_segmented_buf_into_inner_multiple_segments() {
    let mut buf: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    buf.push(b"alpha" as &[u8]);
    buf.push(b"beta" as &[u8]);
    buf.push(b"gamma" as &[u8]);
    buf.push(b"delta" as &[u8]);


    assert_eq!(buf.remaining(), 5 + 4 + 5 + 5);
    assert_eq!(buf.remaining(), 19);
    assert!(buf.has_remaining());
    assert_eq!(buf.chunk(), b"alpha");

    let inner: VecDeque<&[u8]> = buf.into_inner();
    assert_eq!(inner.len(), 4);
    assert_eq!(inner[0], b"alpha");
    assert_eq!(inner[1], b"beta");
    assert_eq!(inner[2], b"gamma");
    assert_eq!(inner[3], b"delta");


    let total: usize = inner.iter().map(|s| s.len()).sum();
    assert_eq!(total, 19);
}

#[test]
fn test_segmented_buf_into_inner_after_partial_advance() {
    let mut buf: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    buf.push(b"first" as &[u8]);
    buf.push(b"second" as &[u8]);
    buf.push(b"third" as &[u8]);


    assert_eq!(buf.remaining(), 5 + 6 + 5);
    assert_eq!(buf.remaining(), 16);


    buf.advance(5);
    assert_eq!(buf.remaining(), 11);
    assert_eq!(buf.chunk(), b"second");


    buf.advance(3);
    assert_eq!(buf.remaining(), 8);
    assert_eq!(buf.chunk(), b"ond");

    let inner: VecDeque<&[u8]> = buf.into_inner();


    let total_remaining: usize = inner.iter().map(|s| s.len()).sum();
    assert_eq!(total_remaining, 8);
}

#[test]
fn test_segmented_buf_into_inner_with_empty_segments() {
    let mut buf: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    buf.push(b"" as &[u8]);
    buf.push(b"nonempty" as &[u8]);
    buf.push(b"" as &[u8]);
    buf.push(b"data" as &[u8]);
    buf.push(b"" as &[u8]);


    assert_eq!(buf.remaining(), 12);
    assert!(buf.has_remaining());
    assert_eq!(buf.chunk(), b"nonempty");

    let inner: VecDeque<&[u8]> = buf.into_inner();

    let total_bytes: usize = inner.iter().map(|s| s.len()).sum();
    assert_eq!(total_bytes, 12);


    let non_empty: Vec<&&[u8]> = inner.iter().filter(|s| !s.is_empty()).collect();
    assert_eq!(non_empty.len(), 2);
    assert_eq!(*non_empty[0], b"nonempty");
    assert_eq!(*non_empty[1], b"data");
}

#[test]
fn test_segmented_buf_into_inner_roundtrip() {
    let mut buf: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    buf.push(b"one" as &[u8]);
    buf.push(b"two" as &[u8]);
    buf.push(b"three" as &[u8]);

    assert_eq!(buf.remaining(), 11);

    let inner: VecDeque<&[u8]> = buf.into_inner();
    assert_eq!(inner.len(), 3);


    let mut buf2: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    for segment in inner.iter() {
        buf2.push(*segment);
    }

    assert_eq!(buf2.remaining(), 11);
    assert_eq!(buf2.chunk(), b"one");

    buf2.advance(3);
    assert_eq!(buf2.chunk(), b"two");
    assert_eq!(buf2.remaining(), 8);

    buf2.advance(3);
    assert_eq!(buf2.chunk(), b"three");
    assert_eq!(buf2.remaining(), 5);
}

#[test]
fn test_segmented_buf_into_inner_large_number_of_segments() {
    let mut buf: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    let segments: Vec<Vec<u8>> = (0u8..100).map(|i| vec![i; (i as usize) + 1]).collect();

    for seg in segments.iter() {
        buf.push(seg.as_slice());
    }

    let expected_total: usize = (0u8..100).map(|i| (i as usize) + 1).sum();
    assert_eq!(buf.remaining(), expected_total);
    assert!(buf.has_remaining());


    buf.advance(1);
    let remaining_after = expected_total - 1;
    assert_eq!(buf.remaining(), remaining_after);

    let inner: VecDeque<&[u8]> = buf.into_inner();
    let inner_total: usize = inner.iter().map(|s| s.len()).sum();
    assert_eq!(inner_total, remaining_after);


    assert!(inner.len() >= 1);
    assert!(inner.len() <= 100);
}

#[test]
fn test_segmented_buf_into_inner_fully_consumed() {
    let mut buf: SegmentedBuf<&[u8]> = SegmentedBuf::new();
    buf.push(b"abc" as &[u8]);
    buf.push(b"de" as &[u8]);

    assert_eq!(buf.remaining(), 5);


    buf.advance(5);
    assert_eq!(buf.remaining(), 0);
    assert!(!buf.has_remaining());

    let inner: VecDeque<&[u8]> = buf.into_inner();

    let total: usize = inner.iter().map(|s| s.len()).sum();
    assert_eq!(total, 0);

    assert!(inner.is_empty() || total == 0);
}