use bstr::BString;
use bstr::ByteSlice;

#[test]
fn test_from_utf8_error_into_vec_basic() {

    let invalid_bytes: Vec<u8> = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0xFF, 0xFE, 0x57, 0x6F, 0x72, 0x6C, 0x64];
    let bstring = BString::from(invalid_bytes.clone());


    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();


    let err_bytes = err.as_bytes();
    assert_eq!(err_bytes.len(), 12);
    assert_eq!(err_bytes[0], 0x48);
    assert_eq!(err_bytes[5], 0xFF);


    let recovered_vec = err.into_vec();
    assert_eq!(recovered_vec.len(), 12);
    assert_eq!(recovered_vec, invalid_bytes);
    assert_eq!(recovered_vec[0], 0x48);
    assert_eq!(recovered_vec[5], 0xFF);
    assert_eq!(recovered_vec[11], 0x64);
}

#[test]
fn test_from_utf8_error_utf8_error_basic() {

    let invalid_bytes: Vec<u8> = vec![0x41, 0x42, 0x43, 0x80, 0x44];
    let bstring = BString::from(invalid_bytes.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();


    assert_eq!(utf8_err.valid_up_to(), 3);


    let bytes = err.as_bytes();
    assert_eq!(bytes[0], 0x41);
    assert_eq!(bytes[1], 0x42);
    assert_eq!(bytes[2], 0x43);
    assert_eq!(bytes[3], 0x80);
    assert_eq!(bytes[4], 0x44);
}

#[test]
fn test_from_utf8_error_into_vec_all_invalid() {

    let invalid_bytes: Vec<u8> = vec![0x80, 0x81, 0x82, 0x83, 0x84, 0x85];
    let bstring = BString::from(invalid_bytes.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();


    assert_eq!(utf8_err.valid_up_to(), 0);

    let recovered = err.into_vec();
    assert_eq!(recovered.len(), 6);
    assert_eq!(recovered[0], 0x80);
    assert_eq!(recovered[5], 0x85);
    assert_eq!(recovered, invalid_bytes);
}

#[test]
fn test_from_utf8_error_utf8_error_truncated_multibyte() {


    let invalid_bytes: Vec<u8> = vec![0x48, 0x69, 0xE2, 0x82];
    let bstring = BString::from(invalid_bytes.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();


    assert_eq!(utf8_err.valid_up_to(), 2);


    assert_eq!(err.as_bytes().len(), 4);
    assert_eq!(err.as_bytes()[0], 0x48);
    assert_eq!(err.as_bytes()[2], 0xE2);


    let recovered = err.into_vec();
    assert_eq!(recovered, invalid_bytes);
}

#[test]
fn test_from_utf8_error_roundtrip_workflow() {


    let input: Vec<u8> = vec![
        0x54, 0x68, 0x65, 0x20,
        0x71, 0x75, 0x69, 0x63, 0x6B,
        0x20,
        0xFE, 0xFF,
        0x20, 0x66, 0x6F, 0x78,
    ];

    let bstring = BString::from(input.clone());
    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();


    let utf8_err = err.utf8_error();
    let valid_up_to = utf8_err.valid_up_to();
    assert_eq!(valid_up_to, 10);


    let raw_bytes = err.as_bytes();
    let valid_prefix = &raw_bytes[..valid_up_to];
    assert_eq!(valid_prefix, b"The quick ");


    let full_vec = err.into_vec();
    assert_eq!(full_vec.len(), 16);
    assert_eq!(&full_vec[..10], b"The quick ");
    assert_eq!(full_vec[10], 0xFE);
    assert_eq!(full_vec[11], 0xFF);
    assert_eq!(&full_vec[12..], b" fox");
}

#[test]
fn test_from_utf8_error_valid_then_invalid_boundary() {


    let input: Vec<u8> = vec![0x63, 0x61, 0x66, 0xC3, 0xA9, 0xFF, 0x21];
    let bstring = BString::from(input.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();


    assert_eq!(utf8_err.valid_up_to(), 5);

    let bytes = err.as_bytes();
    assert_eq!(bytes.len(), 7);

    assert_eq!(bytes[3], 0xC3);
    assert_eq!(bytes[4], 0xA9);

    assert_eq!(bytes[5], 0xFF);

    let recovered = err.into_vec();
    assert_eq!(recovered, input);
    assert_ne!(recovered.len(), 0);
}

#[test]
fn test_from_utf8_error_empty_valid_prefix() {

    let input: Vec<u8> = vec![0xC0, 0x80];
    let bstring = BString::from(input.clone());

    let result = String::try_from(bstring);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let utf8_err = err.utf8_error();


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

    let mut input: Vec<u8> = Vec::with_capacity(1024);

    for i in 0..500u16 {
        input.push((i % 128) as u8);
    }

    input.push(0xFF);

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


    assert_eq!(utf8_err.valid_up_to(), 500);

    let recovered = err.into_vec();
    assert_eq!(recovered.len(), original_len);
    assert_eq!(recovered[500], 0xFF);
    assert_eq!(recovered[0], 0);
    assert_eq!(recovered[127], 127);

    assert_eq!(recovered[501], 0);
    assert_eq!(recovered[501 + 127], 127);
}