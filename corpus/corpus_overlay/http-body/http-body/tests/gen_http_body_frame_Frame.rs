
use http_body::Frame;
use http::HeaderMap;
use http::header::{HeaderValue, CONTENT_TYPE};
use bytes::Bytes;

#[test]
fn frame_is_data_returns_true_for_data_frame() {
    let data = Bytes::from("hello world");
    let frame = Frame::data(data);

    assert!(frame.is_data());
    assert!(!frame.is_trailers());

    let frame2 = Frame::data(Bytes::from(""));
    assert!(frame2.is_data());
    assert!(!frame2.is_trailers());

    let frame3 = Frame::data(Bytes::from(vec![0u8; 1024]));
    assert!(frame3.is_data());
    assert!(!frame3.is_trailers());

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    let trailer_frame: Frame<Bytes> = Frame::trailers(headers);
    assert!(!trailer_frame.is_data());
    assert!(trailer_frame.is_trailers());
}

#[test]
fn frame_data_mut_returns_some_for_data_and_none_for_trailers() {
    let data = Bytes::from("original data");
    let mut frame = Frame::data(data.clone());

    // Pre-state: data_mut returns Some
    assert!(frame.data_mut().is_some());
    assert_eq!(frame.data_mut().unwrap().as_ref(), b"original data");

    // Mutate the data through data_mut
    let data_ref = frame.data_mut().unwrap();
    *data_ref = Bytes::from("modified data");

    // Post-state: verify mutation took effect
    assert_eq!(frame.data_mut().unwrap().as_ref(), b"modified data");
    assert!(frame.is_data());

    // Trailers frame returns None for data_mut
    let mut headers = HeaderMap::new();
    headers.insert("x-request-id", HeaderValue::from_static("abc123"));
    let mut trailer_frame: Frame<Bytes> = Frame::trailers(headers);
    assert!(trailer_frame.data_mut().is_none());
    assert!(!trailer_frame.is_data());
}

#[test]
fn frame_trailers_ref_returns_some_for_trailers_and_none_for_data() {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("x-custom", HeaderValue::from_static("value1"));

    let frame: Frame<Bytes> = Frame::trailers(headers.clone());

    // trailers_ref returns Some for trailer frames
    assert!(frame.trailers_ref().is_some());
    let trailers = frame.trailers_ref().unwrap();
    assert_eq!(trailers.len(), 2);
    assert_eq!(trailers.get(CONTENT_TYPE).unwrap(), "application/json");
    assert_eq!(trailers.get("x-custom").unwrap(), "value1");
    assert!(frame.is_trailers());

    // trailers_ref returns None for data frames
    let data_frame = Frame::data(Bytes::from("some bytes"));
    assert!(data_frame.trailers_ref().is_none());
    assert!(!data_frame.is_trailers());
}

#[test]
fn frame_trailers_mut_allows_modification_of_trailer_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-initial", HeaderValue::from_static("initial"));

    let mut frame: Frame<Bytes> = Frame::trailers(headers);

    // Pre-state checks
    assert!(frame.trailers_mut().is_some());
    assert_eq!(frame.trailers_mut().unwrap().len(), 1);
    assert_eq!(
        frame.trailers_mut().unwrap().get("x-initial").unwrap(),
        "initial"
    );

    // Mutate the trailers
    let trailers = frame.trailers_mut().unwrap();
    trailers.insert("x-added", HeaderValue::from_static("new_value"));
    trailers.remove("x-initial");

    // Post-state checks
    let trailers_after = frame.trailers_ref().unwrap();
    assert_eq!(trailers_after.len(), 1);
    assert!(trailers_after.get("x-initial").is_none());
    assert_eq!(trailers_after.get("x-added").unwrap(), "new_value");

    // Data frame returns None for trailers_mut
    let mut data_frame = Frame::data(Bytes::from("data"));
    assert!(data_frame.trailers_mut().is_none());
}

#[test]
fn frame_map_data_transforms_data_payload() {
    let original = Bytes::from("hello");
    let frame = Frame::data(original);

    // Pre-state
    assert!(frame.is_data());
    assert!(!frame.is_trailers());

    // Map data from Bytes to Vec<u8>
    let mapped_frame = frame.map_data(|b: Bytes| b.to_vec());

    // Post-state: the mapped frame should still be a data frame
    assert!(mapped_frame.is_data());
    assert!(!mapped_frame.is_trailers());
    assert!(mapped_frame.trailers_ref().is_none());

    let data_ref = mapped_frame.data_ref().unwrap();
    assert_eq!(data_ref, &b"hello".to_vec());
    assert_eq!(data_ref.len(), 5);
}

#[test]
fn frame_map_data_on_trailers_preserves_trailers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-trace-id", HeaderValue::from_static("trace-001"));
    headers.insert("x-span-id", HeaderValue::from_static("span-042"));

    let frame: Frame<Bytes> = Frame::trailers(headers);

    // Pre-state
    assert!(frame.is_trailers());
    assert!(!frame.is_data());
    assert_eq!(frame.trailers_ref().unwrap().len(), 2);

    // map_data on a trailers frame should preserve the trailers
    let mapped_frame = frame.map_data(|b: Bytes| b.to_vec());

    // Post-state: still a trailers frame with same content
    assert!(mapped_frame.is_trailers());
    assert!(!mapped_frame.is_data());
    let trailers = mapped_frame.trailers_ref().unwrap();
    assert_eq!(trailers.len(), 2);
    assert_eq!(trailers.get("x-trace-id").unwrap(), "trace-001");
    assert_eq!(trailers.get("x-span-id").unwrap(), "span-042");
}

#[test]
fn frame_map_data_complex_transformation() {
    // Create a data frame with some content
    let original = Bytes::from("abcdefghij");
    let frame = Frame::data(original);

    assert!(frame.is_data());

    // Map: reverse the bytes and convert to String
    let mapped = frame.map_data(|b: Bytes| {
        let mut v = b.to_vec();
        v.reverse();
        String::from_utf8(v).unwrap()
    });

    assert!(mapped.is_data());
    assert!(!mapped.is_trailers());
    assert!(mapped.trailers_ref().is_none());

    let result = mapped.data_ref().unwrap();
    assert_eq!(result, "jihgfedcba");
    assert_eq!(result.len(), 10);

    // Chain another map_data to convert String to usize (length)
    let mapped2 = mapped.map_data(|s: String| s.len());
    assert!(mapped2.is_data());
    assert_eq!(*mapped2.data_ref().unwrap(), 10usize);
}

#[test]
fn frame_data_mut_multiple_mutations() {
    let mut frame = Frame::data(Bytes::from("step0"));

    // First mutation
    assert_eq!(frame.data_mut().unwrap().as_ref(), b"step0");
    *frame.data_mut().unwrap() = Bytes::from("step1");
    assert_eq!(frame.data_mut().unwrap().as_ref(), b"step1");

    // Second mutation
    *frame.data_mut().unwrap() = Bytes::from("step2_longer");
    assert_eq!(frame.data_mut().unwrap().as_ref(), b"step2_longer");
    assert_eq!(frame.data_mut().unwrap().len(), 12);

    // Third mutation to empty
    *frame.data_mut().unwrap() = Bytes::new();
    assert_eq!(frame.data_mut().unwrap().len(), 0);
    assert!(frame.is_data());
}

#[test]
fn frame_trailers_mut_add_many_headers() {
    let headers = HeaderMap::new();
    let mut frame: Frame<Bytes> = Frame::trailers(headers);

    // Pre-state: empty trailers
    assert_eq!(frame.trailers_ref().unwrap().len(), 0);
    assert!(frame.is_trailers());

    // Add multiple headers through trailers_mut
    let trailers = frame.trailers_mut().unwrap();
    for i in 0..10 {
        let name = format!("x-header-{}", i);
        let value = format!("value-{}", i);
        trailers.insert(
            http::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_str(&value).unwrap(),
        );
    }

    // Post-state: verify all headers present
    let trailers_after = frame.trailers_ref().unwrap();
    assert_eq!(trailers_after.len(), 10);
    assert_eq!(trailers_after.get("x-header-0").unwrap(), "value-0");
    assert_eq!(trailers_after.get("x-header-5").unwrap(), "value-5");
    assert_eq!(trailers_after.get("x-header-9").unwrap(), "value-9");
    assert!(trailers_after.get("x-header-10").is_none());
}