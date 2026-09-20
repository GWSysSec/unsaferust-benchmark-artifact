use std::convert::TryInto;

use siphasher::sip128::SipHasher;

fn key_words_from_bytes(key: &[u8; 16]) -> (u64, u64) {
    let mut low = [0_u8; 8];
    let mut high = [0_u8; 8];
    low.copy_from_slice(&key[..8]);
    high.copy_from_slice(&key[8..]);
    (u64::from_le_bytes(low), u64::from_le_bytes(high))
}

fn assert_hash128_views_are_consistent(hash: siphasher::sip128::Hash128) {
    let bytes = hash.as_bytes();
    let (low, high) = hash.as_u64();
    let combined = hash.as_u128();

    assert_eq!(low, u64::from_le_bytes(bytes[..8].try_into().unwrap()));
    assert_eq!(high, u64::from_le_bytes(bytes[8..].try_into().unwrap()));
    assert_eq!(combined as u64, low);
    assert_eq!((combined >> 64) as u64, high);
    assert_eq!(combined, (low as u128) | ((high as u128) << 64));
}

#[test]
fn sip128_hasher_keys_and_key_round_trip_from_word_keys() {
    let key0 = 0x0706_0504_0302_0100_u64;
    let key1 = 0x0f0e_0d0c_0b0a_0908_u64;
    let expected_key_bytes = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
        0x0e, 0x0f,
    ];

    let hasher = SipHasher::new_with_keys(key0, key1);

    assert_eq!(hasher.keys(), (key0, key1));
    assert_eq!(hasher.key(), expected_key_bytes);
    assert_eq!(key_words_from_bytes(&hasher.key()), hasher.keys());

    let reconstructed = SipHasher::new_with_key(&hasher.key());

    assert_eq!(reconstructed.keys(), hasher.keys());
    assert_eq!(reconstructed.key(), hasher.key());

    let messages: [&[u8]; 5] = [
        b"",
        b"a",
        b"siphash-128 keyed hashing",
        b"round-tripping key material should preserve hash output",
        &[0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233],
    ];

    for message in messages {
        let original_hash = hasher.hash(message);
        let reconstructed_hash = reconstructed.hash(message);

        assert_eq!(original_hash.as_bytes(), reconstructed_hash.as_bytes());
        assert_eq!(original_hash.as_u64(), reconstructed_hash.as_u64());
        assert_eq!(original_hash.as_u128(), reconstructed_hash.as_u128());
        assert_hash128_views_are_consistent(original_hash);
    }
}

#[test]
fn sip128_hasher_keys_and_key_round_trip_from_byte_key() {
    let key_bytes = [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
        0x23, 0x01,
    ];
    let expected_keys = (0xfedc_ba98_7654_3210_u64, 0x0123_4567_89ab_cdef_u64);

    let hasher_from_bytes = SipHasher::new_with_key(&key_bytes);
    let hasher_from_words = SipHasher::new_with_keys(expected_keys.0, expected_keys.1);

    assert_eq!(hasher_from_bytes.key(), key_bytes);
    assert_eq!(hasher_from_bytes.keys(), expected_keys);
    assert_eq!(hasher_from_words.key(), key_bytes);
    assert_eq!(hasher_from_words.keys(), expected_keys);

    let base_message = b"message";
    let extended_message = b"message with suffix";

    let base_hash = hasher_from_bytes.hash(base_message);
    let same_base_hash = hasher_from_words.hash(base_message);
    let extended_hash = hasher_from_bytes.hash(extended_message);

    assert_eq!(base_hash.as_bytes(), same_base_hash.as_bytes());
    assert_eq!(base_hash.as_u128(), same_base_hash.as_u128());
    assert_ne!(base_hash.as_u128(), extended_hash.as_u128());

    assert_hash128_views_are_consistent(base_hash);
    assert_hash128_views_are_consistent(extended_hash);
}

#[test]
fn sip128_default_hasher_exposes_zero_key_and_can_be_recreated() {
    let default_hasher = SipHasher::new();

    assert_eq!(default_hasher.keys(), (0_u64, 0_u64));
    assert_eq!(default_hasher.key(), [0_u8; 16]);

    let from_default_keys =
        SipHasher::new_with_keys(default_hasher.keys().0, default_hasher.keys().1);
    let from_default_key_bytes = SipHasher::new_with_key(&default_hasher.key());

    assert_eq!(from_default_keys.keys(), default_hasher.keys());
    assert_eq!(from_default_keys.key(), default_hasher.key());
    assert_eq!(from_default_key_bytes.keys(), default_hasher.keys());
    assert_eq!(from_default_key_bytes.key(), default_hasher.key());

    let empty_hash = default_hasher.hash(b"");
    let empty_hash_from_keys = from_default_keys.hash(b"");
    let empty_hash_from_bytes = from_default_key_bytes.hash(b"");

    assert_eq!(empty_hash.as_u128(), empty_hash_from_keys.as_u128());
    assert_eq!(empty_hash.as_u128(), empty_hash_from_bytes.as_u128());
    assert_hash128_views_are_consistent(empty_hash);
}