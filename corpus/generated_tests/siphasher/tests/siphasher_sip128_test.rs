use siphasher::sip128::{Hash128, SipHasher, SipHasher13, SipHasher24};

fn words_from_le_key_bytes(key: &[u8; 16]) -> (u64, u64) {
    let mut first = [0_u8; 8];
    let mut second = [0_u8; 8];
    first.copy_from_slice(&key[..8]);
    second.copy_from_slice(&key[8..]);
    (u64::from_le_bytes(first), u64::from_le_bytes(second))
}

fn assert_hash_views_match(hash: Hash128) {
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
fn sip128_default_hasher_key_accessors_and_hash_views_are_stable() {
    let hasher = SipHasher::new();

    assert_eq!(hasher.keys(), (0, 0));
    assert_eq!(hasher.key(), [0_u8; 16]);
    assert_eq!(words_from_le_key_bytes(&hasher.key()), hasher.keys());

    let empty_hash = hasher.hash(b"");
    let repeated_empty_hash = hasher.hash(b"");
    assert_eq!(empty_hash.as_u64(), repeated_empty_hash.as_u64());
    assert_eq!(empty_hash.as_u128(), repeated_empty_hash.as_u128());
    assert_hash_views_match(empty_hash);

    let message_hash = hasher.hash(b"same key, different message");
    assert_hash_views_match(message_hash);
    assert_ne!(message_hash.as_u128(), repeated_empty_hash.as_u128());
}

#[test]
fn sip128_key_bytes_round_trip_through_keys_and_hashes() {
    let key = [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
        0x23, 0x01,
    ];
    let (key0, key1) = words_from_le_key_bytes(&key);

    let from_key_bytes = SipHasher24::new_with_key(&key);
    let from_key_words = SipHasher24::new_with_keys(key0, key1);

    assert_eq!(from_key_bytes.keys(), (key0, key1));
    assert_eq!(from_key_bytes.key(), key);
    assert_eq!(from_key_words.keys(), from_key_bytes.keys());
    assert_eq!(from_key_words.key(), from_key_bytes.key());

    let messages: [&[u8]; 4] = [
        b"",
        b"a",
        b"short message",
        b"a longer message that spans more than one siphash block",
    ];

    for message in messages {
        let hash_from_bytes = from_key_bytes.hash(message);
        let hash_from_words = from_key_words.hash(message);

        assert_eq!(hash_from_bytes.as_u64(), hash_from_words.as_u64());
        assert_eq!(hash_from_bytes.as_u128(), hash_from_words.as_u128());
        assert_eq!(hash_from_bytes.as_bytes(), hash_from_words.as_bytes());
        assert_hash_views_match(hash_from_bytes);
    }
}

#[test]
fn sip128_siphasher13_key_accessors_reconstruct_equivalent_hasher() {
    let key0 = 0x0706_0504_0302_0100_u64;
    let key1 = 0x0f0e_0d0c_0b0a_0908_u64;
    let expected_key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
        0x0e, 0x0f,
    ];

    let original = SipHasher13::new_with_keys(key0, key1);
    let reconstructed = SipHasher13::new_with_key(&original.key());

    assert_eq!(original.keys(), (key0, key1));
    assert_eq!(original.key(), expected_key);
    assert_eq!(words_from_le_key_bytes(&original.key()), original.keys());
    assert_eq!(reconstructed.keys(), original.keys());
    assert_eq!(reconstructed.key(), original.key());

    let message = b"reconstructing from key bytes preserves siphash-1-3 output";
    let original_hash = original.hash(message);
    let reconstructed_hash = reconstructed.hash(message);

    assert_eq!(original_hash.as_u64(), reconstructed_hash.as_u64());
    assert_eq!(original_hash.as_u128(), reconstructed_hash.as_u128());
    assert_hash_views_match(original_hash);
}

#[test]
fn sip128_variants_use_same_key_representation_but_distinct_rounds() {
    let key = [
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22,
        0x11, 0x00,
    ];
    let (key0, key1) = words_from_le_key_bytes(&key);

    let sip13 = SipHasher13::new_with_key(&key);
    let sip24 = SipHasher24::new_with_key(&key);
    let default_alias = SipHasher::new_with_key(&key);

    assert_eq!(sip13.keys(), (key0, key1));
    assert_eq!(sip24.keys(), (key0, key1));
    assert_eq!(default_alias.keys(), (key0, key1));
    assert_eq!(sip13.key(), key);
    assert_eq!(sip24.key(), key);
    assert_eq!(default_alias.key(), key);

    let message = b"the same key and message under different siphash rounds";
    let hash13 = sip13.hash(message);
    let hash24 = sip24.hash(message);
    let hash_alias = default_alias.hash(message);

    assert_hash_views_match(hash13);
    assert_hash_views_match(hash24);
    assert_hash_views_match(hash_alias);

    assert_ne!(hash13.as_u128(), hash24.as_u128());
    assert_eq!(hash_alias.as_u64(), hash24.as_u64());
    assert_eq!(hash_alias.as_u128(), hash24.as_u128());
}