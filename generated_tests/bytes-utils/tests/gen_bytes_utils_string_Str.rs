
use bytes_utils::SegmentedBuf;
use bytes::Bytes;

#[test]
fn test_segmented_buf_new_and_basic_operations() {
    let buf: SegmentedBuf<Bytes> = SegmentedBuf::new();

    use bytes::Buf;

    assert_eq!(buf.remaining(), 0);
    assert_eq!(buf.chunk().len(), 0);
    assert_eq!(buf.has_remaining(), false);

    let mut buf2: SegmentedBuf<Bytes> = SegmentedBuf::new();
    buf2.push(Bytes::from_static(b"hello"));
    buf2.push(Bytes::from_static(b" world"));

    assert_eq!(buf2.remaining(), 11);
    assert_eq!(buf2.has_remaining(), true);
    assert_eq!(buf2.chunk(), b"hello");

    buf2.advance(5);
    assert_eq!(buf2.remaining(), 6);
    assert_eq!(buf2.chunk(), b" world");
}

#[test]
fn test_segmented_buf_multiple_segments() {
    use bytes::Buf;

    let mut buf: SegmentedBuf<Bytes> = SegmentedBuf::new();

    assert_eq!(buf.remaining(), 0);

    buf.push(Bytes::from_static(b"abc"));
    buf.push(Bytes::from_static(b"def"));
    buf.push(Bytes::from_static(b"ghi"));

    assert_eq!(buf.remaining(), 9);
    assert_eq!(buf.chunk(), b"abc");

    buf.advance(3);
    assert_eq!(buf.remaining(), 6);
    assert_eq!(buf.chunk(), b"def");

    buf.advance(3);
    assert_eq!(buf.remaining(), 3);
    assert_eq!(buf.chunk(), b"ghi");

    buf.advance(3);
    assert_eq!(buf.remaining(), 0);
    assert_eq!(buf.has_remaining(), false);
}

#[test]
fn test_segmented_buf_partial_advance() {
    use bytes::Buf;

    let mut buf: SegmentedBuf<Bytes> = SegmentedBuf::new();
    buf.push(Bytes::from_static(b"hello"));
    buf.push(Bytes::from_static(b"world"));

    assert_eq!(buf.remaining(), 10);
    assert_eq!(buf.chunk(), b"hello");

    buf.advance(2);
    assert_eq!(buf.remaining(), 8);
    assert_eq!(buf.chunk(), b"llo");

    buf.advance(3);
    assert_eq!(buf.remaining(), 5);
    assert_eq!(buf.chunk(), b"world");

    buf.advance(1);
    assert_eq!(buf.remaining(), 4);
    assert_eq!(buf.chunk(), b"orld");
}

#[test]
fn test_segmented_buf_with_empty_segments() {
    use bytes::Buf;

    let mut buf: SegmentedBuf<Bytes> = SegmentedBuf::new();
    buf.push(Bytes::from_static(b""));
    buf.push(Bytes::from_static(b"data"));
    buf.push(Bytes::from_static(b""));
    buf.push(Bytes::from_static(b"more"));

    assert_eq!(buf.remaining(), 8);
    assert_eq!(buf.has_remaining(), true);


    let chunk = buf.chunk();
    assert_eq!(chunk, b"data");

    buf.advance(4);
    let chunk2 = buf.chunk();
    assert_eq!(chunk2, b"more");

    buf.advance(4);
    assert_eq!(buf.remaining(), 0);
    assert_eq!(buf.has_remaining(), false);
}

#[test]
fn test_segmented_slice_basic() {
    use bytes::Buf;
    use bytes_utils::SegmentedSlice;

    let mut slices: Vec<Bytes> = vec![
        Bytes::from_static(b"first"),
        Bytes::from_static(b"second"),
        Bytes::from_static(b"third"),
    ];

    let mut seg = SegmentedSlice::new(&mut slices);

    assert_eq!(seg.remaining(), 16);
    assert_eq!(seg.has_remaining(), true);
    assert_eq!(seg.chunk(), b"first");

    seg.advance(5);
    assert_eq!(seg.remaining(), 11);
    assert_eq!(seg.chunk(), b"second");

    seg.advance(6);
    assert_eq!(seg.remaining(), 5);
    assert_eq!(seg.chunk(), b"third");
}

#[test]
fn test_segmented_buf_copy_to_bytes() {
    use bytes::Buf;

    let mut buf: SegmentedBuf<Bytes> = SegmentedBuf::new();
    buf.push(Bytes::from_static(b"hello"));
    buf.push(Bytes::from_static(b" "));
    buf.push(Bytes::from_static(b"world"));

    assert_eq!(buf.remaining(), 11);

    let copied = buf.copy_to_bytes(5);
    assert_eq!(&copied[..], b"hello");
    assert_eq!(buf.remaining(), 6);

    let copied2 = buf.copy_to_bytes(1);
    assert_eq!(&copied2[..], b" ");
    assert_eq!(buf.remaining(), 5);

    let copied3 = buf.copy_to_bytes(5);
    assert_eq!(&copied3[..], b"world");
    assert_eq!(buf.remaining(), 0);
}

#[test]
fn test_segmented_buf_collect_from_iterator() {
    use bytes::Buf;

    let segments = vec![
        Bytes::from_static(b"one"),
        Bytes::from_static(b"two"),
        Bytes::from_static(b"three"),
    ];

    let buf: SegmentedBuf<Bytes> = segments.into_iter().collect();

    assert_eq!(buf.remaining(), 11);
    assert_eq!(buf.has_remaining(), true);
    assert_eq!(buf.chunk(), b"one");
}

#[test]
fn test_segmented_slice_with_partial_advance() {
    use bytes::Buf;
    use bytes_utils::SegmentedSlice;

    let mut slices: Vec<Bytes> = vec![
        Bytes::from_static(b"abcdef"),
        Bytes::from_static(b"ghijkl"),
    ];

    let mut seg = SegmentedSlice::new(&mut slices);

    assert_eq!(seg.remaining(), 12);
    assert_eq!(seg.chunk(), b"abcdef");

    seg.advance(3);
    assert_eq!(seg.remaining(), 9);
    assert_eq!(seg.chunk(), b"def");

    seg.advance(3);
    assert_eq!(seg.remaining(), 6);
    assert_eq!(seg.chunk(), b"ghijkl");

    seg.advance(2);
    assert_eq!(seg.remaining(), 4);
    assert_eq!(seg.chunk(), b"ijkl");
}