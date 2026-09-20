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

    // mod_word is non-mutating and returns remainder
    let modval = n.mod_word(7).unwrap();
    assert_eq!(modval, 3u64); // 1200 % 7 == 3
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1200");

    // div_word mutates to quotient, returns remainder
    let rem = n.div_word(7).unwrap();
    assert_eq!(rem, 3u64);
    assert_eq!(n.to_dec_str().unwrap().to_string(), "171"); // 1200 / 7 == 171

    // Confirm further arithmetic composes correctly
    n.mul_word(2).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "342");

    n.sub_word(342).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "0");
    assert_eq!(n.num_bits(), 0);
}

#[test]
fn test_word_arithmetic_large_values() {
    // Start with a large number and apply word operations
    let mut n = BigNum::from_dec_str("100000000000000000000").unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "100000000000000000000");

    n.add_word(12345).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "100000000000000012345");

    n.sub_word(12345).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "100000000000000000000");

    // mod_word on large number: 10^20 mod 7
    // 10 mod 7 = 3; 10^20 mod 7 computed via pow: 3^20 mod 7
    // 3^6 = 729 = 7*104+1 => 3^6 mod 7 = 1, so 3^20 = 3^(6*3+2) mod 7 = 3^2 = 9 mod 7 = 2
    let m = n.mod_word(7).unwrap();
    assert_eq!(m, 2u64);
    // non-mutating
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

    // Build 0b10101 == 21
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
    assert_eq!(n.to_dec_str().unwrap().to_string(), "17"); // 0b10001

    // Set far-away bit: grows the number
    n.set_bit(100).unwrap();
    assert_eq!(n.is_bit_set(100), true);
    assert_eq!(n.num_bits(), 101);

    // clear() resets to zero
    n.clear();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "0");
    assert_eq!(n.is_bit_set(0), false);
    assert_eq!(n.is_bit_set(100), false);
    assert_eq!(n.num_bits(), 0);

    // After clear we can reuse the number
    n.set_bit(3).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "8");
    assert_eq!(n.is_bit_set(3), true);
}

#[test]
fn test_mask_bits_truncation() {
    let mut a = BigNum::from_u32(0xFF).unwrap();
    let hex_a = a.to_hex_str().unwrap().to_string().to_lowercase();
    // to_hex_str may return leading zeros or uppercase; strip leading zeros
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

    let mut c = BigNum::from_dec_str("1023").unwrap(); // 0x3FF
    assert_eq!(c.num_bits(), 10);
    c.mask_bits(5).unwrap();
    assert_eq!(c.to_dec_str().unwrap().to_string(), "31");
    assert_eq!(c.num_bits(), 5);

    // Mask to the same width it already has (at least 1 bit left)
    let mut d = BigNum::from_u32(0b1101_0110).unwrap(); // 214, 8 bits
    d.mask_bits(6).unwrap(); // keep low 6: 0b010110 = 22
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

    // Chain left shifts to build powers of 2
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

    // Right shift back down
    let mut back_four = BigNum::new().unwrap();
    back_four.rshift1(&eight).unwrap();
    assert_eq!(back_four.to_dec_str().unwrap().to_string(), "4");

    // Right shift of an odd number drops the low bit
    let seven = BigNum::from_u32(7).unwrap();
    let mut three = BigNum::new().unwrap();
    three.rshift1(&seven).unwrap();
    assert_eq!(three.to_dec_str().unwrap().to_string(), "3");

    // Source unchanged by shift-into-other operations
    assert_eq!(a.to_dec_str().unwrap().to_string(), "42");
    assert_eq!(seven.to_dec_str().unwrap().to_string(), "7");
}

#[test]
fn test_shift_large_number() {
    // 2^64 == 18446744073709551616
    let base = BigNum::from_dec_str("18446744073709551616").unwrap();
    assert_eq!(base.num_bits(), 65);

    let mut shifted = BigNum::new().unwrap();
    shifted.lshift1(&base).unwrap();
    // 2^65
    assert_eq!(shifted.to_dec_str().unwrap().to_string(), "36893488147419103232");
    assert_eq!(shifted.num_bits(), 66);

    let mut back = BigNum::new().unwrap();
    back.rshift1(&shifted).unwrap();
    assert_eq!(back.to_dec_str().unwrap().to_string(), "18446744073709551616");
    assert_eq!(back.num_bits(), 65);

    // rshift1 of base again => 2^63
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
    assert_eq!(sum.to_dec_str().unwrap().to_string(), "0"); // pre-state
    sum.checked_add(&a, &b).unwrap();
    assert_eq!(sum.to_dec_str().unwrap().to_string(), "1234");

    let mut diff = BigNum::new().unwrap();
    diff.checked_sub(&a, &b).unwrap();
    assert_eq!(diff.to_dec_str().unwrap().to_string(), "766");

    // Operands are not mutated
    assert_eq!(a.to_dec_str().unwrap().to_string(), "1000");
    assert_eq!(b.to_dec_str().unwrap().to_string(), "234");

    // Large numbers
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

    // Negative result: a - x where a=1000 and x is huge
    let mut neg = BigNum::new().unwrap();
    neg.checked_sub(&a, &x).unwrap();
    assert_eq!(
        neg.to_dec_str().unwrap().to_string(),
        "-123456789012345678901234566890"
    );

    // sum - a == b (round-trip check)
    let mut round = BigNum::new().unwrap();
    round.checked_sub(&sum, &a).unwrap();
    assert_eq!(round.to_dec_str().unwrap().to_string(), "234");
}

#[test]
fn test_pseudo_rand_msb_modes() {
    // MAYBE_ZERO: top bit may be zero, bits is an upper bound
    let mut n = BigNum::new().unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "0");
    n.pseudo_rand(64, MsbOption::MAYBE_ZERO, false).unwrap();
    assert!(n.num_bits() <= 64);

    // ONE: top bit is set => exactly `bits` bits
    let mut m = BigNum::new().unwrap();
    m.pseudo_rand(128, MsbOption::ONE, true).unwrap();
    assert_eq!(m.num_bits(), 128);
    assert_eq!(m.is_bit_set(127), true);
    // odd => low bit set
    assert_eq!(m.is_bit_set(0), true);

    // TWO_ONES: top two bits set
    let mut k = BigNum::new().unwrap();
    k.pseudo_rand(256, MsbOption::TWO_ONES, false).unwrap();
    assert_eq!(k.num_bits(), 256);
    assert_eq!(k.is_bit_set(255), true);
    assert_eq!(k.is_bit_set(254), true);

    // Two independent calls produce different values with overwhelming probability
    let mut r1 = BigNum::new().unwrap();
    let mut r2 = BigNum::new().unwrap();
    r1.pseudo_rand(128, MsbOption::ONE, false).unwrap();
    r2.pseudo_rand(128, MsbOption::ONE, false).unwrap();
    assert_ne!(
        r1.to_hex_str().unwrap().to_string(),
        r2.to_hex_str().unwrap().to_string()
    );

    // Reuse via clear(): pseudo_rand works after clear
    k.clear();
    assert_eq!(k.to_dec_str().unwrap().to_string(), "0");
    k.pseudo_rand(32, MsbOption::ONE, true).unwrap();
    assert_eq!(k.num_bits(), 32);
    assert_eq!(k.is_bit_set(31), true);
    assert_eq!(k.is_bit_set(0), true);
}

#[test]
fn test_combined_workflow_bits_and_arithmetic() {
    // Build 2^10 = 1024 via set_bit, verify, then do word arithmetic
    let mut n = BigNum::new().unwrap();
    n.set_bit(10).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1024");
    assert_eq!(n.num_bits(), 11);
    assert_eq!(n.is_bit_set(10), true);
    assert_eq!(n.is_bit_set(0), false);

    // 1024 + 1 = 1025
    n.add_word(1).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1025");
    assert_eq!(n.is_bit_set(0), true);

    // mod_word: 1025 mod 1000 == 25
    assert_eq!(n.mod_word(1000).unwrap(), 25u64);

    // mask_bits keep low 4: 1025 == 0x401, low 4 bits = 1
    n.mask_bits(4).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "1");
    assert_eq!(n.num_bits(), 1);

    // Multiply by 7 => 7
    n.mul_word(7).unwrap();
    assert_eq!(n.to_dec_str().unwrap().to_string(), "7");

    // Right shift by one: 7 >> 1 == 3
    let mut half = BigNum::new().unwrap();
    half.rshift1(&n).unwrap();
    assert_eq!(half.to_dec_str().unwrap().to_string(), "3");

    // Left shift by one: 3 << 1 == 6
    let mut twice = BigNum::new().unwrap();
    twice.lshift1(&half).unwrap();
    assert_eq!(twice.to_dec_str().unwrap().to_string(), "6");

    // checked_add of twice + half => 9
    let mut total = BigNum::new().unwrap();
    total.checked_add(&twice, &half).unwrap();
    assert_eq!(total.to_dec_str().unwrap().to_string(), "9");

    // checked_sub: total - twice => 3
    let mut back = BigNum::new().unwrap();
    back.checked_sub(&total, &twice).unwrap();
    assert_eq!(back.to_dec_str().unwrap().to_string(), "3");

    // Finally clear and ensure zero
    total.clear();
    assert_eq!(total.to_dec_str().unwrap().to_string(), "0");
    assert_eq!(total.num_bits(), 0);
}