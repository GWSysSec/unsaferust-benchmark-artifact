use bytes_utils::SegmentedBuf;
use bytes::Buf;

use bytes::Bytes;
use bytes_utils::string::Str;
use std::convert::TryFrom;

#[test]
fn test_utf8_error_into_inner_from_invalid_bytes() {
    // Create invalid UTF-8 bytes
    let invalid_utf8: Bytes = Bytes::from(vec![0xFF, 0xFE, 0x80, 0x81, 0x82]);

    // Attempt to convert invalid bytes into a Str, which should produce a Utf8Error
    let result = Str::try_from(invalid_utf8.clone());
    assert!(result.is_err(), "Expected Utf8Error from invalid UTF-8 bytes");

    let err = result.unwrap_err();

    // Test utf8_error() method - get the underlying std::str::Utf8Error
    let std_err = err.utf8_error();
    // The invalid byte sequence starts at index 0
    assert_eq!(std_err.valid_up_to(), 0);
    assert!(std_err.error_len().is_some());

    // Test into_inner() - recovers the original bytes
    let recovered = err.into_inner();
    assert_eq!(recovered, invalid_utf8);
    assert_eq!(recovered.len(), 5);
    assert_eq!(recovered[0], 0xFF);
    assert_eq!(recovered[1], 0xFE);
    assert_eq!(recovered[2], 0x80);
}

#[test]
fn test_utf8_error_into_inner_partial_valid_utf8() {
    // Create bytes that start valid but end with invalid UTF-8
    // "hello" is valid, then we append invalid continuation bytes
    let mut data = Vec::from("hello".as_bytes());
    data.push(0xC0); // Invalid: overlong encoding start
    data.push(0x80); // continuation byte

    let bytes_data: Bytes = Bytes::from(data.clone());
    let result = Str::try_from(bytes_data.clone());
    assert!(result.is_err(), "Expected Utf8Error for partially valid UTF-8");

    let err = result.unwrap_err();

    // The valid portion is "hello" (5 bytes)
    let std_err = err.utf8_error();
    assert_eq!(std_err.valid_up_to(), 5);
    assert!(std_err.error_len().is_some());

    // Recover the inner bytes
    let recovered = err.into_inner();
    assert_eq!(recovered.len(), 7);
    assert_eq!(&recovered[..5], b"hello");
    assert_eq!(recovered[5], 0xC0);
    assert_eq!(recovered[6], 0x80);
}

#[test]
fn test_utf8_error_utf8_error_details_various_positions() {
    // Test with invalid byte at different positions
    // Valid UTF-8 prefix of 10 bytes, then invalid
    let mut data = Vec::from("0123456789".as_bytes());
    data.push(0xFE); // Invalid byte

    let bytes_data: Bytes = Bytes::from(data);
    let result = Str::try_from(bytes_data.clone());
    assert!(result.is_err());

    let err = result.unwrap_err();
    let std_err = err.utf8_error();

    // valid_up_to should be 10 (the 10 ASCII chars)
    assert_eq!(std_err.valid_up_to(), 10);
    assert!(std_err.error_len().is_some());
    assert_eq!(std_err.error_len().unwrap(), 1);

    // Verify into_inner gives back all 11 bytes
    let inner = err.into_inner();
    assert_eq!(inner.len(), 11);
    assert_eq!(&inner[..10], b"0123456789");
    assert_eq!(inner[10], 0xFE);
}

#[test]
fn test_utf8_error_into_inner_empty_invalid() {
    // A single invalid byte
    let bytes_data: Bytes = Bytes::from(vec![0x80]); // lone continuation byte
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

    // Also test with truncated multi-byte sequence
    // Start of a 3-byte sequence but missing continuation bytes
    let bytes_data2: Bytes = Bytes::from(vec![0xE0]);
    let result2 = Str::try_from(bytes_data2.clone());
    assert!(result2.is_err());

    let err2 = result2.unwrap_err();
    let std_err2 = err2.utf8_error();
    assert_eq!(std_err2.valid_up_to(), 0);
    // error_len may be None for truncated sequences
    let inner2 = err2.into_inner();
    assert_eq!(inner2.len(), 1);
    assert_eq!(inner2[0], 0xE0);
}

#[test]
fn test_utf8_error_with_multibyte_boundary() {
    // Valid multi-byte UTF-8 followed by invalid bytes
    // "café" in UTF-8 is [99, 97, 102, C3, A9] then add invalid
    let mut data = "café".as_bytes().to_vec();
    let valid_len = data.len(); // should be 5
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
    // Verify the valid prefix is preserved
    assert_eq!(&inner[..valid_len], "café".as_bytes());
    assert_eq!(inner[valid_len], 0xFF);
    assert_eq!(inner[valid_len + 1], 0xFE);
}

#[test]
fn test_utf8_error_combined_workflow_with_segmented_buf() {
    // Create a SegmentedBuf, extract bytes from it, then test Utf8Error
    let mut seg_buf = SegmentedBuf::new();

    // Push valid chunk
    let chunk1 = Bytes::from("valid text ");
    seg_buf.push(chunk1);

    // Read some bytes from segmented buf
    assert!(seg_buf.has_remaining());
    let remaining = seg_buf.remaining();
    assert!(remaining > 0);

    // Now create invalid UTF-8 and test error recovery
    // 0xED 0xA0 0x80 is a surrogate half - but std::str::from_utf8 reports
    // error_len of 1 at the 0xED byte (not 3), because it detects the invalid
    // sequence one byte at a time.
    let invalid_data: Bytes = Bytes::from(vec![
        b'A', b'B', b'C', // valid ASCII
        0xED, 0xA0, 0x80, // surrogate half (invalid UTF-8)
        b'D', b'E',
    ]);

    let result = Str::try_from(invalid_data.clone());
    assert!(result.is_err());

    let err = result.unwrap_err();
    let std_err = err.utf8_error();
    // "ABC" is valid (3 bytes), then invalid starts
    assert_eq!(std_err.valid_up_to(), 3);
    assert!(std_err.error_len().is_some());
    // std::str::from_utf8 reports error_len based on the actual invalid sequence length
    let actual_error_len = std_err.error_len().unwrap();
    assert!(actual_error_len >= 1);

    let inner = err.into_inner();
    assert_eq!(inner.len(), 8);
    assert_eq!(&inner[..3], b"ABC");
    assert_eq!(inner[3], 0xED);
}