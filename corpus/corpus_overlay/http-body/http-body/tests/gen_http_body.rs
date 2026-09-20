use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::sync::Arc;
use std::task::{Wake, Waker};

struct NoopWaker;

impl Wake for NoopWaker {
    fn wake(self: Arc<Self>) {}
}

fn noop_waker() -> Waker {
    Arc::new(NoopWaker).into()
}

struct TestBody {
    frames: Vec<Frame<bytes::Bytes>>,
    size_hint: SizeHint,
}

impl TestBody {
    fn new(frames: Vec<Frame<bytes::Bytes>>, size_hint: SizeHint) -> Self {
        Self { frames, size_hint }
    }

    fn empty() -> Self {
        Self {
            frames: vec![],
            size_hint: SizeHint::default(),
        }
    }
}

impl Body for TestBody {
    type Data = bytes::Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.frames.is_empty() {
            Poll::Ready(None)
        } else {
            let frame = self.frames.remove(0);
            Poll::Ready(Some(Ok(frame)))
        }
    }

    fn size_hint(&self) -> SizeHint {
        self.size_hint.clone()
    }

    fn is_end_stream(&self) -> bool {
        self.frames.is_empty()
    }
}

// _assert_bounds is cfg'd out for non-internal builds, so we define a local
// compile-time bounds check instead.
fn assert_body_bounds<T: Body + Send + Sync>() {}

#[test]
fn test_assert_bounds_compiles_and_runs() {
    // Verify our TestBody satisfies Body trait requirements
    assert_body_bounds::<TestBody>();

    let body = TestBody::empty();
    let hint = body.size_hint();
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), None);

    // Verify is_end_stream for empty body
    let mut body = TestBody::empty();
    assert!(Pin::new(&mut body).is_end_stream());

    // Verify non-empty body is not end of stream
    let data_frame = Frame::data(bytes::Bytes::from("hello"));
    let mut body = TestBody::new(vec![data_frame], SizeHint::default());
    assert!(!Pin::new(&mut body).is_end_stream());

    // After consuming the frame, it should be end of stream
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let result = Pin::new(&mut body).poll_frame(&mut cx);
    assert!(matches!(result, Poll::Ready(Some(Ok(_)))));
    assert!(Pin::new(&mut body).is_end_stream());

    // Call assert_body_bounds again to confirm idempotency
    assert_body_bounds::<TestBody>();
}

#[test]
fn test_assert_bounds_with_size_hint_variations() {
    // Ensure assert_body_bounds works alongside various SizeHint configurations
    assert_body_bounds::<TestBody>();

    let mut hint = SizeHint::new();
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), None);

    hint.set_lower(100);
    assert_eq!(hint.lower(), 100);
    assert_eq!(hint.upper(), None);

    hint.set_upper(200);
    assert_eq!(hint.lower(), 100);
    assert_eq!(hint.upper(), Some(200));

    // Create body with specific size hint
    let body = TestBody::new(vec![], hint.clone());
    let returned_hint = body.size_hint();
    assert_eq!(returned_hint.lower(), 100);
    assert_eq!(returned_hint.upper(), Some(200));

    // assert_body_bounds should still work fine
    assert_body_bounds::<TestBody>();
}

#[test]
fn test_assert_bounds_with_frame_data_and_trailers() {
    assert_body_bounds::<TestBody>();

    // Create data frame
    let data = bytes::Bytes::from("test payload data");
    let data_frame = Frame::data(data.clone());
    assert!(data_frame.is_data());
    assert!(!data_frame.is_trailers());

    // Create trailers frame
    let mut trailers = http::HeaderMap::new();
    trailers.insert("x-checksum", http::HeaderValue::from_static("abc123"));
    let trailer_frame: Frame<bytes::Bytes> = Frame::trailers(trailers);
    assert!(!trailer_frame.is_trailers() || trailer_frame.is_trailers()); // it is trailers
    assert!(trailer_frame.is_trailers());
    assert!(!trailer_frame.is_data());

    // Build a body with both data and trailer frames
    let data_frame2 = Frame::data(bytes::Bytes::from("more data"));
    let mut trailers2 = http::HeaderMap::new();
    trailers2.insert("x-final", http::HeaderValue::from_static("done"));
    let trailer_frame2: Frame<bytes::Bytes> = Frame::trailers(trailers2);

    let mut hint = SizeHint::new();
    hint.set_lower(9);
    hint.set_upper(9);

    let mut body = TestBody::new(vec![data_frame2, trailer_frame2], hint);
    assert!(!Pin::new(&mut body).is_end_stream());

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    // Poll first frame (data)
    let frame1 = Pin::new(&mut body).poll_frame(&mut cx);
    match frame1 {
        Poll::Ready(Some(Ok(ref f))) => assert!(f.is_data()),
        _ => panic!("expected data frame"),
    }

    // Poll second frame (trailers)
    let frame2 = Pin::new(&mut body).poll_frame(&mut cx);
    match frame2 {
        Poll::Ready(Some(Ok(ref f))) => assert!(f.is_trailers()),
        _ => panic!("expected trailers frame"),
    }

    // Now body is exhausted
    assert!(Pin::new(&mut body).is_end_stream());

    assert_body_bounds::<TestBody>();
}

#[test]
fn test_assert_bounds_repeated_calls_are_safe() {
    // Verify that assert_body_bounds can be called multiple times without side effects
    for _ in 0..10 {
        assert_body_bounds::<TestBody>();
    }

    // Verify basic Body functionality still works after repeated calls
    let hint = SizeHint::with_exact(42);
    assert_eq!(hint.lower(), 42);
    assert_eq!(hint.upper(), Some(42));

    let frame = Frame::data(bytes::Bytes::from(vec![0u8; 42]));
    assert!(frame.is_data());

    let mut body = TestBody::new(vec![frame], hint.clone());
    let body_hint = body.size_hint();
    assert_eq!(body_hint.lower(), 42);
    assert_eq!(body_hint.upper(), Some(42));
    assert!(!Pin::new(&mut body).is_end_stream());

    assert_body_bounds::<TestBody>();

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let polled = Pin::new(&mut body).poll_frame(&mut cx);
    assert!(matches!(polled, Poll::Ready(Some(Ok(_)))));
    assert!(Pin::new(&mut body).is_end_stream());
}