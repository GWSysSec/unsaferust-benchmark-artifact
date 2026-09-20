use std::io::{Cursor, Read};

#[test]
fn u64_round_trip_uses_messagepack_u64_marker_and_big_endian_payload() {
    let original = 0x0123_4567_89ab_cdef_u64;
    let mut encoded = Vec::new();

    rmp::encode::write_u64(&mut encoded, original).expect("u64 should encode");

    assert_eq!(encoded.len(), 9);
    assert_eq!(encoded[0], 0xcf);
    assert_eq!(
        &encoded[1..],
        &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
    );

    let mut cursor = Cursor::new(encoded.as_slice());
    let decoded = rmp::decode::read_u64(&mut cursor).expect("u64 should decode");

    assert_eq!(decoded, original);
    assert_eq!(cursor.position(), encoded.len() as u64);
}

#[test]
fn read_u64_can_be_used_inside_a_larger_messagepack_workflow() {
    let values = [0_u64, 1, u32::MAX as u64 + 1, u64::MAX];
    let mut encoded = Vec::new();

    rmp::encode::write_array_len(&mut encoded, values.len() as u32).expect("array len");
    for value in values {
        rmp::encode::write_u64(&mut encoded, value).expect("array element u64");
    }
    rmp::encode::write_bool(&mut encoded, true).expect("trailing bool");

    assert_eq!(encoded[0], 0x94);

    let mut cursor = Cursor::new(encoded.as_slice());
    let decoded_len = rmp::decode::read_array_len(&mut cursor).expect("decode array len");
    assert_eq!(decoded_len, values.len() as u32);

    for expected in values {
        let decoded = rmp::decode::read_u64(&mut cursor).expect("decode u64 element");
        assert_eq!(decoded, expected);
    }

    let trailing = rmp::decode::read_bool(&mut cursor).expect("decode trailing bool");
    assert!(trailing);
    assert_eq!(cursor.position(), encoded.len() as u64);
}

#[test]
fn read_u64_reports_errors_for_wrong_marker_and_truncated_payload() {
    let mut wrong_marker = Cursor::new(vec![0xc3]);
    let wrong_marker_result = rmp::decode::read_u64(&mut wrong_marker);
    assert!(wrong_marker_result.is_err());

    let mut truncated = Cursor::new(vec![0xcf, 0x00, 0x01, 0x02]);
    let truncated_result = rmp::decode::read_u64(&mut truncated);
    assert!(truncated_result.is_err());
}

#[test]
fn read_u64_leaves_following_bytes_available_for_subsequent_decoders() {
    let mut encoded = Vec::new();

    rmp::encode::write_u64(&mut encoded, 42).expect("first u64");
    rmp::encode::write_str(&mut encoded, "done").expect("following string");

    let mut cursor = Cursor::new(encoded.as_slice());

    let decoded_number = rmp::decode::read_u64(&mut cursor).expect("decode first u64");
    assert_eq!(decoded_number, 42);

    let string_len = rmp::decode::read_str_len(&mut cursor).expect("decode string len");
    assert_eq!(string_len, 4);

    let mut string_buf = vec![0_u8; string_len as usize];
    cursor
        .read_exact(&mut string_buf)
        .expect("read encoded string bytes");
    assert_eq!(string_buf, b"done");
    assert_eq!(cursor.position(), encoded.len() as u64);
}