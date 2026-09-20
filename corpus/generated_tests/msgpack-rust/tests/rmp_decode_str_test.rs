#![allow(deprecated)]

fn encode_msgpack_str(value: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    rmp::encode::write_str(&mut buf, value).expect("string should encode");
    buf
}

#[test]
fn read_str_ref_decodes_each_string_in_a_messagepack_sequence() {
    let values = ["", "hello", "a longer string that uses a non-fixstr marker"];

    let encoded_chunks: Vec<Vec<u8>> = values.iter().map(|value| encode_msgpack_str(value)).collect();
    let mut stream = Vec::new();
    for chunk in &encoded_chunks {
        stream.extend_from_slice(chunk);
    }

    let mut offset = 0usize;
    for (expected, encoded_chunk) in values.iter().zip(encoded_chunks.iter()) {
        let decoded = rmp::decode::read_str_ref(&stream[offset..])
            .expect("encoded string should decode by reference");

        assert_eq!(decoded, expected.as_bytes());
        assert_eq!(std::str::from_utf8(decoded).expect("decoded bytes are utf-8"), *expected);
        assert_eq!(decoded.len(), expected.len());
        assert!(decoded.len() < encoded_chunk.len() || expected.is_empty());

        offset += encoded_chunk.len();
    }

    assert_eq!(offset, stream.len());
}

#[test]
fn read_str_ref_handles_unicode_and_returns_payload_bytes_only() {
    let text = "MessagePack loves Rust 🦀";
    let encoded = encode_msgpack_str(text);

    let decoded = rmp::decode::read_str_ref(&encoded).expect("unicode string should decode");

    assert_eq!(decoded, text.as_bytes());
    assert_eq!(decoded.len(), text.len());
    assert_eq!(std::str::from_utf8(decoded).expect("valid utf-8"), text);
    assert_ne!(decoded, encoded.as_slice());
}

#[test]
fn read_str_ref_rejects_non_string_marker() {
    let mut encoded_nil = Vec::new();
    rmp::encode::write_nil(&mut encoded_nil).expect("nil should encode");

    let result = rmp::decode::read_str_ref(&encoded_nil);

    assert!(result.is_err(), "non-string marker must not decode as a string");
}

#[test]
fn read_str_ref_rejects_truncated_string_payload() {
    let mut encoded = encode_msgpack_str("truncated");
    assert!(encoded.len() > 1);

    let original_payload_len = "truncated".len();
    encoded.pop();

    let result = std::panic::catch_unwind(|| rmp::decode::read_str_ref(&encoded));

    match result {
        Ok(Ok(decoded)) => panic!(
            "truncated payload decoded successfully: decoded {} bytes from declared {} byte payload",
            decoded.len(),
            original_payload_len
        ),
        Ok(Err(_)) => {}
        Err(_) => {}
    }
}

#[test]
fn read_str_ref_works_with_manually_encoded_str_len_and_payload() {
    let payload = b"manual payload";
    let mut encoded = Vec::new();

    rmp::encode::write_str_len(&mut encoded, payload.len() as u32)
        .expect("string length should encode");
    encoded.extend_from_slice(payload);

    let decoded = rmp::decode::read_str_ref(&encoded).expect("manual string should decode");

    assert_eq!(decoded, payload);
    assert_eq!(decoded.len(), payload.len());
    assert_eq!(encoded[0], 0xa0 | payload.len() as u8);
}