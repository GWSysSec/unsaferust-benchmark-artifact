use siphasher::sip128::{SipHasher13, SipHasher24};

fn key_words_from_bytes(key: &[u8; 16]) -> (u64, u64) {
    let mut first = [0_u8; 8];
    let mut second = [0_u8; 8];
    first.copy_from_slice(&key[..8]);
    second.copy_from_slice(&key[8..]);
    (u64::from_le_bytes(first), u64::from_le_bytes(second))
}

fn assert_hash128_conversions_are_consistent(hash: siphasher::sip128::Hash128) {
    let (low, high) = hash.as_u64();
    let combined = hash.as_u128();

    assert_eq!(combined as u64, low);
    assert_eq!((combined >> 64) as u64, high);
    assert_eq!(combined, (low as u128) | ((high as u128) << 64));
}

#[test]
fn hash128_u128_and_u64_views_are_consistent_for_keyed_siphash24() {
    let key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    ];
    let (key0, key1) = key_words_from_bytes(&key);

    let hasher_from_bytes = SipHasher24::new_with_key(&key);
    let hasher_from_words = SipHasher24::new_with_keys(key0, key1);

    assert_eq!(hasher_from_bytes.keys(), (key0, key1));
    assert_eq!(hasher_from_bytes.key(), key);
    assert_eq!(hasher_from_words.keys(), hasher_from_bytes.keys());
    assert_eq!(hasher_from_words.key(), hasher_from_bytes.key());

    let message = b"the quick brown fox jumps over the lazy dog";
    let hash_from_bytes = hasher_from_bytes.hash(message);
    let hash_from_words = hasher_from_words.hash(message);

    assert_eq!(hash_from_bytes.as_u64(), hash_from_words.as_u64());
    assert_eq!(hash_from_bytes.as_u128(), hash_from_words.as_u128());
    assert_hash128_conversions_are_consistent(hash_from_bytes);
}

#[test]
fn hash128_conversions_are_stable_and_input_sensitive_for_siphash13() {
    let key = [
        0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87,
        0x78, 0x69, 0x5a, 0x4b, 0x3c, 0x2d, 0x1e, 0x0f,
    ];

    let hasher = SipHasher13::new_with_key(&key);

    let empty_hash = hasher.hash(b"");
    let short_hash = hasher.hash(b"message");
    let repeated_short_hash = hasher.hash(b"message");
    let longer_hash = hasher.hash(b"message with extra bytes");

    assert_eq!(short_hash.as_u64(), repeated_short_hash.as_u64());
    assert_eq!(short_hash.as_u128(), repeated_short_hash.as_u128());
    assert_ne!(empty_hash.as_u128(), short_hash.as_u128());
    assert_ne!(short_hash.as_u64(), longer_hash.as_u64());

    assert_hash128_conversions_are_consistent(empty_hash);
    assert_hash128_conversions_are_consistent(short_hash);
    assert_hash128_conversions_are_consistent(longer_hash);
}

#[test]
fn default_and_keyed_hashers_produce_deterministic_hash128_values() {
    let default_24 = SipHasher24::new();
    let explicit_zero_24 = SipHasher24::new_with_keys(0, 0);
    let default_13 = SipHasher13::new();
    let explicit_zero_13 = SipHasher13::new_with_keys(0, 0);

    assert_eq!(default_24.keys(), (0, 0));
    assert_eq!(default_13.keys(), (0, 0));
    assert_eq!(default_24.key(), [0_u8; 16]);
    assert_eq!(default_13.key(), [0_u8; 16]);

    let payload_parts: [&[u8]; 4] = [b"multi", b"-", b"step", b"-workflow"];
    let mut payload = Vec::new();
    for part in payload_parts {
        payload.extend_from_slice(part);
    }

    let default_24_hash = default_24.hash(&payload);
    let zero_24_hash = explicit_zero_24.hash(&payload);
    let default_13_hash = default_13.hash(&payload);
    let zero_13_hash = explicit_zero_13.hash(&payload);

    assert_eq!(default_24_hash.as_u128(), zero_24_hash.as_u128());
    assert_eq!(default_24_hash.as_u64(), zero_24_hash.as_u64());
    assert_eq!(default_13_hash.as_u128(), zero_13_hash.as_u128());
    assert_eq!(default_13_hash.as_u64(), zero_13_hash.as_u64());

    assert_hash128_conversions_are_consistent(default_24_hash);
    assert_hash128_conversions_are_consistent(default_13_hash);
}