use siphasher::sip::SipHasher13;

#[test]
fn sip_hasher13_round_trips_keys_to_canonical_little_endian_key_bytes() {
    let key0 = 0x0706_0504_0302_0100u64;
    let key1 = 0x0f0e_0d0c_0b0a_0908u64;

    let hasher = SipHasher13::new_with_keys(key0, key1);

    assert_eq!(hasher.keys(), (key0, key1));
    assert_eq!(
        hasher.key(),
        [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
            0x0d, 0x0e, 0x0f,
        ]
    );

    let reconstructed = SipHasher13::new_with_key(&hasher.key());

    assert_eq!(reconstructed.keys(), hasher.keys());
    assert_eq!(reconstructed.key(), hasher.key());

    let messages: [&[u8]; 4] = [
        b"",
        b"a",
        b"the same SipHasher13 key should produce the same output",
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 255],
    ];

    for message in messages {
        assert_eq!(reconstructed.hash(message), hasher.hash(message));
    }
}

#[test]
fn sip_hasher13_round_trips_byte_key_to_u64_keys() {
    let key_bytes = [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
        0x23, 0x01,
    ];

    let hasher = SipHasher13::new_with_key(&key_bytes);

    assert_eq!(hasher.key(), key_bytes);
    assert_eq!(
        hasher.keys(),
        (0xfedc_ba98_7654_3210u64, 0x0123_4567_89ab_cdefu64)
    );

    let reconstructed = SipHasher13::new_with_keys(hasher.keys().0, hasher.keys().1);

    assert_eq!(reconstructed.key(), key_bytes);
    assert_eq!(reconstructed.keys(), hasher.keys());

    let payload = b"byte-key construction and u64-key construction must agree";
    assert_eq!(reconstructed.hash(payload), hasher.hash(payload));
}

#[test]
fn sip_hasher13_default_constructor_uses_zero_key_material() {
    let default_hasher = SipHasher13::new();
    let explicit_zero_hasher = SipHasher13::new_with_keys(0, 0);
    let zero_byte_hasher = SipHasher13::new_with_key(&[0u8; 16]);

    assert_eq!(default_hasher.keys(), (0, 0));
    assert_eq!(default_hasher.key(), [0u8; 16]);
    assert_eq!(explicit_zero_hasher.keys(), default_hasher.keys());
    assert_eq!(zero_byte_hasher.key(), default_hasher.key());

    let empty = b"";
    let non_empty = b"default key material should be deterministic";

    assert_eq!(default_hasher.hash(empty), explicit_zero_hasher.hash(empty));
    assert_eq!(default_hasher.hash(non_empty), zero_byte_hasher.hash(non_empty));
}

#[test]
fn sip_hasher13_different_keys_are_reported_and_affect_hashes() {
    let first = SipHasher13::new_with_keys(
        0x1111_2222_3333_4444u64,
        0x5555_6666_7777_8888u64,
    );
    let second = SipHasher13::new_with_keys(
        0x9999_aaaa_bbbb_ccccu64,
        0xdddd_eeee_ffff_0000u64,
    );

    assert_ne!(first.keys(), second.keys());
    assert_ne!(first.key(), second.key());

    assert_eq!(
        first.key(),
        [
            0x44, 0x44, 0x33, 0x33, 0x22, 0x22, 0x11, 0x11, 0x88, 0x88, 0x77, 0x77, 0x66,
            0x66, 0x55, 0x55,
        ]
    );
    assert_eq!(
        second.key(),
        [
            0xcc, 0xcc, 0xbb, 0xbb, 0xaa, 0xaa, 0x99, 0x99, 0x00, 0x00, 0xff, 0xff, 0xee,
            0xee, 0xdd, 0xdd,
        ]
    );

    let message = b"same input with different keys should normally have different SipHash tags";
    assert_ne!(first.hash(message), second.hash(message));
}