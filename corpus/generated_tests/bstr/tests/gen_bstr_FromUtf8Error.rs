use bstr::BString;
use bstr::ByteSlice;

#[test]
fn test_from_utf8_error_into_vec_basic() {
    // Create a BString with invalid UTF-8
    let invalid_bytes: Vec<u8> = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0xFF, 0xFE, 0x57, 0x6F, 0x72, 0x6C, 0x64];
    let bstring = BString::from(invalid_bytes.clone());

    // Attempt to convert to String, which should fail
    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();

    // Check as_bytes before consuming
    let err_bytes = err.as_bytes();
    assert_eq!(err_bytes.len(), 12);
    assert_eq!(err_bytes[0], 0x48); // 'H'
    assert_eq!(err_bytes[5], 0xFF); // invalid byte

    // Now consume the error and get the vec back
    let recovered_vec = err.into_vec();
    assert_eq!(recovered_vec.len(), 12);
    assert_eq!(recovered_vec, invalid_bytes);
    assert_eq!(recovered_vec[0], 0x48);
    assert_eq!(recovered_vec[5], 0xFF);
    assert_eq!(recovered_vec[11], 0x64); // 'd'
}

#[test]
fn test_from_utf8_error_utf8_error_basic() {
    // Create bytes that start valid but become invalid
    let invalid_bytes: Vec<u8> = vec![0x41, 0x42, 0x43, 0x80, 0x44];
    let bstring = BString::from(invalid_bytes.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();

    // The valid portion is "ABC" (3 bytes), so error is at offset 3
    assert_eq!(utf8_err.valid_up_to(), 3);

    // Verify the bytes are still accessible
    let bytes = err.as_bytes();
    assert_eq!(bytes[0], 0x41); // 'A'
    assert_eq!(bytes[1], 0x42); // 'B'
    assert_eq!(bytes[2], 0x43); // 'C'
    assert_eq!(bytes[3], 0x80); // invalid
    assert_eq!(bytes[4], 0x44); // 'D'
}

#[test]
fn test_from_utf8_error_into_vec_all_invalid() {
    // All bytes are invalid UTF-8 continuation bytes
    let invalid_bytes: Vec<u8> = vec![0x80, 0x81, 0x82, 0x83, 0x84, 0x85];
    let bstring = BString::from(invalid_bytes.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();

    // Error should be at the very start since first byte is invalid
    assert_eq!(utf8_err.valid_up_to(), 0);

    let recovered = err.into_vec();
    assert_eq!(recovered.len(), 6);
    assert_eq!(recovered[0], 0x80);
    assert_eq!(recovered[5], 0x85);
    assert_eq!(recovered, invalid_bytes);
}

#[test]
fn test_from_utf8_error_utf8_error_truncated_multibyte() {
    // Start with valid ASCII, then a truncated 3-byte UTF-8 sequence
    // U+20AC (€) is E2 82 AC in UTF-8; truncate to just E2 82
    let invalid_bytes: Vec<u8> = vec![0x48, 0x69, 0xE2, 0x82];
    let bstring = BString::from(invalid_bytes.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();

    // "Hi" is valid (2 bytes), then the truncated sequence starts at offset 2
    assert_eq!(utf8_err.valid_up_to(), 2);

    // Verify we can still get the bytes
    assert_eq!(err.as_bytes().len(), 4);
    assert_eq!(err.as_bytes()[0], 0x48);
    assert_eq!(err.as_bytes()[2], 0xE2);

    // Recover the original vector
    let recovered = err.into_vec();
    assert_eq!(recovered, invalid_bytes);
}

#[test]
fn test_from_utf8_error_roundtrip_workflow() {
    // Simulate a workflow: receive bytes, try to parse as UTF-8, on failure
    // extract the valid prefix and the raw bytes for further processing
    let input: Vec<u8> = vec![
        0x54, 0x68, 0x65, 0x20, // "The "
        0x71, 0x75, 0x69, 0x63, 0x6B, // "quick"
        0x20, // " "
        0xFE, 0xFF, // invalid bytes
        0x20, 0x66, 0x6F, 0x78, // " fox"
    ];

    let bstring = BString::from(input.clone());
    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();

    // Check the utf8 error details
    let utf8_err = err.utf8_error();
    let valid_up_to = utf8_err.valid_up_to();
    assert_eq!(valid_up_to, 10); // "The quick " is 10 bytes

    // Extract the valid prefix from the raw bytes
    let raw_bytes = err.as_bytes();
    let valid_prefix = &raw_bytes[..valid_up_to];
    assert_eq!(valid_prefix, b"The quick ");

    // Recover the full vector for alternative processing
    let full_vec = err.into_vec();
    assert_eq!(full_vec.len(), 16);
    assert_eq!(&full_vec[..10], b"The quick ");
    assert_eq!(full_vec[10], 0xFE);
    assert_eq!(full_vec[11], 0xFF);
    assert_eq!(&full_vec[12..], b" fox");
}

#[test]
fn test_from_utf8_error_valid_then_invalid_boundary() {
    // Test with valid multi-byte UTF-8 followed by invalid bytes
    // U+00E9 (é) = C3 A9
    let input: Vec<u8> = vec![0x63, 0x61, 0x66, 0xC3, 0xA9, 0xFF, 0x21];
    let bstring = BString::from(input.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();

    // "café" is valid: c(1) + a(1) + f(1) + é(2) = 5 bytes
    assert_eq!(utf8_err.valid_up_to(), 5);

    let bytes = err.as_bytes();
    assert_eq!(bytes.len(), 7);
    // Verify the é encoding
    assert_eq!(bytes[3], 0xC3);
    assert_eq!(bytes[4], 0xA9);
    // The invalid byte
    assert_eq!(bytes[5], 0xFF);

    let recovered = err.into_vec();
    assert_eq!(recovered, input);
    assert_ne!(recovered.len(), 0);
}

#[test]
fn test_from_utf8_error_empty_valid_prefix() {
    // First byte is immediately invalid
    let input: Vec<u8> = vec![0xC0, 0x80]; // overlong encoding (invalid UTF-8)
    let bstring = BString::from(input.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();

    // 0xC0 starts an overlong sequence which is invalid
    assert_eq!(utf8_err.valid_up_to(), 0);

    assert_eq!(err.as_bytes().len(), 2);
    assert_eq!(err.as_bytes()[0], 0xC0);
    assert_eq!(err.as_bytes()[1], 0x80);

    let recovered = err.into_vec();
    assert_eq!(recovered, input);
    assert_eq!(recovered.len(), 2);
}

#[test]
fn test_from_utf8_error_large_buffer_into_vec_preserves_all() {
    // Test with a larger buffer to ensure into_vec preserves capacity/content
    let mut input: Vec<u8> = Vec::with_capacity(1024);
    // Fill with valid ASCII
    for i in 0..500u16 {
        input.push((i % 128) as u8);
    }
    // Insert invalid byte
    input.push(0xFF);
    // More valid ASCII
    for i in 0..500u16 {
        input.push((i % 128) as u8);
    }

    let original_len = input.len();
    assert_eq!(original_len, 1001);

    let bstring = BString::from(input.clone());
    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();

    // The first 500 bytes are valid (all < 128)
    assert_eq!(utf8_err.valid_up_to(), 500);

    let recovered = err.into_vec();
    assert_eq!(recovered.len(), original_len);
    assert_eq!(recovered[500], 0xFF);
    assert_eq!(recovered[0], 0);
    assert_eq!(recovered[127], 127);
    // Verify the pattern after the invalid byte
    assert_eq!(recovered[501], 0);
    assert_eq!(recovered[501 + 127], 127);
}