use nom::number::complete::{u24, i24, u128 as nom_u128, i128 as nom_i128, recognize_float_parts};
use nom::number::Endianness;
use nom::IResult;
use nom::error::Error;

#[test]
fn test_u24_big_endian_basic() {
    let parser = u24::<&[u8], Error<&[u8]>>(Endianness::Big);


    let input: &[u8] = &[0x01, 0x02, 0x03, 0xFF];
    let result: IResult<&[u8], u32, Error<&[u8]>> = parser(input);
    let (remaining, value) = result.unwrap();
    assert_eq!(value, 66051u32);
    assert_eq!(remaining, &[0xFF]);


    let input2: &[u8] = &[0x00, 0x00, 0x00];
    let (remaining2, value2) = parser(input2).unwrap();
    assert_eq!(value2, 0u32);
    assert_eq!(remaining2.len(), 0);


    let input3: &[u8] = &[0xFF, 0xFF, 0xFF];
    let (remaining3, value3) = parser(input3).unwrap();
    assert_eq!(value3, 16777215u32);
    assert_eq!(remaining3.len(), 0);


    let input4: &[u8] = &[0x80, 0x00, 0x00, 0xAA, 0xBB];
    let (remaining4, value4) = parser(input4).unwrap();
    assert_eq!(value4, 8388608u32);
    assert_eq!(remaining4, &[0xAA, 0xBB]);
}

#[test]
fn test_u24_little_endian_basic() {
    let parser = u24::<&[u8], Error<&[u8]>>(Endianness::Little);


    let input: &[u8] = &[0x03, 0x02, 0x01, 0xDD];
    let (remaining, value) = parser(input).unwrap();
    assert_eq!(value, 66051u32);
    assert_eq!(remaining, &[0xDD]);


    let input2: &[u8] = &[0xFF, 0xFF, 0xFF];
    let (remaining2, value2) = parser(input2).unwrap();
    assert_eq!(value2, 16777215u32);
    assert_eq!(remaining2.len(), 0);


    let input3: &[u8] = &[0x00, 0x00, 0x80];
    let (remaining3, value3) = parser(input3).unwrap();
    assert_eq!(value3, 8388608u32);
    assert_eq!(remaining3.len(), 0);


    let input4: &[u8] = &[0x01, 0x00, 0x00];
    let (remaining4, value4) = parser(input4).unwrap();
    assert_eq!(value4, 1u32);
    assert_eq!(remaining4.len(), 0);
}

#[test]
fn test_u24_insufficient_input() {
    let parser = u24::<&[u8], Error<&[u8]>>(Endianness::Big);


    let input: &[u8] = &[0x01, 0x02];
    let result = parser(input);
    assert!(result.is_err());


    let input2: &[u8] = &[0x01];
    let result2 = parser(input2);
    assert!(result2.is_err());


    let input3: &[u8] = &[];
    let result3 = parser(input3);
    assert!(result3.is_err());


    let input4: &[u8] = &[0x01, 0x02, 0x03];
    let result4 = parser(input4);
    assert!(result4.is_ok());

    let (rem, val) = result4.unwrap();
    assert_eq!(val, 66051u32);
    assert_eq!(rem.len(), 0);
}

#[test]
fn test_i24_big_endian_positive_and_negative() {
    let parser = i24::<&[u8], Error<&[u8]>>(Endianness::Big);


    let input: &[u8] = &[0x00, 0x00, 0x01];
    let (remaining, value) = parser(input).unwrap();
    assert_eq!(value, 1i32);
    assert_eq!(remaining.len(), 0);


    let input2: &[u8] = &[0x7F, 0xFF, 0xFF];
    let (remaining2, value2) = parser(input2).unwrap();
    assert_eq!(value2, 8388607i32);
    assert_eq!(remaining2.len(), 0);


    let input3: &[u8] = &[0xFF, 0xFF, 0xFF];
    let (remaining3, value3) = parser(input3).unwrap();
    assert_eq!(value3, -1i32);
    assert_eq!(remaining3.len(), 0);


    let input4: &[u8] = &[0x80, 0x00, 0x00];
    let (remaining4, value4) = parser(input4).unwrap();
    assert_eq!(value4, -8388608i32);
    assert_eq!(remaining4.len(), 0);


    let input5: &[u8] = &[0x00, 0x00, 0x00, 0xAA];
    let (remaining5, value5) = parser(input5).unwrap();
    assert_eq!(value5, 0i32);
    assert_eq!(remaining5, &[0xAA]);
}

#[test]
fn test_i24_little_endian_positive_and_negative() {
    let parser = i24::<&[u8], Error<&[u8]>>(Endianness::Little);


    let input: &[u8] = &[0x01, 0x00, 0x00];
    let (remaining, value) = parser(input).unwrap();
    assert_eq!(value, 1i32);
    assert_eq!(remaining.len(), 0);


    let input2: &[u8] = &[0xFF, 0xFF, 0x7F];
    let (remaining2, value2) = parser(input2).unwrap();
    assert_eq!(value2, 8388607i32);
    assert_eq!(remaining2.len(), 0);


    let input3: &[u8] = &[0xFF, 0xFF, 0xFF];
    let (remaining3, value3) = parser(input3).unwrap();
    assert_eq!(value3, -1i32);
    assert_eq!(remaining3.len(), 0);


    let input4: &[u8] = &[0x00, 0x00, 0x80];
    let (remaining4, value4) = parser(input4).unwrap();
    assert_eq!(value4, -8388608i32);
    assert_eq!(remaining4.len(), 0);


    let input5: &[u8] = &[0xFE, 0xFF, 0xFF, 0x11];
    let (remaining5, value5) = parser(input5).unwrap();
    assert_eq!(value5, -2i32);
    assert_eq!(remaining5, &[0x11]);
}

#[test]
fn test_u128_big_endian() {

    let mut input3_arr = [0u8; 17];
    input3_arr[15] = 0x01;
    input3_arr[16] = 0xAA;

    let mut input4_arr = [0u8; 16];
    input4_arr[14] = 0x01;
    input4_arr[15] = 0x00;

    let parser = nom_u128::<&[u8], Error<&[u8]>>(Endianness::Big);


    let input: &[u8] = &[0u8; 16];
    let (remaining, value) = parser(input).unwrap();
    assert_eq!(value, 0u128);
    assert_eq!(remaining.len(), 0);


    let input2: &[u8] = &[0xFF; 16];
    let (remaining2, value2) = parser(input2).unwrap();
    assert_eq!(value2, u128::MAX);
    assert_eq!(remaining2.len(), 0);


    let (remaining3, value3) = parser(&input3_arr).unwrap();
    assert_eq!(value3, 1u128);
    assert_eq!(remaining3, &[0xAA]);


    let (remaining4, value4) = parser(&input4_arr).unwrap();
    assert_eq!(value4, 256u128);
    assert_eq!(remaining4.len(), 0);
}

#[test]
fn test_u128_little_endian() {

    let mut input3_arr = [0u8; 17];
    input3_arr[0] = 0x01;
    input3_arr[16] = 0xBB;

    let mut input4_arr = [0u8; 16];
    input4_arr[1] = 0x01;

    let parser = nom_u128::<&[u8], Error<&[u8]>>(Endianness::Little);


    let input: &[u8] = &[0u8; 16];
    let (remaining, value) = parser(input).unwrap();
    assert_eq!(value, 0u128);
    assert_eq!(remaining.len(), 0);


    let input2: &[u8] = &[0xFF; 16];
    let (remaining2, value2) = parser(input2).unwrap();
    assert_eq!(value2, u128::MAX);
    assert_eq!(remaining2.len(), 0);


    let (remaining3, value3) = parser(&input3_arr).unwrap();
    assert_eq!(value3, 1u128);
    assert_eq!(remaining3, &[0xBB]);


    let (remaining4, value4) = parser(&input4_arr).unwrap();
    assert_eq!(value4, 256u128);
    assert_eq!(remaining4.len(), 0);
}

#[test]
fn test_u128_insufficient_input() {
    let parser = nom_u128::<&[u8], Error<&[u8]>>(Endianness::Big);


    let input: &[u8] = &[0x01; 15];
    let result = parser(input);
    assert!(result.is_err());


    let input2: &[u8] = &[];
    let result2 = parser(input2);
    assert!(result2.is_err());


    let input3: &[u8] = &[0xFF];
    let result3 = parser(input3);
    assert!(result3.is_err());


    let input4: &[u8] = &[0x00; 16];
    let result4 = parser(input4);
    assert!(result4.is_ok());
    let (rem, val) = result4.unwrap();
    assert_eq!(val, 0u128);
    assert_eq!(rem.len(), 0);
}

#[test]
fn test_i128_big_endian() {

    let mut input3_arr = [0xFF; 16];
    input3_arr[0] = 0x7F;

    let mut input4_arr = [0x00; 16];
    input4_arr[0] = 0x80;

    let mut input5_arr = [0u8; 17];
    input5_arr[15] = 0x01;
    input5_arr[16] = 0xCC;

    let parser = nom_i128::<&[u8], Error<&[u8]>>(Endianness::Big);


    let input: &[u8] = &[0u8; 16];
    let (remaining, value) = parser(input).unwrap();
    assert_eq!(value, 0i128);
    assert_eq!(remaining.len(), 0);


    let input2: &[u8] = &[0xFF; 16];
    let (remaining2, value2) = parser(input2).unwrap();
    assert_eq!(value2, -1i128);
    assert_eq!(remaining2.len(), 0);


    let (remaining3, value3) = parser(&input3_arr).unwrap();
    assert_eq!(value3, i128::MAX);
    assert_eq!(remaining3.len(), 0);


    let (remaining4, value4) = parser(&input4_arr).unwrap();
    assert_eq!(value4, i128::MIN);
    assert_eq!(remaining4.len(), 0);


    let (remaining5, value5) = parser(&input5_arr).unwrap();
    assert_eq!(value5, 1i128);
    assert_eq!(remaining5, &[0xCC]);
}

#[test]
fn test_i128_little_endian() {

    let mut input3_arr = [0xFF; 16];
    input3_arr[15] = 0x7F;

    let mut input4_arr = [0x00; 16];
    input4_arr[15] = 0x80;

    let mut input5_arr = [0u8; 17];
    input5_arr[0] = 0x01;
    input5_arr[16] = 0xDD;

    let parser = nom_i128::<&[u8], Error<&[u8]>>(Endianness::Little);


    let input: &[u8] = &[0u8; 16];
    let (remaining, value) = parser(input).unwrap();
    assert_eq!(value, 0i128);
    assert_eq!(remaining.len(), 0);


    let input2: &[u8] = &[0xFF; 16];
    let (remaining2, value2) = parser(input2).unwrap();
    assert_eq!(value2, -1i128);
    assert_eq!(remaining2.len(), 0);


    let (remaining3, value3) = parser(&input3_arr).unwrap();
    assert_eq!(value3, i128::MAX);
    assert_eq!(remaining3.len(), 0);


    let (remaining4, value4) = parser(&input4_arr).unwrap();
    assert_eq!(value4, i128::MIN);
    assert_eq!(remaining4.len(), 0);


    let (remaining5, value5) = parser(&input5_arr).unwrap();
    assert_eq!(value5, 1i128);
    assert_eq!(remaining5, &[0xDD]);
}

#[test]
fn test_recognize_float_parts_simple_integer() {

    let result: IResult<&str, (bool, &str, &str, i32), Error<&str>> =
        recognize_float_parts("123rest");
    let (remaining, (positive, integer, fractional, exponent)) = result.unwrap();
    assert_eq!(remaining, "rest");
    assert_eq!(positive, true);
    assert_eq!(integer, "123");
    assert_eq!(fractional, "");
    assert_eq!(exponent, 0i32);
}

#[test]
fn test_recognize_float_parts_negative() {
    let result: IResult<&str, (bool, &str, &str, i32), Error<&str>> =
        recognize_float_parts("-456.78");
    let (remaining, (positive, integer, fractional, exponent)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(positive, false);
    assert_eq!(integer, "456");
    assert_eq!(fractional, "78");
    assert_eq!(exponent, 0i32);
}

#[test]
fn test_recognize_float_parts_with_exponent() {
    let result: IResult<&str, (bool, &str, &str, i32), Error<&str>> =
        recognize_float_parts("1.5e10");
    let (remaining, (positive, integer, fractional, exponent)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(positive, true);
    assert_eq!(integer, "1");
    assert_eq!(fractional, "5");
    assert_eq!(exponent, 10i32);
}

#[test]
fn test_recognize_float_parts_negative_exponent() {
    let result: IResult<&str, (bool, &str, &str, i32), Error<&str>> =
        recognize_float_parts("3.14e-2tail");
    let (remaining, (positive, integer, fractional, exponent)) = result.unwrap();
    assert_eq!(remaining, "tail");
    assert_eq!(positive, true);
    assert_eq!(integer, "3");
    assert_eq!(fractional, "14");
    assert_eq!(exponent, -2i32);
}

#[test]
fn test_recognize_float_parts_positive_sign() {
    let result: IResult<&str, (bool, &str, &str, i32), Error<&str>> =
        recognize_float_parts("+99.9E3");
    let (remaining, (positive, integer, fractional, exponent)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(positive, true);
    assert_eq!(integer, "99");
    assert_eq!(fractional, "9");
    assert_eq!(exponent, 3i32);
}

#[test]
fn test_recognize_float_parts_only_fractional() {

    let result: IResult<&str, (bool, &str, &str, i32), Error<&str>> =
        recognize_float_parts(".5");
    let (remaining, (positive, integer, fractional, exponent)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(positive, true);
    assert_eq!(integer, "");
    assert_eq!(fractional, "5");
    assert_eq!(exponent, 0i32);
}

#[test]
fn test_u24_i24_roundtrip_boundary_values() {

    let big_parser_u = u24::<&[u8], Error<&[u8]>>(Endianness::Big);
    let big_parser_i = i24::<&[u8], Error<&[u8]>>(Endianness::Big);


    let input: &[u8] = &[0x80, 0x00, 0x00];
    let (_, u_val) = big_parser_u(input).unwrap();
    let (_, i_val) = big_parser_i(input).unwrap();
    assert_eq!(u_val, 8388608u32);
    assert_eq!(i_val, -8388608i32);


    let input2: &[u8] = &[0x7F, 0xFF, 0xFF];
    let (_, u_val2) = big_parser_u(input2).unwrap();
    let (_, i_val2) = big_parser_i(input2).unwrap();
    assert_eq!(u_val2, 8388607u32);
    assert_eq!(i_val2, 8388607i32);


    let input3: &[u8] = &[0xFF, 0xFF, 0xFE];
    let (_, u_val3) = big_parser_u(input3).unwrap();
    let (_, i_val3) = big_parser_i(input3).unwrap();
    assert_eq!(u_val3, 16777214u32);
    assert_eq!(i_val3, -2i32);


    let input4: &[u8] = &[0x00, 0x00, 0x01];
    let (_, u_val4) = big_parser_u(input4).unwrap();
    let (_, i_val4) = big_parser_i(input4).unwrap();
    assert_eq!(u_val4, 1u32);
    assert_eq!(i_val4, 1i32);
}

#[test]
fn test_u128_i128_endianness_consistency() {

    let be_parser = nom_u128::<&[u8], Error<&[u8]>>(Endianness::Big);
    let le_parser = nom_u128::<&[u8], Error<&[u8]>>(Endianness::Little);


    let be_input: &[u8] = &[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
    ];
    let (_, be_val) = be_parser(be_input).unwrap();


    let le_input: &[u8] = &[
        0x10, 0x0F, 0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09,
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
    ];
    let (_, le_val) = le_parser(le_input).unwrap();

    assert_eq!(be_val, le_val);
    assert_eq!(be_val, 0x0102030405060708090A0B0C0D0E0F10u128);


    let expected: u128 = (0x01u128 << 120) | (0x02u128 << 112) | (0x03u128 << 104)
        | (0x04u128 << 96) | (0x05u128 << 88) | (0x06u128 << 80)
        | (0x07u128 << 72) | (0x08u128 << 64) | (0x09u128 << 56)
        | (0x0Au128 << 48) | (0x0Bu128 << 40) | (0x0Cu128 << 32)
        | (0x0Du128 << 24) | (0x0Eu128 << 16) | (0x0Fu128 << 8)
        | 0x10u128;
    assert_eq!(be_val, expected);
    assert_eq!(le_val, expected);
}

#[test]
fn test_recognize_float_parts_various_formats() {

    let result: IResult<&str, (bool, &str, &str, i32), Error<&str>> =
        recognize_float_parts("42.");
    let (remaining, (positive, integer, fractional, exponent)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(positive, true);
    assert_eq!(integer, "42");
    assert_eq!(fractional, "");
    assert_eq!(exponent, 0i32);


    let result2: IResult<&str, (bool, &str, &str, i32), Error<&str>> =
        recognize_float_parts("1e100");
    let (remaining2, (positive2, integer2, fractional2, exponent2)) = result2.unwrap();
    assert_eq!(remaining2, "");
    assert_eq!(positive2, true);
    assert_eq!(integer2, "1");
    assert_eq!(fractional2, "");
    assert_eq!(exponent2, 100i32);
}