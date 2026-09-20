use siphasher::sip::SipHasher24;

#[test]
fn sip_hasher24_keys_are_exposed_and_key_bytes_are_little_endian() {
    let key0 = 0x0706_0504_0302_0100u64;
    let key1 = 0x0f0e_0d0c_0b0a_0908u64;
    let hasher = SipHasher24::new_with_keys(key0, key1);

    assert_eq!(hasher.keys(), (key0, key1));
    assert_eq!(
        hasher.key(),
        [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
            0x0d, 0x0e, 0x0f,
        ]
    );

    let reconstructed = SipHasher24::new_with_key(&hasher.key());

    assert_eq!(reconstructed.keys(), hasher.keys());
    assert_eq!(reconstructed.key(), hasher.key());

    let messages: [&[u8]; 5] = [
        b"",
        b"a",
        b"hello, siphash-2-4",
        b"the same key should produce the same hash for this message",
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 255],
    ];

    for message in messages {
        assert_eq!(
            reconstructed.hash(message),
            hasher.hash(message),
            "hashes should match after reconstructing from key bytes for message {:?}",
            message
        );
    }
}

#[test]
fn sip_hasher24_byte_key_round_trips_to_u64_keys_and_back() {
    let key_bytes = [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
        0x23, 0x01,
    ];

    let hasher = SipHasher24::new_with_key(&key_bytes);

    assert_eq!(hasher.key(), key_bytes);
    assert_eq!(
        hasher.keys(),
        (0xfedc_ba98_7654_3210u64, 0x0123_4567_89ab_cdefu64)
    );

    let from_u64_keys = SipHasher24::new_with_keys(hasher.keys().0, hasher.keys().1);

    assert_eq!(from_u64_keys.key(), key_bytes);
    assert_eq!(from_u64_keys.keys(), hasher.keys());

    let payload = b"constructing with bytes or u64 keys must be equivalent";
    assert_eq!(from_u64_keys.hash(payload), hasher.hash(payload));

    let changed_key = [
        0x11, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
        0x23, 0x01,
    ];
    let changed_hasher = SipHasher24::new_with_key(&changed_key);

    assert_ne!(changed_hasher.key(), hasher.key());
    assert_ne!(changed_hasher.keys(), hasher.keys());
}

#[test]
fn sip_hasher24_default_constructor_uses_zero_keys_and_is_reconstructible() {
    let default_hasher = SipHasher24::new();

    assert_eq!(default_hasher.keys(), (0, 0));
    assert_eq!(default_hasher.key(), [0u8; 16]);

    let from_keys = SipHasher24::new_with_keys(0, 0);
    let from_key_bytes = SipHasher24::new_with_key(&[0u8; 16]);

    assert_eq!(from_keys.keys(), default_hasher.keys());
    assert_eq!(from_key_bytes.key(), default_hasher.key());

    let inputs: [&[u8]; 4] = [
        b"",
        b"default keys",
        b"same zero key across constructors",
        &[42, 42, 42, 0, 1, 2, 3],
    ];

    for input in inputs {
        let expected = default_hasher.hash(input);
        assert_eq!(from_keys.hash(input), expected);
        assert_eq!(from_key_bytes.hash(input), expected);
    }
}