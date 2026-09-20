use siphasher::sip::{SipHasher, SipHasher13, SipHasher24};

fn key_words_from_bytes(key: &[u8; 16]) -> (u64, u64) {
    let mut first = [0_u8; 8];
    let mut second = [0_u8; 8];

    first.copy_from_slice(&key[..8]);
    second.copy_from_slice(&key[8..]);

    (u64::from_le_bytes(first), u64::from_le_bytes(second))
}

#[test]
fn siphasher24_keys_and_key_round_trip_between_constructors() {
    let key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
        0x0e, 0x0f,
    ];
    let (key0, key1) = key_words_from_bytes(&key);

    let from_bytes = SipHasher24::new_with_key(&key);
    let from_words = SipHasher24::new_with_keys(key0, key1);

    assert_eq!(from_bytes.keys(), (key0, key1));
    assert_eq!(from_bytes.key(), key);
    assert_eq!(from_words.keys(), from_bytes.keys());
    assert_eq!(from_words.key(), from_bytes.key());

    let messages: [&[u8]; 4] = [
        b"",
        b"a",
        b"the quick brown fox jumps over the lazy dog",
        b"the quick brown fox jumps over the lazy cog",
    ];

    for message in messages {
        assert_eq!(from_bytes.hash(message), from_words.hash(message));
    }

    assert_eq!(from_bytes.hash(b""), 0x726f_db47_dd0e_0e31);
    assert_ne!(
        from_bytes.hash(b"the quick brown fox jumps over the lazy dog"),
        from_bytes.hash(b"the quick brown fox jumps over the lazy cog")
    );
}

#[test]
fn siphasher_alias_uses_same_key_representation_as_siphasher24() {
    let key0 = 0x0706_0504_0302_0100_u64;
    let key1 = 0x0f0e_0d0c_0b0a_0908_u64;
    let expected_key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
        0x0e, 0x0f,
    ];

    let alias_hasher = SipHasher::new_with_keys(key0, key1);
    let sip24_hasher = SipHasher24::new_with_key(&expected_key);

    assert_eq!(alias_hasher.keys(), (key0, key1));
    assert_eq!(alias_hasher.key(), expected_key);
    assert_eq!(key_words_from_bytes(&alias_hasher.key()), alias_hasher.keys());
    assert_eq!(alias_hasher.keys(), sip24_hasher.keys());
    assert_eq!(alias_hasher.key(), sip24_hasher.key());

    let payload = b"same key material should produce same SipHash-2-4 result";
    assert_eq!(alias_hasher.hash(payload), sip24_hasher.hash(payload));
}

#[test]
fn siphasher13_preserves_key_material_and_is_deterministic() {
    let key = [
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22,
        0x11, 0x00,
    ];
    let (key0, key1) = key_words_from_bytes(&key);

    let from_bytes = SipHasher13::new_with_key(&key);
    let reconstructed = SipHasher13::new_with_keys(from_bytes.keys().0, from_bytes.keys().1);

    assert_eq!(from_bytes.keys(), (key0, key1));
    assert_eq!(from_bytes.key(), key);
    assert_eq!(reconstructed.keys(), from_bytes.keys());
    assert_eq!(reconstructed.key(), from_bytes.key());

    let message = b"deterministic hashing with SipHash-1-3";
    let first = from_bytes.hash(message);
    let second = from_bytes.hash(message);
    let reconstructed_hash = reconstructed.hash(message);

    assert_eq!(first, second);
    assert_eq!(first, reconstructed_hash);
    assert_ne!(first, from_bytes.hash(b"deterministic hashing with SipHash-1-3."));
}

#[test]
fn default_sip_hashers_expose_zero_key_material() {
    let zero_key = [0_u8; 16];

    let default_alias = SipHasher::new();
    let default_13 = SipHasher13::new();
    let default_24 = SipHasher24::new();

    assert_eq!(default_alias.keys(), (0, 0));
    assert_eq!(default_alias.key(), zero_key);
    assert_eq!(default_13.keys(), (0, 0));
    assert_eq!(default_13.key(), zero_key);
    assert_eq!(default_24.keys(), (0, 0));
    assert_eq!(default_24.key(), zero_key);

    let message = b"default key material";
    assert_eq!(
        default_alias.hash(message),
        SipHasher::new_with_key(&zero_key).hash(message)
    );
    assert_eq!(
        default_13.hash(message),
        SipHasher13::new_with_keys(0, 0).hash(message)
    );
    assert_eq!(
        default_24.hash(message),
        SipHasher24::new_with_key(&zero_key).hash(message)
    );
}