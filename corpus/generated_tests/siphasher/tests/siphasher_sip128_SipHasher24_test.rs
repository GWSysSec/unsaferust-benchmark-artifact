use siphasher::sip128::SipHasher24;

fn key_words_from_bytes(key: &[u8; 16]) -> (u64, u64) {
    let mut first = [0_u8; 8];
    let mut second = [0_u8; 8];

    first.copy_from_slice(&key[..8]);
    second.copy_from_slice(&key[8..]);

    (u64::from_le_bytes(first), u64::from_le_bytes(second))
}

fn key_bytes_from_words(key0: u64, key1: u64) -> [u8; 16] {
    let mut key = [0_u8; 16];

    key[..8].copy_from_slice(&key0.to_le_bytes());
    key[8..].copy_from_slice(&key1.to_le_bytes());

    key
}

#[test]
fn sip128_siphasher24_keys_and_key_round_trip_between_byte_and_word_constructors() {
    let key = [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
        0x23, 0x01,
    ];
    let (key0, key1) = key_words_from_bytes(&key);

    let from_bytes = SipHasher24::new_with_key(&key);
    let from_words = SipHasher24::new_with_keys(key0, key1);

    assert_eq!(from_bytes.keys(), (key0, key1));
    assert_eq!(from_bytes.key(), key);
    assert_eq!(from_words.keys(), from_bytes.keys());
    assert_eq!(from_words.key(), from_bytes.key());
    assert_eq!(key_bytes_from_words(key0, key1), key);

    let messages: [&[u8]; 4] = [
        b"",
        b"a",
        b"siphash-128 keyed workflow",
        b"the quick brown fox jumps over the lazy dog",
    ];

    for message in messages {
        let hash_from_bytes = from_bytes.hash(message);
        let hash_from_words = from_words.hash(message);

        assert_eq!(hash_from_bytes.as_bytes(), hash_from_words.as_bytes());
        assert_eq!(hash_from_bytes.as_u64(), hash_from_words.as_u64());
        assert_eq!(hash_from_bytes.as_u128(), hash_from_words.as_u128());
    }
}

#[test]
fn sip128_siphasher24_key_returns_a_stable_copy_and_hashing_does_not_mutate_keys() {
    let key0 = 0x0123_4567_89ab_cdef_u64;
    let key1 = 0xfedc_ba98_7654_3210_u64;
    let hasher = SipHasher24::new_with_keys(key0, key1);
    let original_key = hasher.key();

    assert_eq!(hasher.keys(), (key0, key1));
    assert_eq!(original_key, key_bytes_from_words(key0, key1));

    let mut mutated_copy = original_key;
    mutated_copy[0] ^= 0xff;
    mutated_copy[15] ^= 0xff;

    assert_ne!(mutated_copy, original_key);
    assert_eq!(hasher.key(), original_key);
    assert_eq!(hasher.keys(), (key0, key1));

    let before_hashing = hasher.key();
    let first_hash = hasher.hash(b"first payload");
    let second_hash = hasher.hash(b"second payload with different length");

    assert_eq!(hasher.key(), before_hashing);
    assert_eq!(hasher.keys(), key_words_from_bytes(&before_hashing));
    assert_ne!(first_hash.as_u128(), second_hash.as_u128());

    let reconstructed = SipHasher24::new_with_key(&hasher.key());
    assert_eq!(reconstructed.keys(), hasher.keys());
    assert_eq!(reconstructed.key(), hasher.key());
    assert_eq!(
        reconstructed.hash(b"first payload").as_bytes(),
        first_hash.as_bytes()
    );
}

#[test]
fn sip128_siphasher24_default_constructor_matches_explicit_zero_keys() {
    let default_hasher = SipHasher24::new();
    let explicit_zero_hasher = SipHasher24::new_with_keys(0, 0);
    let zero_key_hasher = SipHasher24::new_with_key(&[0_u8; 16]);

    assert_eq!(default_hasher.keys(), (0, 0));
    assert_eq!(default_hasher.key(), [0_u8; 16]);
    assert_eq!(explicit_zero_hasher.keys(), default_hasher.keys());
    assert_eq!(zero_key_hasher.keys(), default_hasher.keys());
    assert_eq!(explicit_zero_hasher.key(), default_hasher.key());
    assert_eq!(zero_key_hasher.key(), default_hasher.key());

    let edge_case_messages: [&[u8]; 3] = [b"", &[0_u8], &[0_u8; 32]];

    for message in edge_case_messages {
        let default_hash = default_hasher.hash(message);
        let explicit_hash = explicit_zero_hasher.hash(message);
        let byte_key_hash = zero_key_hasher.hash(message);

        assert_eq!(default_hash.as_u64(), explicit_hash.as_u64());
        assert_eq!(default_hash.as_bytes(), byte_key_hash.as_bytes());
        assert_eq!(default_hash.as_u128(), byte_key_hash.as_u128());
    }
}