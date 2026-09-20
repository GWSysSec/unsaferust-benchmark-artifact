use http_body::Frame;
use bytes::Bytes;
use http::HeaderMap;

#[test]
fn frame_data_creation_and_inspection() {
    let data = Bytes::from("hello world");
    let frame = Frame::data(data.clone());

    assert!(frame.is_data());
    assert!(!frame.is_trailers());

    let data_ref = frame.data_ref().unwrap();
    assert_eq!(data_ref, &data);
    assert_eq!(data_ref.len(), 11);
    assert_eq!(&data_ref[..5], b"hello");
    assert_eq!(&data_ref[6..], b"world");

    assert!(frame.trailers_ref().is_none());

    let extracted = frame.into_data().unwrap();
    assert_eq!(extracted, Bytes::from("hello world"));
}

#[test]
fn frame_trailers_creation_and_inspection() {
    let mut headers = HeaderMap::new();
    headers.insert("x-checksum", "abc123".parse().unwrap());
    headers.insert("x-request-id", "req-42".parse().unwrap());

    let frame: Frame<Bytes> = Frame::trailers(headers.clone());

    assert!(frame.is_trailers());
    assert!(!frame.is_data());

    let trailers_ref = frame.trailers_ref().unwrap();
    assert_eq!(trailers_ref.len(), 2);
    assert_eq!(trailers_ref.get("x-checksum").unwrap(), "abc123");
    assert_eq!(trailers_ref.get("x-request-id").unwrap(), "req-42");

    assert!(frame.data_ref().is_none());

    let extracted = frame.into_trailers().unwrap();
    assert_eq!(extracted.len(), 2);
    assert_eq!(extracted.get("x-checksum").unwrap(), "abc123");
}

#[test]
fn frame_into_data_fails_for_trailers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-foo", "bar".parse().unwrap());

    let frame: Frame<Bytes> = Frame::trailers(headers);

    assert!(!frame.is_data());
    assert!(frame.is_trailers());
    assert!(frame.data_ref().is_none());

    let result = frame.into_data();
    assert!(result.is_err());

    let frame_back = result.unwrap_err();
    assert!(frame_back.is_trailers());
    assert!(!frame_back.is_data());
    assert!(frame_back.trailers_ref().is_some());
    assert_eq!(frame_back.trailers_ref().unwrap().get("x-foo").unwrap(), "bar");
}

#[test]
fn frame_into_trailers_fails_for_data() {
    let data = Bytes::from("some payload data");
    let frame = Frame::data(data.clone());

    assert!(frame.is_data());
    assert!(!frame.is_trailers());
    assert!(frame.trailers_ref().is_none());

    let result = frame.into_trailers();
    assert!(result.is_err());

    let frame_back = result.unwrap_err();
    assert!(frame_back.is_data());
    assert!(!frame_back.is_trailers());
    assert_eq!(frame_back.data_ref().unwrap(), &data);
    assert_eq!(frame_back.data_ref().unwrap().len(), 17);
}

#[test]
fn frame_data_mut_modification() {
    let data = Bytes::from("original");
    let mut frame = Frame::data(data);

    assert!(frame.is_data());
    assert_eq!(frame.data_ref().unwrap(), &Bytes::from("original"));

    let data_mut = frame.data_mut().unwrap();
    *data_mut = Bytes::from("modified");

    assert_eq!(frame.data_ref().unwrap(), &Bytes::from("modified"));
    assert_eq!(frame.data_ref().unwrap().len(), 8);
    assert!(frame.is_data());
    assert!(!frame.is_trailers());

    let extracted = frame.into_data().unwrap();
    assert_eq!(extracted, Bytes::from("modified"));
}

#[test]
fn frame_trailers_mut_modification() {
    let mut headers = HeaderMap::new();
    headers.insert("x-initial", "value1".parse().unwrap());

    let mut frame: Frame<Bytes> = Frame::trailers(headers);

    assert!(frame.is_trailers());
    assert_eq!(frame.trailers_ref().unwrap().len(), 1);

    let trailers_mut = frame.trailers_mut().unwrap();
    trailers_mut.insert("x-added", "value2".parse().unwrap());
    trailers_mut.insert("x-another", "value3".parse().unwrap());

    assert_eq!(frame.trailers_ref().unwrap().len(), 3);
    assert_eq!(frame.trailers_ref().unwrap().get("x-initial").unwrap(), "value1");
    assert_eq!(frame.trailers_ref().unwrap().get("x-added").unwrap(), "value2");
    assert_eq!(frame.trailers_ref().unwrap().get("x-another").unwrap(), "value3");
    assert!(frame.data_mut().is_none());
}

#[test]
fn frame_empty_data() {
    let data = Bytes::new();
    let mut frame = Frame::data(data.clone());

    assert!(frame.is_data());
    assert!(!frame.is_trailers());

    let data_ref = frame.data_ref().unwrap();
    assert_eq!(data_ref.len(), 0);
    assert!(data_ref.is_empty());
    assert_eq!(data_ref, &Bytes::new());
    assert!(frame.trailers_ref().is_none());
    assert!(frame.trailers_mut().is_none());

    let extracted = frame.into_data().unwrap();
    assert!(extracted.is_empty());
}

#[test]
fn frame_empty_trailers() {
    let headers = HeaderMap::new();
    let mut frame: Frame<Bytes> = Frame::trailers(headers);

    assert!(frame.is_trailers());
    assert!(!frame.is_data());

    let trailers_ref = frame.trailers_ref().unwrap();
    assert_eq!(trailers_ref.len(), 0);
    assert!(trailers_ref.is_empty());
    assert!(frame.data_ref().is_none());
    assert!(frame.data_mut().is_none());

    let extracted = frame.into_trailers().unwrap();
    assert!(extracted.is_empty());
}

#[test]
fn frame_large_data_payload() {
    let large_vec = vec![0xABu8; 1024 * 1024];
    let data = Bytes::from(large_vec);
    let frame = Frame::data(data.clone());

    assert!(frame.is_data());
    assert!(!frame.is_trailers());

    let data_ref = frame.data_ref().unwrap();
    assert_eq!(data_ref.len(), 1024 * 1024);
    assert_eq!(data_ref[0], 0xAB);
    assert_eq!(data_ref[1024 * 1024 - 1], 0xAB);
    assert_eq!(data_ref[512 * 1024], 0xAB);

    let extracted = frame.into_data().unwrap();
    assert_eq!(extracted.len(), 1024 * 1024);
}

#[test]
fn frame_data_roundtrip_preserves_content() {
    let original = Bytes::from("The quick brown fox jumps over the lazy dog");
    let frame = Frame::data(original.clone());

    assert!(frame.is_data());
    let data_ref = frame.data_ref().unwrap();
    assert_eq!(data_ref, &original);
    assert_eq!(data_ref.len(), 43);

    let result = frame.into_trailers();
    assert!(result.is_err());

    let frame_recovered = result.unwrap_err();
    let final_data = frame_recovered.into_data().unwrap();
    assert_eq!(final_data, original);
    assert_eq!(final_data.len(), 43);
}