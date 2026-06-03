use siphasher::sip128::SipHasher13;

fn key_words_from_bytes(key: &[u8; 16]) -> (u64, u64) {
    let mut first = [0_u8; 8];
    let mut second = [0_u8; 8];
    first.copy_from_slice(&key[..8]);
    second.copy_from_slice(&key[8..]);
    (u64::from_le_bytes(first), u64::from_le_bytes(second))
}

fn assert_hash128_views_are_consistent(hash: siphasher::sip128::Hash128) {
    let bytes = hash.as_bytes();
    let (low, high) = hash.as_u64();
    let combined = hash.as_u128();

    let mut low_bytes = [0_u8; 8];
    let mut high_bytes = [0_u8; 8];
    low_bytes.copy_from_slice(&bytes[..8]);
    high_bytes.copy_from_slice(&bytes[8..]);

    assert_eq!(low, u64::from_le_bytes(low_bytes));
    assert_eq!(high, u64::from_le_bytes(high_bytes));
    assert_eq!(combined as u64, low);
    assert_eq!((combined >> 64) as u64, high);
    assert_eq!(combined, (low as u128) | ((high as u128) << 64));
}

#[test]
fn sip128_siphasher13_keys_and_key_round_trip_from_word_keys() {
    let key0 = 0x0706_0504_0302_0100_u64;
    let key1 = 0x0f0e_0d0c_0b0a_0908_u64;
    let expected_key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
        0x0e, 0x0f,
    ];

    let hasher = SipHasher13::new_with_keys(key0, key1);

    assert_eq!(hasher.keys(), (key0, key1));
    assert_eq!(hasher.key(), expected_key);
    assert_eq!(key_words_from_bytes(&hasher.key()), hasher.keys());

    let reconstructed = SipHasher13::new_with_key(&hasher.key());

    assert_eq!(reconstructed.keys(), hasher.keys());
    assert_eq!(reconstructed.key(), hasher.key());

    let messages: [&[u8]; 5] = [
        b"",
        b"a",
        b"hello from an integration test",
        b"siphash-1-3 with 128-bit output should be deterministic",
        &[0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144_u8, 233_u8],
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
fn sip128_siphasher13_keys_and_key_round_trip_from_byte_key() {
    let key = [
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22,
        0x11, 0x00,
    ];
    let expected_words = key_words_from_bytes(&key);

    let from_bytes = SipHasher13::new_with_key(&key);
    let from_words = SipHasher13::new_with_keys(expected_words.0, expected_words.1);

    assert_eq!(from_bytes.key(), key);
    assert_eq!(from_bytes.keys(), expected_words);
    assert_eq!(from_words.key(), key);
    assert_eq!(from_words.keys(), expected_words);
    assert_eq!(from_words.key(), from_bytes.key());
    assert_eq!(from_words.keys(), from_bytes.keys());

    let empty_hash_from_bytes = from_bytes.hash(b"");
    let empty_hash_from_words = from_words.hash(b"");
    let payload_hash_from_bytes = from_bytes.hash(b"payload with a non-empty message");
    let payload_hash_from_words = from_words.hash(b"payload with a non-empty message");

    assert_eq!(empty_hash_from_bytes.as_bytes(), empty_hash_from_words.as_bytes());
    assert_eq!(payload_hash_from_bytes.as_bytes(), payload_hash_from_words.as_bytes());
    assert_ne!(
        empty_hash_from_bytes.as_bytes(),
        payload_hash_from_bytes.as_bytes(),
        "different messages should not collide for this fixed key in this regression check"
    );

    assert_hash128_views_are_consistent(empty_hash_from_bytes);
    assert_hash128_views_are_consistent(payload_hash_from_bytes);
}

#[test]
fn sip128_siphasher13_default_key_is_zero_and_reconstructible() {
    let default_hasher = SipHasher13::new();
    let zero_key = [0_u8; 16];

    assert_eq!(default_hasher.keys(), (0, 0));
    assert_eq!(default_hasher.key(), zero_key);
    assert_eq!(key_words_from_bytes(&default_hasher.key()), (0, 0));

    let reconstructed_from_key = SipHasher13::new_with_key(&default_hasher.key());
    let reconstructed_from_words = SipHasher13::new_with_keys(0, 0);

    assert_eq!(reconstructed_from_key.keys(), default_hasher.keys());
    assert_eq!(reconstructed_from_key.key(), default_hasher.key());
    assert_eq!(reconstructed_from_words.keys(), default_hasher.keys());
    assert_eq!(reconstructed_from_words.key(), default_hasher.key());

    let message = b"default-key SipHasher13 regression message";
    let default_hash = default_hasher.hash(message);

    assert_eq!(
        default_hash.as_bytes(),
        reconstructed_from_key.hash(message).as_bytes()
    );
    assert_eq!(
        default_hash.as_u64(),
        reconstructed_from_words.hash(message).as_u64()
    );

    assert_hash128_views_are_consistent(default_hash);
}