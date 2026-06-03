use siphasher::sip::SipHasher;

#[test]
fn sip_hasher_keys_and_key_round_trip_from_u64_keys() {
    let key0 = 0x0706_0504_0302_0100u64;
    let key1 = 0x0f0e_0d0c_0b0a_0908u64;

    let hasher = SipHasher::new_with_keys(key0, key1);

    assert_eq!(hasher.keys(), (key0, key1));
    assert_eq!(
        hasher.key(),
        [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
            0x0d, 0x0e, 0x0f
        ]
    );

    let reconstructed = SipHasher::new_with_key(&hasher.key());
    assert_eq!(reconstructed.keys(), hasher.keys());
    assert_eq!(reconstructed.key(), hasher.key());

    let message = b"the same keys should produce the same tag";
    assert_eq!(reconstructed.hash(message), hasher.hash(message));
}

#[test]
fn sip_hasher_keys_and_key_round_trip_from_byte_key() {
    let key_bytes = [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
        0x23, 0x01,
    ];

    let hasher = SipHasher::new_with_key(&key_bytes);

    assert_eq!(hasher.key(), key_bytes);
    assert_eq!(
        hasher.keys(),
        (0xfedc_ba98_7654_3210u64, 0x0123_4567_89ab_cdefu64)
    );

    let from_extracted_keys = {
        let (key0, key1) = hasher.keys();
        SipHasher::new_with_keys(key0, key1)
    };

    assert_eq!(from_extracted_keys.key(), key_bytes);
    assert_eq!(from_extracted_keys.keys(), hasher.keys());

    let payloads: [&[u8]; 4] = [
        b"",
        b"a",
        b"short payload",
        b"a longer payload that crosses multiple eight-byte blocks",
    ];

    for payload in payloads {
        assert_eq!(
            from_extracted_keys.hash(payload),
            hasher.hash(payload),
            "hash mismatch for payload {:?}",
            payload
        );
    }
}

#[test]
fn sip_hasher_default_key_is_stable_and_equivalent_to_explicit_default_key() {
    let default_hasher = SipHasher::new();

    assert_eq!(default_hasher.keys(), (0, 0));
    assert_eq!(default_hasher.key(), [0u8; 16]);

    let explicit_from_keys = SipHasher::new_with_keys(0, 0);
    let explicit_from_key = SipHasher::new_with_key(&[0u8; 16]);

    assert_eq!(explicit_from_keys.keys(), default_hasher.keys());
    assert_eq!(explicit_from_key.key(), default_hasher.key());

    let edge_case_messages: [&[u8]; 5] = [
        b"",
        &[0],
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0, 0, 0, 0],
        &[255; 31],
    ];

    for message in edge_case_messages {
        let default_hash = default_hasher.hash(message);
        assert_eq!(explicit_from_keys.hash(message), default_hash);
        assert_eq!(explicit_from_key.hash(message), default_hash);
    }
}

#[test]
fn sip_hasher_different_keys_preserve_key_material_and_change_hashes() {
    let first = SipHasher::new_with_keys(0x1111_2222_3333_4444, 0x5555_6666_7777_8888);
    let second = SipHasher::new_with_keys(0x9999_aaaa_bbbb_cccc, 0xdddd_eeee_ffff_0000);

    assert_ne!(first.keys(), second.keys());
    assert_ne!(first.key(), second.key());

    let message = b"key separation matters for siphash";
    let first_hash = first.hash(message);
    let second_hash = second.hash(message);

    assert_ne!(first_hash, second_hash);

    let first_recreated = SipHasher::new_with_key(&first.key());
    let second_recreated = SipHasher::new_with_key(&second.key());

    assert_eq!(first_recreated.keys(), first.keys());
    assert_eq!(second_recreated.keys(), second.keys());
    assert_eq!(first_recreated.hash(message), first_hash);
    assert_eq!(second_recreated.hash(message), second_hash);
}