use winnow::Parser;
use winnow::binary::{
    be_i16, be_i32, be_i64, be_u16, be_u24, be_u32, be_u64, be_u8, le_i16, le_i32, le_u16, le_u24,
    le_u32, le_u64, length_take, u16 as bin_u16, u32 as bin_u32, Endianness,
};
use winnow::error::ContextError;
use winnow::token::take;

#[test]
fn test_be_le_unsigned_basic() {

    let mut data: &[u8] = &[0x01, 0x02, 0x03];
    let r: Result<u8, ContextError> = be_u8(&mut data);
    assert_eq!(r.unwrap(), 0x01);
    assert_eq!(data, &[0x02u8, 0x03][..]);


    let mut data: &[u8] = &[0x12, 0x34, 0xff];
    let r: Result<u16, ContextError> = be_u16(&mut data);
    assert_eq!(r.unwrap(), 0x1234);
    assert_eq!(data, &[0xffu8][..]);


    let mut data: &[u8] = &[0x12, 0x34, 0xff];
    let r: Result<u16, ContextError> = le_u16(&mut data);
    assert_eq!(r.unwrap(), 0x3412);
    assert_eq!(data.len(), 1);


    let mut data: &[u8] = &[0xde, 0xad, 0xbe, 0xef];
    let r: Result<u32, ContextError> = be_u32(&mut data);
    assert_eq!(r.unwrap(), 0xdeadbeef);
    assert!(data.is_empty());

    let mut data: &[u8] = &[0xde, 0xad, 0xbe, 0xef];
    let r: Result<u32, ContextError> = le_u32(&mut data);
    assert_eq!(r.unwrap(), 0xefbeadde);
    assert!(data.is_empty());
}

#[test]
fn test_be_le_24_and_64() {
    let mut data: &[u8] = &[0x01, 0x02, 0x03, 0xaa];
    let r: Result<u32, ContextError> = be_u24(&mut data);
    assert_eq!(r.unwrap(), 0x010203);
    assert_eq!(data, &[0xaau8][..]);

    let mut data: &[u8] = &[0x01, 0x02, 0x03, 0xaa];
    let r: Result<u32, ContextError> = le_u24(&mut data);
    assert_eq!(r.unwrap(), 0x030201);
    assert_eq!(data, &[0xaau8][..]);

    let mut data: &[u8] = &[0, 0, 0, 0, 0, 0, 0, 0x42];
    let r: Result<u64, ContextError> = be_u64(&mut data);
    assert_eq!(r.unwrap(), 0x42);
    assert!(data.is_empty());

    let mut data: &[u8] = &[0x42, 0, 0, 0, 0, 0, 0, 0];
    let r: Result<u64, ContextError> = le_u64(&mut data);
    assert_eq!(r.unwrap(), 0x42);
    assert!(data.is_empty());


    let mut short: &[u8] = &[0x01, 0x02];
    let r: Result<u32, ContextError> = be_u32(&mut short);
    assert!(r.is_err());
}

#[test]
fn test_signed_parsers() {
    let mut data: &[u8] = &[0xff, 0xff];
    let r: Result<i16, ContextError> = be_i16(&mut data);
    assert_eq!(r.unwrap(), -1);
    assert!(data.is_empty());

    let mut data: &[u8] = &[0x00, 0x80];
    let r: Result<i16, ContextError> = le_i16(&mut data);
    assert_eq!(r.unwrap(), i16::MIN);
    assert!(data.is_empty());

    let mut data: &[u8] = &[0x80, 0x00, 0x00, 0x00];
    let r: Result<i32, ContextError> = be_i32(&mut data);
    assert_eq!(r.unwrap(), i32::MIN);
    assert!(data.is_empty());

    let mut data: &[u8] = &[0xff, 0xff, 0xff, 0xff];
    let r: Result<i32, ContextError> = le_i32(&mut data);
    assert_eq!(r.unwrap(), -1);
    assert!(data.is_empty());

    let mut data: &[u8] = &[0, 0, 0, 0, 0, 0, 0, 1];
    let r: Result<i64, ContextError> = be_i64(&mut data);
    assert_eq!(r.unwrap(), 1);
    assert!(data.is_empty());
}

#[test]
fn test_endianness_dispatch() {

    let value: u32 = 0xCAFEBABE;
    let bytes = value.to_ne_bytes();

    let mut be_parser = bin_u16::<&[u8], ContextError>(Endianness::Big);
    let mut le_parser = bin_u16::<&[u8], ContextError>(Endianness::Little);
    let mut native_parser = bin_u32::<&[u8], ContextError>(Endianness::Native);

    let mut input1: &[u8] = &[0x12, 0x34];
    let r1 = be_parser.parse_next(&mut input1).unwrap();
    assert_eq!(r1, 0x1234);
    assert!(input1.is_empty());

    let mut input2: &[u8] = &[0x12, 0x34];
    let r2 = le_parser.parse_next(&mut input2).unwrap();
    assert_eq!(r2, 0x3412);
    assert!(input2.is_empty());


    let mut input3: &[u8] = &bytes;
    let r3 = native_parser.parse_next(&mut input3).unwrap();
    assert_eq!(r3, value);
    assert!(input3.is_empty());


    let mut short: &[u8] = &[0x12];
    let r4 = be_parser.parse_next(&mut short);
    assert!(r4.is_err());
}

#[test]
fn test_length_take_workflow() {

    let mut parser = length_take::<&[u8], u8, ContextError, _>(be_u8);

    let mut input: &[u8] = &[0x03, b'a', b'b', b'c', b'X', b'Y'];
    let r = parser.parse_next(&mut input).unwrap();
    assert_eq!(r, &[b'a', b'b', b'c'][..]);
    assert_eq!(input, &[b'X', b'Y'][..]);


    let mut input2: &[u8] = &[0x00, 0xaa];
    let r2 = parser.parse_next(&mut input2).unwrap();
    assert_eq!(r2.len(), 0);
    assert_eq!(input2, &[0xaau8][..]);


    let mut input3: &[u8] = &[0x05, b'a', b'b'];
    let r3 = parser.parse_next(&mut input3);
    assert!(r3.is_err());


    let mut input4: &[u8] = &[0x02, b'h', b'i', b'!', b'!'];
    let payload = parser.parse_next(&mut input4).unwrap();
    assert_eq!(payload, b"hi");
    let mut take2 = take::<usize, &[u8], ContextError>(2usize);
    let trailing = take2.parse_next(&mut input4).unwrap();
    assert_eq!(trailing, b"!!");
    assert!(input4.is_empty());
}