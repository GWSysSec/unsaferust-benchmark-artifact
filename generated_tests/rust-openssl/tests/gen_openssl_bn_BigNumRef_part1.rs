use openssl::bn::{BigNum, MsbOption};

#[test]
fn test_word_arithmetic_workflow() {
    let mut n = BigNum::from_u32(100).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "100");
    assert_eq!(n.num_bits(), 7);

    n.add_word(50).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "150");

    n.sub_word(30).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "120");

    n.mul_word(10).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1200");


    let modval = n.mod_word(7).unwrap();
    assert_eq!(modval, 3u64);
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1200");


    let rem = n.div_word(7).unwrap();
    assert_eq!(rem, 3u64);
    assert_eq!(n.to_dec_str().unwrap().to_string(), "171");


    n.mul_word(2).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "342");

    n.sub_word(342).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "0");
    assert_eq!(n.num_bits(), 0);
}

#[test]
fn test_word_arithmetic_large_values() {

    let mut n = BigNum::from_dec_str("100000000000000000000").unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "100000000000000000000");

    n.add_word(12345).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "100000000000000012345");

    n.sub_word(12345).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "100000000000000000000");




    let m = n.mod_word(7).unwrap();
    assert_eq!(m, 2u64);

    assert_eq!(n.to_dec_str().unwrap().to_string(), "100000000000000000000");

    n.mul_word(3).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "300000000000000000000");

    let r = n.div_word(3).unwrap();
    assert_eq!(r, 0u64);
    assert_eq!(n.to_dec_str().unwrap().to_string(), "100000000000000000000");
}

#[test]
fn test_bit_set_clear_is_set_and_clear() {
    let mut n = BigNum::new().unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "0");
    assert_eq!(n.is_bit_set(0), false);
    assert_eq!(n.is_bit_set(100), false);


    n.set_bit(0).unwrap();
    n.set_bit(2).unwrap();
    n.set_bit(4).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "21");

    assert_eq!(n.is_bit_set(0), true);
    assert_eq!(n.is_bit_set(1), false);
    assert_eq!(n.is_bit_set(2), true);
    assert_eq!(n.is_bit_set(3), false);
    assert_eq!(n.is_bit_set(4), true);
    assert_eq!(n.is_bit_set(5), false);

    n.clear_bit(2).unwrap();
    assert_eq!(n.is_bit_set(2), false);
    assert_eq!(n.to_dec_str().unwrap().to_string(), "17");


    n.set_bit(100).unwrap();
    assert_eq!(n.is_bit_set(100), true);
    assert_eq!(n.num_bits(), 101);


    n.clear();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "0");
    assert_eq!(n.is_bit_set(0), false);
    assert_eq!(n.is_bit_set(100), false);
    assert_eq!(n.num_bits(), 0);


    n.set_bit(3).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "8");
    assert_eq!(n.is_bit_set(3), true);
}

#[test]
fn test_mask_bits_truncation() {
    let mut a = BigNum::from_u32(0xFF).unwrap();
    let hex_a = a.to_hex_str().unwrap().to_string().to_lowercase();

    let hex_a_trimmed = hex_a.trim_start_matches('0');
    let hex_a_trimmed = if hex_a_trimmed.is_empty() { "0" } else { hex_a_trimmed };
    assert_eq!(hex_a_trimmed, "ff");
    assert_eq!(a.num_bits(), 8);

    a.mask_bits(4).unwrap();
    let hex_masked = a.to_hex_str().unwrap().to_string().to_lowercase();
    let hex_masked_trimmed = hex_masked.trim_start_matches('0');
    let hex_masked_trimmed = if hex_masked_trimmed.is_empty() { "0" } else { hex_masked_trimmed };
    assert_eq!(hex_masked_trimmed, "f");
    assert_eq!(a.to_dec_str().unwrap().to_string(), "15");
    assert_eq!(a.num_bits(), 4);

    let mut b = BigNum::from_u32(0xDEAD).unwrap();
    assert_eq!(b.num_bits(), 16);
    b.mask_bits(8).unwrap();
    let hex_b = b.to_hex_str().unwrap().to_string().to_lowercase();
    let hex_b_trimmed = hex_b.trim_start_matches('0');
    let hex_b_trimmed = if hex_b_trimmed.is_empty() { "0" } else { hex_b_trimmed };
    assert_eq!(hex_b_trimmed, "ad");
    assert_eq!(b.num_bits(), 8);

    let mut c = BigNum::from_dec_str("1023").unwrap();
    assert_eq!(c.num_bits(), 10);
    c.mask_bits(5).unwrap();
    assert_eq!(c.to_dec_str().unwrap().to_string(), "31");
    assert_eq!(c.num_bits(), 5);


    let mut d = BigNum::from_u32(0b1101_0110).unwrap();
    d.mask_bits(6).unwrap();
    assert_eq!(d.to_dec_str().unwrap().to_string(), "22");
}

#[test]
fn test_shift_by_one_operations() {
    let a = BigNum::from_u32(42).unwrap();
    assert_eq!(a.to_dec_str().unwrap().to_string(), "42");

    let mut doubled = BigNum::new().unwrap();
    doubled.lshift1(&a).unwrap();
    assert_eq!(doubled.to_dec_str().unwrap().to_string(), "84");

    let mut halved = BigNum::new().unwrap();
    halved.rshift1(&a).unwrap();
    assert_eq!(halved.to_dec_str().unwrap().to_string(), "21");


    let one = BigNum::from_u32(1).unwrap();
    let mut two = BigNum::new().unwrap();
    two.lshift1(&one).unwrap();
    assert_eq!(two.to_dec_str().unwrap().to_string(), "2");

    let mut four = BigNum::new().unwrap();
    four.lshift1(&two).unwrap();
    assert_eq!(four.to_dec_str().unwrap().to_string(), "4");

    let mut eight = BigNum::new().unwrap();
    eight.lshift1(&four).unwrap();
    assert_eq!(eight.to_dec_str().unwrap().to_string(), "8");


    let mut back_four = BigNum::new().unwrap();
    back_four.rshift1(&eight).unwrap();
    assert_eq!(back_four.to_dec_str().unwrap().to_string(), "4");


    let seven = BigNum::from_u32(7).unwrap();
    let mut three = BigNum::new().unwrap();
    three.rshift1(&seven).unwrap();
    assert_eq!(three.to_dec_str().unwrap().to_string(), "3");


    assert_eq!(a.to_dec_str().unwrap().to_string(), "42");
    assert_eq!(seven.to_dec_str().unwrap().to_string(), "7");
}

#[test]
fn test_shift_large_number() {

    let base = BigNum::from_dec_str("18446744073709551616").unwrap();
    assert_eq!(base.num_bits(), 65);

    let mut shifted = BigNum::new().unwrap();
    shifted.lshift1(&base).unwrap();

    assert_eq!(shifted.to_dec_str().unwrap().to_string(), "36893488147419103232");
    assert_eq!(shifted.num_bits(), 66);

    let mut back = BigNum::new().unwrap();
    back.rshift1(&shifted).unwrap();
    assert_eq!(back.to_dec_str().unwrap().to_string(), "18446744073709551616");
    assert_eq!(back.num_bits(), 65);


    let mut half = BigNum::new().unwrap();
    half.rshift1(&base).unwrap();
    assert_eq!(half.to_dec_str().unwrap().to_string(), "9223372036854775808");
    assert_eq!(half.num_bits(), 64);
}

#[test]
fn test_checked_add_and_sub() {
    let a = BigNum::from_u32(1000).unwrap();
    let b = BigNum::from_u32(234).unwrap();

    let mut sum = BigNum::new().unwrap();
    assert_eq!(sum.to_dec_str().unwrap().to_string(), "0");
    sum.checked_add(&a, &b).unwrap();
    assert_eq!(sum.to_dec_str().unwrap().to_string(), "1234");

    let mut diff = BigNum::new().unwrap();
    diff.checked_sub(&a, &b).unwrap();
    assert_eq!(diff.to_dec_str().unwrap().to_string(), "766");


    assert_eq!(a.to_dec_str().unwrap().to_string(), "1000");
    assert_eq!(b.to_dec_str().unwrap().to_string(), "234");


    let x = BigNum::from_dec_str("123456789012345678901234567890").unwrap();
    let y = BigNum::from_dec_str("987654321098765432109876543210").unwrap();

    let mut large_sum = BigNum::new().unwrap();
    large_sum.checked_add(&x, &y).unwrap();
    assert_eq!(
        large_sum.to_dec_str().unwrap().to_string(),
        "1111111110111111111011111111100"
    );

    let mut large_diff = BigNum::new().unwrap();
    large_diff.checked_sub(&y, &x).unwrap();
    assert_eq!(
        large_diff.to_dec_str().unwrap().to_string(),
        "864197532086419753208641975320"
    );


    let mut neg = BigNum::new().unwrap();
    neg.checked_sub(&a, &x).unwrap();
    assert_eq!(
        neg.to_dec_str().unwrap().to_string(),
        "-123456789012345678901234566890"
    );


    let mut round = BigNum::new().unwrap();
    round.checked_sub(&sum, &a).unwrap();
    assert_eq!(round.to_dec_str().unwrap().to_string(), "234");
}

#[test]
fn test_pseudo_rand_msb_modes() {

    let mut n = BigNum::new().unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "0");
    n.pseudo_rand(64, MsbOption::MAYBE_ZERO, false).unwrap();
    assert!(n.num_bits() <= 64);


    let mut m = BigNum::new().unwrap();
    m.pseudo_rand(128, MsbOption::ONE, true).unwrap();
    assert_eq!(m.num_bits(), 128);
    assert_eq!(m.is_bit_set(127), true);

    assert_eq!(m.is_bit_set(0), true);


    let mut k = BigNum::new().unwrap();
    k.pseudo_rand(256, MsbOption::TWO_ONES, false).unwrap();
    assert_eq!(k.num_bits(), 256);
    assert_eq!(k.is_bit_set(255), true);
    assert_eq!(k.is_bit_set(254), true);


    let mut r1 = BigNum::new().unwrap();
    let mut r2 = BigNum::new().unwrap();
    r1.pseudo_rand(128, MsbOption::ONE, false).unwrap();
    r2.pseudo_rand(128, MsbOption::ONE, false).unwrap();
    assert_ne!(
        r1.to_hex_str().unwrap().to_string(),
        r2.to_hex_str().unwrap().to_string()
    );


    k.clear();
    assert_eq!(k.to_dec_str().unwrap().to_string(), "0");
    k.pseudo_rand(32, MsbOption::ONE, true).unwrap();
    assert_eq!(k.num_bits(), 32);
    assert_eq!(k.is_bit_set(31), true);
    assert_eq!(k.is_bit_set(0), true);
}

#[test]
fn test_combined_workflow_bits_and_arithmetic() {

    let mut n = BigNum::new().unwrap();
    n.set_bit(10).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1024");
    assert_eq!(n.num_bits(), 11);
    assert_eq!(n.is_bit_set(10), true);
    assert_eq!(n.is_bit_set(0), false);


    n.add_word(1).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1025");
    assert_eq!(n.is_bit_set(0), true);


    assert_eq!(n.mod_word(1000).unwrap(), 25u64);


    n.mask_bits(4).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1");
    assert_eq!(n.num_bits(), 1);


    n.mul_word(7).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "7");


    let mut half = BigNum::new().unwrap();
    half.rshift1(&n).unwrap();
    assert_eq!(half.to_dec_str().unwrap().to_string(), "3");


    let mut twice = BigNum::new().unwrap();
    twice.lshift1(&half).unwrap();
    assert_eq!(twice.to_dec_str().unwrap().to_string(), "6");


    let mut total = BigNum::new().unwrap();
    total.checked_add(&twice, &half).unwrap();
    assert_eq!(total.to_dec_str().unwrap().to_string(), "9");


    let mut back = BigNum::new().unwrap();
    back.checked_sub(&total, &twice).unwrap();
    assert_eq!(back.to_dec_str().unwrap().to_string(), "3");


    total.clear();
    assert_eq!(total.to_dec_str().unwrap().to_string(), "0");
    assert_eq!(total.num_bits(), 0);
}