use scroll::Sleb128;

#[test]
fn sleb128_read_basic_values() {

    let bytes = [0x00u8];
    let mut offset = 0;
    let v = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(v, 0);
    assert_eq!(offset, 1);


    let bytes = [0x7fu8];
    let mut offset = 0;
    let v = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(v, -1);
    assert_eq!(offset, 1);


    let bytes = [0x01u8];
    let mut offset = 0;
    let v = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(v, 1);
    assert_eq!(offset, 1);
}

#[test]
fn sleb128_read_multibyte() {

    let bytes = [0xc0u8, 0xbb, 0x78];
    let mut offset = 0;
    let v = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(v, -123456);
    assert_eq!(offset, 3);


    let bytes = [0xe5u8, 0x8e, 0x26];
    let mut offset = 0;
    let v = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(v, 624485);
    assert_eq!(offset, 3);


    let bytes = [0xc0u8, 0x00];
    let mut offset = 0;
    let v = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(v, 64);
    assert_eq!(offset, 2);


    let bytes = [0x40u8];
    let mut offset = 0;
    let v = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(v, -64);
    assert_eq!(offset, 1);
}

#[test]
fn sleb128_read_sequential_and_errors() {

    let bytes = [0x00u8, 0x01, 0x7f, 0xc0, 0x00];
    let mut offset = 0;

    let a = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(a, 0);
    assert_eq!(offset, 1);

    let b = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(b, 1);
    assert_eq!(offset, 2);

    let c = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(c, -1);
    assert_eq!(offset, 3);

    let d = Sleb128::read(&bytes, &mut offset).unwrap();
    assert_eq!(d, 64);
    assert_eq!(offset, 5);


    let truncated = [0x80u8];
    let mut offset = 0;
    let res = Sleb128::read(&truncated, &mut offset);
    assert!(res.is_err());


    let empty: [u8; 0] = [];
    let mut offset = 0;
    let res = Sleb128::read(&empty, &mut offset);
    assert!(res.is_err());
}