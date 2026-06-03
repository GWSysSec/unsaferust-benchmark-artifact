use siphasher::sip128::{SipHasher};
use std::hash::Hasher;

#[test]
fn test_sip128_keys_default_zero() {
    let hasher = SipHasher::new();
    let (k0, k1) = hasher.keys();
    assert_eq!(k0, 0u64);
    assert_eq!(k1, 0u64);

    let key_bytes = hasher.key();
    assert_eq!(key_bytes.len(), 16);
    assert_eq!(key_bytes, [0u8; 16]);


    let mut lo = [0u8; 8];
    let mut hi = [0u8; 8];
    lo.copy_from_slice(&key_bytes[0..8]);
    hi.copy_from_slice(&key_bytes[8..16]);
    assert_eq!(u64::from_le_bytes(lo), k0);
    assert_eq!(u64::from_le_bytes(hi), k1);
    assert_eq!(u64::from_le_bytes(lo), 0u64);
    assert_eq!(u64::from_le_bytes(hi), 0u64);
}

#[test]
fn test_sip128_keys_invariant_over_writes() {
    let mut hasher = SipHasher::new();
    let (k0_init, k1_init) = hasher.keys();
    let key_init = hasher.key();


    assert_eq!(k0_init, 0u64);
    assert_eq!(k1_init, 0u64);
    assert_eq!(key_init, [0u8; 16]);


    hasher.write(b"first chunk of data");
    let (k0_mid, k1_mid) = hasher.keys();
    let key_mid = hasher.key();
    assert_eq!(k0_init, k0_mid);
    assert_eq!(k1_init, k1_mid);
    assert_eq!(key_init, key_mid);


    hasher.write_u64(0xDEAD_BEEF_CAFE_BABE);
    hasher.write_u32(0x1234_5678);
    hasher.write_u8(0xAB);

    let (k0_end, k1_end) = hasher.keys();
    let key_end = hasher.key();
    assert_eq!(k0_init, k0_end);
    assert_eq!(k1_init, k1_end);
    assert_eq!(key_init, key_end);


    let digest = hasher.finish();
    assert_ne!(digest, 0u64);
}

#[test]
fn test_sip128_keys_multi_instance_identical() {
    let h1 = SipHasher::new();
    let h2 = SipHasher::new();
    let h3 = SipHasher::new();

    let keys1 = h1.keys();
    let keys2 = h2.keys();
    let keys3 = h3.keys();


    assert_eq!(keys1, keys2);
    assert_eq!(keys2, keys3);
    assert_eq!(keys1.0, 0u64);
    assert_eq!(keys1.1, 0u64);

    let b1 = h1.key();
    let b2 = h2.key();
    let b3 = h3.key();

    assert_eq!(b1, b2);
    assert_eq!(b2, b3);
    assert_eq!(b1, [0u8; 16]);

    assert_eq!(b1.len(), 16);
}

#[test]
fn test_sip128_key_bytes_and_keys_consistent() {
    let hasher = SipHasher::new();
    let key_bytes = hasher.key();
    let (k0, k1) = hasher.keys();


    let mut lo = [0u8; 8];
    let mut hi = [0u8; 8];
    lo.copy_from_slice(&key_bytes[..8]);
    hi.copy_from_slice(&key_bytes[8..]);
    let reassembled_k0 = u64::from_le_bytes(lo);
    let reassembled_k1 = u64::from_le_bytes(hi);

    assert_eq!(reassembled_k0, k0);
    assert_eq!(reassembled_k1, k1);
    assert_eq!(key_bytes.len(), 16);


    assert_eq!(k0, 0u64);
    assert_eq!(k1, 0u64);
    assert_eq!(reassembled_k0, 0u64);
    assert_eq!(reassembled_k1, 0u64);


    let non_zero_count = key_bytes.iter().filter(|b| **b != 0).count();
    assert_eq!(non_zero_count, 0usize);
}

#[test]
fn test_sip128_full_workflow_determinism() {
    let mut hasher_a = SipHasher::new();


    let initial_keys = hasher_a.keys();
    let initial_key_bytes = hasher_a.key();
    assert_eq!(initial_keys, (0u64, 0u64));
    assert_eq!(initial_key_bytes, [0u8; 16]);


    let payload: &[u8] = b"The quick brown fox jumps over the lazy dog";
    hasher_a.write(payload);
    hasher_a.write_u64(42);
    hasher_a.write_i32(-7);


    let keys_after = hasher_a.keys();
    let key_bytes_after = hasher_a.key();
    assert_eq!(keys_after, initial_keys);
    assert_eq!(key_bytes_after, initial_key_bytes);

    let digest_a = hasher_a.finish();


    let mut hasher_b = SipHasher::new();
    assert_eq!(hasher_b.keys(), initial_keys);
    assert_eq!(hasher_b.key(), initial_key_bytes);

    hasher_b.write(payload);
    hasher_b.write_u64(42);
    hasher_b.write_i32(-7);
    let digest_b = hasher_b.finish();

    assert_eq!(digest_a, digest_b);

    assert_eq!(hasher_b.keys(), (0u64, 0u64));
    assert_eq!(hasher_b.key(), [0u8; 16]);
}

#[test]
fn test_sip128_keys_key_roundtrip_large_input() {
    let mut hasher = SipHasher::new();


    let (k0_before, k1_before) = hasher.keys();
    let key_before = hasher.key();
    assert_eq!(k0_before, 0u64);
    assert_eq!(k1_before, 0u64);
    assert_eq!(key_before, [0u8; 16]);


    let buf = vec![0xA5u8; 1024 * 1024];
    hasher.write(&buf);


    let (k0_after, k1_after) = hasher.keys();
    let key_after = hasher.key();
    assert_eq!(k0_after, k0_before);
    assert_eq!(k1_after, k1_before);
    assert_eq!(key_after, key_before);


    let digest1 = hasher.finish();

    let mut hasher2 = SipHasher::new();
    hasher2.write(&buf);
    let digest2 = hasher2.finish();
    assert_eq!(digest1, digest2);


    let empty_hasher = SipHasher::new();
    let empty_digest = empty_hasher.finish();
    assert_ne!(digest1, empty_digest);

    assert_eq!(empty_hasher.keys(), (0u64, 0u64));
}