use bytes_utils::SegmentedBuf;
use bytes::Buf;

use bytes::Bytes;
use bytes_utils::string::Str;
use std::convert::TryFrom;

#[test]
fn test_utf8_error_into_inner_from_invalid_bytes() {

    let invalid_utf8: Bytes = Bytes::from(vec![0xFF, 0xFE, 0x80, 0x81, 0x82]);


    let result = Str::try_from(invalid_utf8.clone());
    assert!(result.is_err(), "Expected Utf8Error from invalid UTF-8 bytes");

    let err = result.unwrap_err();


    let std_err = err.utf8_error();

    assert_eq!(std_err.valid_up_to(), 0);
    assert!(std_err.error_len().is_some());


    let recovered = err.into_inner();
    assert_eq!(recovered, invalid_utf8);
    assert_eq!(recovered.len(), 5);
    assert_eq!(recovered[0], 0xFF);
    assert_eq!(recovered[1], 0xFE);
    assert_eq!(recovered[2], 0x80);
}

#[test]
fn test_utf8_error_into_inner_partial_valid_utf8() {


    let mut data = Vec::from("hello".as_bytes());
    data.push(0xC0);
    data.push(0x80);

    let bytes_data: Bytes = Bytes::from(data.clone());
    let result = Str::try_from(bytes_data.clone());
    assert!(result.is_err(), "Expected Utf8Error for partially valid UTF-8");

    let err = result.unwrap_err();


    let std_err = err.utf8_error();
    assert_eq!(std_err.valid_up_to(), 5);
    assert!(std_err.error_len().is_some());


    let recovered = err.into_inner();
    assert_eq!(recovered.len(), 7);
    assert_eq!(&recovered[..5], b"hello");
    assert_eq!(recovered[5], 0xC0);
    assert_eq!(recovered[6], 0x80);
}

#[test]
fn test_utf8_error_utf8_error_details_various_positions() {


    let mut data = Vec::from("0123456789".as_bytes());
    data.push(0xFE);

    let bytes_data: Bytes = Bytes::from(data);
    let result = Str::try_from(bytes_data.clone());
    assert!(result.is_err());

    let err = result.unwrap_err();
    let std_err = err.utf8_error();


    assert_eq!(std_err.valid_up_to(), 10);
    assert!(std_err.error_len().is_some());
    assert_eq!(std_err.error_len().unwrap(), 1);


    let inner = err.into_inner();
    assert_eq!(inner.len(), 11);
    assert_eq!(&inner[..10], b"0123456789");
    assert_eq!(inner[10], 0xFE);
}

#[test]
fn test_utf8_error_into_inner_empty_invalid() {

    let bytes_data: Bytes = Bytes::from(vec![0x80]);
    let result = Str::try_from(bytes_data.clone());
    assert!(result.is_err());

    let err = result.unwrap_err();
    let std_err = err.utf8_error();
    assert_eq!(std_err.valid_up_to(), 0);
    assert!(std_err.error_len().is_some());
    assert_eq!(std_err.error_len().unwrap(), 1);

    let inner = err.into_inner();
    assert_eq!(inner.len(), 1);
    assert_eq!(inner[0], 0x80);



    let bytes_data2: Bytes = Bytes::from(vec![0xE0]);
    let result2 = Str::try_from(bytes_data2.clone());
    assert!(result2.is_err());

    let err2 = result2.unwrap_err();
    let std_err2 = err2.utf8_error();
    assert_eq!(std_err2.valid_up_to(), 0);

    let inner2 = err2.into_inner();
    assert_eq!(inner2.len(), 1);
    assert_eq!(inner2[0], 0xE0);
}

#[test]
fn test_utf8_error_with_multibyte_boundary() {


    let mut data = "café".as_bytes().to_vec();
    let valid_len = data.len();
    data.push(0xFF);
    data.push(0xFE);

    let bytes_data: Bytes = Bytes::from(data.clone());
    let result = Str::try_from(bytes_data.clone());
    assert!(result.is_err());

    let err = result.unwrap_err();
    let std_err = err.utf8_error();
    assert_eq!(std_err.valid_up_to(), valid_len);
    assert!(std_err.error_len().is_some());

    let inner = err.into_inner();
    assert_eq!(inner.len(), valid_len + 2);

    assert_eq!(&inner[..valid_len], "café".as_bytes());
    assert_eq!(inner[valid_len], 0xFF);
    assert_eq!(inner[valid_len + 1], 0xFE);
}

#[test]
fn test_utf8_error_combined_workflow_with_segmented_buf() {

    let mut seg_buf = SegmentedBuf::new();


    let chunk1 = Bytes::from("valid text ");
    seg_buf.push(chunk1);


    assert!(seg_buf.has_remaining());
    let remaining = seg_buf.remaining();
    assert!(remaining > 0);





    let invalid_data: Bytes = Bytes::from(vec![
        b'A', b'B', b'C',
        0xED, 0xA0, 0x80,
        b'D', b'E',
    ]);

    let result = Str::try_from(invalid_data.clone());
    assert!(result.is_err());

    let err = result.unwrap_err();
    let std_err = err.utf8_error();

    assert_eq!(std_err.valid_up_to(), 3);
    assert!(std_err.error_len().is_some());

    let actual_error_len = std_err.error_len().unwrap();
    assert!(actual_error_len >= 1);

    let inner = err.into_inner();
    assert_eq!(inner.len(), 8);
    assert_eq!(&inner[..3], b"ABC");
    assert_eq!(inner[3], 0xED);
}