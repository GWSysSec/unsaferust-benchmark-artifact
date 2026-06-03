use encoding_rs::mem::convert_latin1_to_str;
use encoding_rs::mem::convert_latin1_to_str_partial;
use encoding_rs::mem::convert_latin1_to_utf8_partial;
use encoding_rs::mem::convert_utf16_to_str;

#[test]
fn test_partial_and_utf16_str_conversions() {
    let latin1: &[u8] = &[0x48, 0xE9, 0x6C, 0x6C];


    let mut big = vec![0u8; latin1.len() * 2];
    let dst_str2 = unsafe { std::str::from_utf8_unchecked_mut(&mut big) };
    let written2 = convert_latin1_to_str(latin1, dst_str2);
    assert!(written2 > 0);
    assert!(written2 <= big.len());


    let u16_data: Vec<u16> = "Hé".encode_utf16().collect();
    let mut str_dst = vec![0u8; u16_data.len() * 3];
    let dst_s = unsafe { std::str::from_utf8_unchecked_mut(&mut str_dst) };
    let n = convert_utf16_to_str(&u16_data, dst_s);
    assert!(n > 0);
    assert!(n <= str_dst.len());


    let mut small = vec![0u8; 2];
    let dst_str_small = unsafe { std::str::from_utf8_unchecked_mut(&mut small) };
    let (read, written_small) = convert_latin1_to_str_partial(latin1, dst_str_small);
    assert!(read <= latin1.len());
    assert!(written_small <= small.len());


    let u16_long: Vec<u16> = "Hello, 世界".encode_utf16().collect();
    let mut tight = vec![0u8; u16_long.len() * 3];
    let dst_tight = unsafe { std::str::from_utf8_unchecked_mut(&mut tight) };
    let n_long = convert_utf16_to_str(&u16_long, dst_tight);
    assert!(n_long > 0);


    let mut raw_dst = vec![0u8; 3];
    let (r_raw, w_raw) = convert_latin1_to_utf8_partial(latin1, &mut raw_dst);
    assert!(r_raw <= latin1.len());
    assert!(w_raw <= raw_dst.len());


    let exact_dst_len = latin1.len() * 2;
    let mut exact = vec![0u8; exact_dst_len];
    let dst_exact = unsafe { std::str::from_utf8_unchecked_mut(&mut exact) };
    let w_exact = convert_latin1_to_str(latin1, dst_exact);
    assert!(w_exact > 0);
    assert!(w_exact <= exact_dst_len);
}