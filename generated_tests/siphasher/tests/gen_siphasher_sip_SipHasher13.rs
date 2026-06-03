use siphasher::sip::SipHasher13;
use std::hash::Hasher;

#[test]
fn test_siphasher13_default_keys_are_zero() {
    let hasher = SipHasher13::new();
    let (k0, k1) = hasher.keys();


    assert_eq!(k0, 0u64);
    assert_eq!(k1, 0u64);


    let key_bytes: [u8; 16] = hasher.key();
    assert_eq!(key_bytes.len(), 16);
    assert_eq!(key_bytes, [0u8; 16]);
    assert_eq!(&key_bytes[..8], &[0u8; 8]);
    assert_eq!(&key_bytes[8..], &[0u8; 8]);


    let mut expected = [0u8; 16];
    expected[0..8].copy_from_slice(&k0.to_le_bytes());
    expected[8..16].copy_from_slice(&k1.to_le_bytes());
    assert_eq!(key_bytes, expected);


    let hasher2 = SipHasher13::new();
    let (k0b, k1b) = hasher2.keys();
    assert_eq!(k0, k0b);
    assert_eq!(k1, k1b);
    assert_eq!(key_bytes, hasher2.key());
}

#[test]
fn test_siphasher13_keys_stable_across_writes() {
    let mut hasher = SipHasher13::new();
    let (k0_before, k1_before) = hasher.keys();
    let key_before = hasher.key();

    assert_eq!(k0_before, 0u64);
    assert_eq!(k1_before, 0u64);
    assert_eq!(key_before, [0u8; 16]);


    hasher.write(b"hello");
    let (k0_mid, k1_mid) = hasher.keys();
    assert_eq!(k0_mid, k0_before);
    assert_eq!(k1_mid, k1_before);
    assert_eq!(hasher.key(), key_before);


    hasher.write_u64(0xDEAD_BEEF_DEAD_BEEFu64);
    hasher.write_u8(42);
    hasher.write(b", world!");

    let (k0_after, k1_after) = hasher.keys();
    assert_eq!(k0_after, k0_before);
    assert_eq!(k1_after, k1_before);
    assert_eq!(hasher.key(), key_before);


    let _digest = hasher.finish();
    let (k0_fin, k1_fin) = hasher.keys();
    assert_eq!(k0_fin, k0_before);
    assert_eq!(k1_fin, k1_before);
    assert_eq!(hasher.key(), key_before);
}

#[test]
fn test_siphasher13_key_byte_layout_multistep() {
    let p0: &[u8] = b"";
    let p1: &[u8] = b"a";
    let p2: &[u8] = b"The quick brown fox jumps over the lazy dog";
    let p3_buf: Vec<u8> = vec![0xFFu8; 256];
    let p3: &[u8] = &p3_buf;
    let payloads: [&[u8]; 4] = [p0, p1, p2, p3];

    let mut digests: Vec<u64> = Vec::new();
    for payload in payloads.iter() {
        let mut hasher = SipHasher13::new();


        let (k0, k1) = hasher.keys();
        let kb = hasher.key();
        assert_eq!(k0, 0u64);
        assert_eq!(k1, 0u64);
        assert_eq!(kb, [0u8; 16]);


        let mut derived = [0u8; 16];
        derived[0..8].copy_from_slice(&k0.to_le_bytes());
        derived[8..16].copy_from_slice(&k1.to_le_bytes());
        assert_eq!(derived, kb);

        hasher.write(payload);


        let (k0b, k1b) = hasher.keys();
        assert_eq!(k0b, k0);
        assert_eq!(k1b, k1);
        assert_eq!(hasher.key(), kb);

        digests.push(hasher.finish());
    }


    assert_eq!(digests.len(), 4);
    assert_ne!(digests[0], digests[1]);
    assert_ne!(digests[1], digests[2]);
    assert_ne!(digests[2], digests[3]);
    assert_ne!(digests[0], digests[3]);
    assert_ne!(digests[0], digests[2]);


    let mut h1 = SipHasher13::new();
    h1.write(b"");
    let d1 = h1.finish();
    let mut h2 = SipHasher13::new();
    h2.write(b"");
    let d2 = h2.finish();
    assert_eq!(d1, d2);
    assert_eq!(d1, digests[0]);
}

#[test]
fn test_siphasher13_key_accessors_consistency_after_reuse() {


    let mut hasher = SipHasher13::new();

    let initial_keys = hasher.keys();
    let initial_key_bytes = hasher.key();

    assert_eq!(initial_keys.0, 0u64);
    assert_eq!(initial_keys.1, 0u64);
    assert_eq!(initial_key_bytes, [0u8; 16]);

    let messages: [&[u8]; 3] = [b"alpha", b"beta-beta", b"gamma_gamma_gamma"];
    let mut observed_digests: Vec<u64> = Vec::with_capacity(messages.len());

    for msg in messages.iter() {
        hasher.write(msg);
        let d = hasher.finish();
        observed_digests.push(d);


        let (kc0, kc1) = hasher.keys();
        assert_eq!(kc0, initial_keys.0);
        assert_eq!(kc1, initial_keys.1);
        assert_eq!(hasher.key(), initial_key_bytes);
    }

    assert_eq!(observed_digests.len(), 3);


    let (fk0, fk1) = hasher.keys();
    let mut rebuilt = [0u8; 16];
    rebuilt[0..8].copy_from_slice(&fk0.to_le_bytes());
    rebuilt[8..16].copy_from_slice(&fk1.to_le_bytes());
    assert_eq!(rebuilt, hasher.key());
    assert_eq!(rebuilt, initial_key_bytes);
    assert_eq!(fk0, 0u64);
    assert_eq!(fk1, 0u64);
}