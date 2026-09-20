extern crate brotli_decompressor;

use brotli_decompressor::HuffmanCode;

#[test]
fn test_huffman_code_eq_identical_values() {
    let code1 = HuffmanCode {
        bits: 5,
        value: 42,
    };
    let code2 = HuffmanCode {
        bits: 5,
        value: 42,
    };

    // Two codes with identical bits and value should be equal
    assert_eq!(code1.eq(&code2), true);
    assert_eq!(code2.eq(&code1), true);
    assert!(code1.eq(&code2));
    assert!(code2.eq(&code1));

    // Reflexivity: a code equals itself
    assert_eq!(code1.eq(&code1), true);
    assert_eq!(code2.eq(&code2), true);

    // PartialEq via == operator
    assert!(code1 == code2);
    assert!(!(code1 != code2));
}

#[test]
fn test_huffman_code_eq_different_bits() {
    let code1 = HuffmanCode {
        bits: 3,
        value: 100,
    };
    let code2 = HuffmanCode {
        bits: 7,
        value: 100,
    };

    // Same value but different bits should NOT be equal
    assert_eq!(code1.eq(&code2), false);
    assert_eq!(code2.eq(&code1), false);
    assert!(code1 != code2);
    assert!(code2 != code1);

    // Each still equals itself
    assert_eq!(code1.eq(&code1), true);
    assert_eq!(code2.eq(&code2), true);

    // Operator check
    assert!(code1 != code2);
    assert!(!(code1 == code2));
}

#[test]
fn test_huffman_code_eq_different_values() {
    let code1 = HuffmanCode {
        bits: 10,
        value: 200,
    };
    let code2 = HuffmanCode {
        bits: 10,
        value: 201,
    };

    // Same bits but different value should NOT be equal
    assert_eq!(code1.eq(&code2), false);
    assert_eq!(code2.eq(&code1), false);
    assert!(code1 != code2);
    assert!(code2 != code1);

    // Reflexivity
    assert_eq!(code1.eq(&code1), true);
    assert_eq!(code2.eq(&code2), true);

    // Verify the fields are what we set
    assert_eq!(code1.bits, 10);
    assert_eq!(code2.value, 201);
}

#[test]
fn test_huffman_code_eq_both_fields_different() {
    let code1 = HuffmanCode {
        bits: 1,
        value: 0,
    };
    let code2 = HuffmanCode {
        bits: 15,
        value: 65535,
    };

    // Both fields differ
    assert_eq!(code1.eq(&code2), false);
    assert_eq!(code2.eq(&code1), false);
    assert!(code1 != code2);

    // Verify field values
    assert_eq!(code1.bits, 1);
    assert_eq!(code1.value, 0);
    assert_eq!(code2.bits, 15);
    assert_eq!(code2.value, 65535);

    // Reflexivity still holds
    assert_eq!(code1.eq(&code1), true);
    assert_eq!(code2.eq(&code2), true);
}

#[test]
fn test_huffman_code_eq_boundary_values() {
    // Test with zero values
    let zero_code = HuffmanCode {
        bits: 0,
        value: 0,
    };
    let another_zero = HuffmanCode {
        bits: 0,
        value: 0,
    };
    assert_eq!(zero_code.eq(&another_zero), true);
    assert!(zero_code == another_zero);

    // Test with max u8 bits and max u16 value
    let max_code = HuffmanCode {
        bits: 255,
        value: 65535,
    };
    let max_code_copy = HuffmanCode {
        bits: 255,
        value: 65535,
    };
    assert_eq!(max_code.eq(&max_code_copy), true);
    assert!(max_code == max_code_copy);

    // Zero vs max should not be equal
    assert_eq!(zero_code.eq(&max_code), false);
    assert!(zero_code != max_code);

    // Symmetry between zero and max
    assert_eq!(max_code.eq(&zero_code), false);
    assert!(max_code != zero_code);
}

#[test]
fn test_huffman_code_eq_transitivity() {
    let a = HuffmanCode {
        bits: 8,
        value: 128,
    };
    let b = HuffmanCode {
        bits: 8,
        value: 128,
    };
    let c = HuffmanCode {
        bits: 8,
        value: 128,
    };

    // Transitivity: if a == b and b == c, then a == c
    assert_eq!(a.eq(&b), true);
    assert_eq!(b.eq(&c), true);
    assert_eq!(a.eq(&c), true);

    // All pairwise
    assert!(a == b);
    assert!(b == c);
    assert!(a == c);

    // Introduce a different one to break the chain
    let d = HuffmanCode {
        bits: 8,
        value: 129,
    };
    assert_eq!(a.eq(&d), false);
    assert!(a != d);
}

#[test]
fn test_huffman_code_eq_collection_of_codes() {
    // Create a collection and verify equality semantics across multiple instances
    let codes: Vec<HuffmanCode> = (0..16u16).map(|i| HuffmanCode {
        bits: (i as u8) + 1,
        value: i * 10,
    }).collect();

    // Each code should equal itself
    for code in &codes {
        assert_eq!(code.eq(code), true);
    }

    // Adjacent codes should differ (different bits and values)
    for i in 0..15 {
        assert_eq!(codes[i].eq(&codes[i + 1]), false);
        assert!(codes[i] != codes[i + 1]);
    }

    // First and last should differ
    assert_eq!(codes[0].eq(&codes[15]), false);
    assert!(codes[0] != codes[15]);

    // Verify specific field values
    assert_eq!(codes[0].bits, 1);
    assert_eq!(codes[0].value, 0);
    assert_eq!(codes[15].bits, 16);
    assert_eq!(codes[15].value, 150);

    // A clone of codes[5] should equal codes[5]
    let clone_5 = HuffmanCode {
        bits: codes[5].bits,
        value: codes[5].value,
    };
    assert_eq!(clone_5.eq(&codes[5]), true);
    assert!(clone_5 == codes[5]);
}

#[test]
fn test_huffman_code_eq_single_bit_difference() {
    // Test where only the lowest bit of value differs
    let code_a = HuffmanCode {
        bits: 4,
        value: 0b1010_1010,
    };
    let code_b = HuffmanCode {
        bits: 4,
        value: 0b1010_1011,
    };

    assert_eq!(code_a.eq(&code_b), false);
    assert!(code_a != code_b);
    assert_eq!(code_a.eq(&code_a), true);
    assert_eq!(code_b.eq(&code_b), true);

    // Test where only the lowest bit of bits differs
    let code_c = HuffmanCode {
        bits: 0b0000_1110,
        value: 500,
    };
    let code_d = HuffmanCode {
        bits: 0b0000_1111,
        value: 500,
    };

    assert_eq!(code_c.eq(&code_d), false);
    assert!(code_c != code_d);
    assert_eq!(code_c.eq(&code_c), true);
    assert_eq!(code_d.eq(&code_d), true);
}