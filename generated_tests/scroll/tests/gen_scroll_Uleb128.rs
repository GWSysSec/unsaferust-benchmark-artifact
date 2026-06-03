use scroll::Uleb128;

#[test]
fn test_uleb128_read_single_byte_zero() {
    let bytes = [0x00];
    let mut offset = 0usize;
    let val = Uleb128::read(&bytes, &mut offset).expect("read zero");
    assert_eq!(val, 0);
    assert_eq!(offset, 1);
}

#[test]
fn test_uleb128_read_single_byte_small() {
    let bytes = [0x7f];
    let mut offset = 0usize;
    let val = Uleb128::read(&bytes, &mut offset).expect("read 127");
    assert_eq!(val, 127);
    assert_eq!(offset, 1);
}

#[test]
fn test_uleb128_read_multi_byte_624485() {

    let bytes = [0xE5, 0x8E, 0x26];
    let mut offset = 0usize;
    let val = Uleb128::read(&bytes, &mut offset).expect("read 624485");
    assert_eq!(val, 624485);
    assert_eq!(offset, 3);
}

#[test]
fn test_uleb128_read_two_byte_128() {

    let bytes = [0x80, 0x01];
    let mut offset = 0usize;
    let val = Uleb128::read(&bytes, &mut offset).expect("read 128");
    assert_eq!(val, 128);
    assert_eq!(offset, 2);
}

#[test]
fn test_uleb128_read_sequential() {

    let bytes = [0x01, 0x80, 0x01, 0x7f];
    let mut offset = 0usize;

    let v1 = Uleb128::read(&bytes, &mut offset).expect("first");
    assert_eq!(v1, 1);
    assert_eq!(offset, 1);

    let v2 = Uleb128::read(&bytes, &mut offset).expect("second");
    assert_eq!(v2, 128);
    assert_eq!(offset, 3);

    let v3 = Uleb128::read(&bytes, &mut offset).expect("third");
    assert_eq!(v3, 127);
    assert_eq!(offset, 4);
}

#[test]
fn test_uleb128_read_truncated_returns_error() {

    let bytes = [0x80];
    let mut offset = 0usize;
    let result = Uleb128::read(&bytes, &mut offset);
    match result {
        Ok(v) => panic!("expected error reading truncated uleb128, got {}", v),
        Err(e) => {
            let msg = format!("{}", e);
            assert!(!msg.is_empty());
        }
    }
}

#[test]
fn test_uleb128_read_empty_returns_error() {
    let bytes: [u8; 0] = [];
    let mut offset = 0usize;
    let result = Uleb128::read(&bytes, &mut offset);
    assert!(result.is_err(), "expected error reading from empty buffer");
}

#[test]
fn test_uleb128_read_offset_advances_correctly() {

    let bytes = [0xAC, 0x02, 0xFF, 0xFF];
    let mut offset = 0usize;
    let v = Uleb128::read(&bytes, &mut offset).expect("read 300");
    assert_eq!(v, 300);
    assert_eq!(offset, 2);

    assert_eq!(bytes.len() - offset, 2);
}