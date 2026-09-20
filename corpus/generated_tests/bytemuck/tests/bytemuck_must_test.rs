use bytemuck::{cast_mut, cast_slice_mut};

#[test]
fn cast_mut_reinterprets_and_mutates_single_value_bytes() {
    let initial = 0x1122_3344_u32;
    let mut value = initial;
    let mut expected_bytes = initial.to_ne_bytes();

    {
        let bytes: &mut [u8; 4] = cast_mut::<u32, [u8; 4]>(&mut value);
        assert_eq!(*bytes, expected_bytes);

        bytes[0] ^= 0xFF;
        bytes[3] = 0xA5;
        expected_bytes[0] ^= 0xFF;
        expected_bytes[3] = 0xA5;

        assert_eq!(*bytes, expected_bytes);
    }

    assert_eq!(value, u32::from_ne_bytes(expected_bytes));

    {
        let bytes_again: &mut [u8; 4] = cast_mut::<u32, [u8; 4]>(&mut value);
        bytes_again.copy_from_slice(&[1, 2, 3, 4]);
    }

    assert_eq!(value, u32::from_ne_bytes([1, 2, 3, 4]));
}

#[test]
fn cast_slice_mut_exposes_contiguous_bytes_for_multi_value_workflow() {
    let mut words = [0x0102_0304_u32, 0x1112_1314_u32, 0xA1A2_A3A4_u32];
    let original_words = words;
    let mut expected_bytes = Vec::new();
    for word in original_words {
        expected_bytes.extend_from_slice(&word.to_ne_bytes());
    }

    {
        let bytes: &mut [u8] = cast_slice_mut::<u32, u8>(&mut words);
        assert_eq!(bytes.len(), 3 * core::mem::size_of::<u32>());
        assert_eq!(bytes, expected_bytes.as_slice());

        let replacement = 0xDEAD_BEEF_u32.to_ne_bytes();
        bytes[4..8].copy_from_slice(&replacement);
        expected_bytes[4..8].copy_from_slice(&replacement);

        assert_eq!(&bytes[4..8], replacement.as_slice());
        assert_eq!(bytes, expected_bytes.as_slice());
    }

    assert_eq!(words[0], original_words[0]);
    assert_eq!(words[1], 0xDEAD_BEEF_u32);
    assert_eq!(words[2], original_words[2]);
}

#[test]
fn cast_slice_mut_handles_empty_slices_and_round_trip_byte_edits() {
    let mut empty_words: [u32; 0] = [];
    let empty_bytes: &mut [u8] = cast_slice_mut::<u32, u8>(&mut empty_words);
    assert!(empty_bytes.is_empty());

    let mut words = [0_u32, u32::MAX];
    {
        let bytes: &mut [u8] = cast_slice_mut::<u32, u8>(&mut words);
        assert_eq!(bytes.len(), 8);

        bytes[..4].copy_from_slice(&0x5566_7788_u32.to_ne_bytes());
        bytes[4..].copy_from_slice(&0x99AA_BBCC_u32.to_ne_bytes());
    }

    assert_eq!(words, [0x5566_7788_u32, 0x99AA_BBCC_u32]);
}