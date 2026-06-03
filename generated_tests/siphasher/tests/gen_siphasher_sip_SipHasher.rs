use siphasher::sip::SipHasher;
use std::hash::Hasher;

#[test]
fn test_default_hasher_keys_are_zero() {
    let hasher = SipHasher::new();
    let (k0, k1) = hasher.keys();


    assert_eq!(k0, 0u64);
    assert_eq!(k1, 0u64);

    let key_bytes = hasher.key();
    assert_eq!(key_bytes.len(), 16);
    assert_eq!(key_bytes, [0u8; 16]);


    for (i, b) in key_bytes.iter().enumerate() {
        assert_eq!(*b, 0u8, "byte at index {} should be zero", i);
    }


    assert_eq!(hasher.keys(), (0u64, 0u64));
    assert_ne!(hasher.keys(), (1u64, 0u64));
}

#[test]
fn test_keys_and_key_byte_layout_consistency() {
    let hasher = SipHasher::new();
    let (k0, k1) = hasher.keys();
    let key_bytes = hasher.key();




    let mut first_half = [0u8; 8];
    let mut second_half = [0u8; 8];
    first_half.copy_from_slice(&key_bytes[0..8]);
    second_half.copy_from_slice(&key_bytes[8..16]);

    let reconstructed_k0 = u64::from_le_bytes(first_half);
    let reconstructed_k1 = u64::from_le_bytes(second_half);

    assert_eq!(reconstructed_k0, k0, "first 8 bytes must encode k0 (LE)");
    assert_eq!(reconstructed_k1, k1, "last 8 bytes must encode k1 (LE)");


    assert_eq!(key_bytes[0..8], first_half);
    assert_eq!(key_bytes[8..16], second_half);


    let mut repacked = [0u8; 16];
    repacked[0..8].copy_from_slice(&k0.to_le_bytes());
    repacked[8..16].copy_from_slice(&k1.to_le_bytes());
    assert_eq!(repacked, key_bytes);


    assert_eq!(std::mem::size_of_val(&key_bytes), 16);
    assert_eq!(std::mem::size_of_val(&(k0, k1)), 16);
}

#[test]
fn test_keys_are_stable_across_hashing_operations() {
    let mut hasher = SipHasher::new();


    let keys_before = hasher.keys();
    let key_bytes_before = hasher.key();
    assert_eq!(keys_before, (0, 0));
    assert_eq!(key_bytes_before, [0u8; 16]);


    hasher.write(b"the quick brown fox");
    let intermediate = hasher.finish();


    let keys_mid = hasher.keys();
    let key_bytes_mid = hasher.key();
    assert_eq!(keys_mid, keys_before);
    assert_eq!(key_bytes_mid, key_bytes_before);

    hasher.write(b" jumps over the lazy dog");
    hasher.write_u64(0xDEAD_BEEF_CAFE_BABE);
    hasher.write_u8(0xAB);
    let final_hash = hasher.finish();


    let keys_after = hasher.keys();
    let key_bytes_after = hasher.key();
    assert_eq!(keys_after, keys_before);
    assert_eq!(key_bytes_after, key_bytes_before);


    assert_ne!(intermediate, 0u64);
    assert_ne!(final_hash, intermediate);
}

#[test]
fn test_multiple_default_hashers_share_identical_keys() {
    let h1 = SipHasher::new();
    let h2 = SipHasher::new();
    let h3 = SipHasher::new();


    assert_eq!(h1.keys(), h2.keys());
    assert_eq!(h2.keys(), h3.keys());
    assert_eq!(h1.key(), h2.key());
    assert_eq!(h2.key(), h3.key());


    assert_eq!(h1.keys(), (0u64, 0u64));
    assert_eq!(h3.key(), [0u8; 16]);


    let payload: &[u8] = b"integration-test-payload-0123456789";
    let mut a = SipHasher::new();
    let mut b = SipHasher::new();
    a.write(payload);
    b.write(payload);
    let ha = a.finish();
    let hb = b.finish();
    assert_eq!(ha, hb);


    assert_eq!(a.keys(), b.keys());
    assert_eq!(a.key(), b.key());
}

#[test]
fn test_keys_observation_does_not_disturb_hasher_state() {

    let mut baseline = SipHasher::new();
    baseline.write(b"payload-A");
    baseline.write_u32(42);
    baseline.write(b"payload-B");
    let baseline_hash = baseline.finish();


    let mut observed = SipHasher::new();
    let k_before = observed.keys();
    let kb_before = observed.key();
    assert_eq!(k_before, (0, 0));
    assert_eq!(kb_before, [0u8; 16]);

    observed.write(b"payload-A");
    let k_mid1 = observed.keys();
    assert_eq!(k_mid1, k_before);

    observed.write_u32(42);
    let kb_mid = observed.key();
    assert_eq!(kb_mid, kb_before);

    observed.write(b"payload-B");
    let k_mid2 = observed.keys();
    let kb_mid2 = observed.key();
    assert_eq!(k_mid2, k_before);
    assert_eq!(kb_mid2, kb_before);

    let observed_hash = observed.finish();


    assert_eq!(observed_hash, baseline_hash);


    assert_eq!(observed.keys(), (0u64, 0u64));
    assert_eq!(observed.key(), [0u8; 16]);
}

#[test]
fn test_key_bytes_length_and_slice_access() {
    let hasher = SipHasher::new();
    let bytes = hasher.key();


    assert_eq!(bytes.len(), 16);


    let (lo_slice, hi_slice) = bytes.split_at(8);
    assert_eq!(lo_slice.len(), 8);
    assert_eq!(hi_slice.len(), 8);

    let (k0, k1) = hasher.keys();
    let mut lo = [0u8; 8];
    let mut hi = [0u8; 8];
    lo.copy_from_slice(lo_slice);
    hi.copy_from_slice(hi_slice);

    assert_eq!(u64::from_le_bytes(lo), k0);
    assert_eq!(u64::from_le_bytes(hi), k1);


    let bytes2 = hasher.key();
    assert_eq!(bytes, bytes2);

    let keys2 = hasher.keys();
    assert_eq!((k0, k1), keys2);
}