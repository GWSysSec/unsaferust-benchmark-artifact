use openssl::symm::{Cipher, encrypt, decrypt, Crypter, Mode};
use openssl::provider::Provider;

fn load_legacy_provider() -> Option<Provider> {
    Provider::load(None, "legacy").ok()
}

#[test]
fn test_aes_128_ecb_encrypt_decrypt_roundtrip() {
    let cipher = Cipher::aes_128_ecb();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let plaintext = b"Hello AES-128-ECB world!!!!!!!!!" ; // 32 bytes

    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), None);
    assert_eq!(cipher.block_size(), 16);

    let ciphertext = encrypt(cipher, key, None, plaintext).unwrap();
    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..32], &plaintext[..]);

    let decrypted = decrypt(cipher, key, None, &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Encrypt same plaintext again - ECB is deterministic
    let ciphertext2 = encrypt(cipher, key, None, plaintext).unwrap();
    assert_eq!(ciphertext, ciphertext2);
}

#[test]
fn test_aes_192_ecb_encrypt_decrypt() {
    let cipher = Cipher::aes_192_ecb();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17";
    let plaintext = b"AES-192-ECB test data block!!!!!" ; // 32 bytes

    assert_eq!(cipher.key_len(), 24);
    assert_eq!(cipher.iv_len(), None);
    assert_eq!(cipher.block_size(), 16);

    let ciphertext = encrypt(cipher, key, None, plaintext).unwrap();
    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..32], &plaintext[..]);

    let decrypted = decrypt(cipher, key, None, &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Different key produces different ciphertext
    let key2 = b"\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27";
    let ciphertext2 = encrypt(cipher, key2, None, plaintext).unwrap();
    assert_ne!(ciphertext, ciphertext2);
}

#[test]
fn test_aes_192_cbc_encrypt_decrypt() {
    let cipher = Cipher::aes_192_cbc();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let plaintext = b"AES-192-CBC needs an IV to work!";

    assert_eq!(cipher.key_len(), 24);
    assert_eq!(cipher.iv_len(), Some(16));
    assert_eq!(cipher.block_size(), 16);

    let ciphertext = encrypt(cipher, key, Some(iv), plaintext).unwrap();
    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..32], &plaintext[..]);

    let decrypted = decrypt(cipher, key, Some(iv), &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Different IV produces different ciphertext
    let iv2 = b"\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f";
    let ciphertext2 = encrypt(cipher, key, Some(iv2), plaintext).unwrap();
    assert_ne!(ciphertext, ciphertext2);
}

#[test]
fn test_aes_192_gcm_encrypt_decrypt_with_tag() {
    let cipher = Cipher::aes_192_gcm();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b";
    let plaintext = b"AES-192-GCM authenticated encryption test data";
    let aad = b"additional authenticated data";

    assert_eq!(cipher.key_len(), 24);
    assert_eq!(cipher.iv_len(), Some(12));

    // Encrypt
    let mut tag = vec![0u8; 16];
    let mut encrypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv)).unwrap();
    encrypter.aad_update(aad).unwrap();
    let mut ciphertext = vec![0u8; plaintext.len() + cipher.block_size()];
    let mut count = encrypter.update(plaintext, &mut ciphertext).unwrap();
    count += encrypter.finalize(&mut ciphertext[count..]).unwrap();
    ciphertext.truncate(count);
    encrypter.get_tag(&mut tag).unwrap();

    assert_eq!(ciphertext.len(), plaintext.len());
    assert_ne!(&ciphertext[..], &plaintext[..]);
    assert_eq!(tag.len(), 16);

    // Decrypt
    let mut decrypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv)).unwrap();
    decrypter.aad_update(aad).unwrap();
    let mut decrypted = vec![0u8; ciphertext.len() + cipher.block_size()];
    let mut dec_count = decrypter.update(&ciphertext, &mut decrypted).unwrap();
    decrypter.set_tag(&tag).unwrap();
    dec_count += decrypter.finalize(&mut decrypted[dec_count..]).unwrap();
    decrypted.truncate(dec_count);

    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);
}

#[test]
fn test_aes_256_gcm_encrypt_decrypt_with_tag() {
    let cipher = Cipher::aes_256_gcm();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b";
    let plaintext = b"AES-256-GCM is the gold standard for authenticated encryption";
    let aad = b"metadata that must be authenticated";

    assert_eq!(cipher.key_len(), 32);
    assert_eq!(cipher.iv_len(), Some(12));

    // Encrypt
    let mut tag = vec![0u8; 16];
    let mut encrypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv)).unwrap();
    encrypter.aad_update(aad).unwrap();
    let mut ciphertext = vec![0u8; plaintext.len() + cipher.block_size()];
    let mut count = encrypter.update(plaintext, &mut ciphertext).unwrap();
    count += encrypter.finalize(&mut ciphertext[count..]).unwrap();
    ciphertext.truncate(count);
    encrypter.get_tag(&mut tag).unwrap();

    assert_eq!(ciphertext.len(), plaintext.len());
    assert_ne!(&ciphertext[..], &plaintext[..]);

    // Decrypt with correct tag
    let mut decrypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv)).unwrap();
    decrypter.aad_update(aad).unwrap();
    let mut decrypted = vec![0u8; ciphertext.len() + cipher.block_size()];
    let mut dec_count = decrypter.update(&ciphertext, &mut decrypted).unwrap();
    decrypter.set_tag(&tag).unwrap();
    dec_count += decrypter.finalize(&mut decrypted[dec_count..]).unwrap();
    decrypted.truncate(dec_count);

    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Tampered tag should fail
    let mut bad_tag = tag.clone();
    bad_tag[0] ^= 0xff;
    let mut decrypter2 = Crypter::new(cipher, Mode::Decrypt, key, Some(iv)).unwrap();
    decrypter2.aad_update(aad).unwrap();
    let mut decrypted2 = vec![0u8; ciphertext.len() + cipher.block_size()];
    let _ = decrypter2.update(&ciphertext, &mut decrypted2).unwrap();
    decrypter2.set_tag(&bad_tag).unwrap();
    let result = decrypter2.finalize(&mut decrypted2[ciphertext.len()..]);
    assert!(result.is_err());
}

#[test]
fn test_bf_cbc_encrypt_decrypt() {
    let _legacy = load_legacy_provider();
    let cipher = Cipher::bf_cbc();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07";
    let plaintext = b"Blowfish CBC mode encryption test data here!!!";

    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), Some(8));
    assert_eq!(cipher.block_size(), 8);

    let ciphertext = encrypt(cipher, key, Some(iv), plaintext).unwrap();
    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..8], &plaintext[..8]);

    let decrypted = decrypt(cipher, key, Some(iv), &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Different IV produces different ciphertext
    let iv2 = b"\x10\x11\x12\x13\x14\x15\x16\x17";
    let ciphertext2 = encrypt(cipher, key, Some(iv2), plaintext).unwrap();
    assert_ne!(ciphertext, ciphertext2);
}

#[test]
fn test_bf_ecb_encrypt_decrypt() {
    let _legacy = load_legacy_provider();
    let cipher = Cipher::bf_ecb();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let plaintext = b"BF-ECB!!" ; // 8 bytes = one block

    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), None);
    assert_eq!(cipher.block_size(), 8);

    let ciphertext = encrypt(cipher, key, None, plaintext).unwrap();
    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..8], &plaintext[..]);

    let decrypted = decrypt(cipher, key, None, &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // ECB is deterministic
    let ciphertext2 = encrypt(cipher, key, None, plaintext).unwrap();
    assert_eq!(ciphertext, ciphertext2);
}

#[test]
fn test_bf_cfb64_encrypt_decrypt() {
    let _legacy = load_legacy_provider();
    let cipher = Cipher::bf_cfb64();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07";
    let plaintext = b"Blowfish CFB64 stream cipher mode test";

    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), Some(8));

    let ciphertext = encrypt(cipher, key, Some(iv), plaintext).unwrap();
    assert_eq!(ciphertext.len(), plaintext.len());
    assert_ne!(&ciphertext[..], &plaintext[..]);

    let decrypted = decrypt(cipher, key, Some(iv), &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Verify stream cipher property: ciphertext length equals plaintext length
    let short_pt = b"Hi";
    let short_ct = encrypt(cipher, key, Some(iv), short_pt).unwrap();
    assert_eq!(short_ct.len(), short_pt.len());

    let short_dec = decrypt(cipher, key, Some(iv), &short_ct).unwrap();
    assert_eq!(&short_dec[..], &short_pt[..]);
}

#[test]
fn test_bf_ofb_encrypt_decrypt() {
    let _legacy = load_legacy_provider();
    let cipher = Cipher::bf_ofb();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07";
    let plaintext = b"Blowfish OFB mode test data for streaming";

    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), Some(8));

    let ciphertext = encrypt(cipher, key, Some(iv), plaintext).unwrap();
    assert_eq!(ciphertext.len(), plaintext.len());
    assert_ne!(&ciphertext[..], &plaintext[..]);

    let decrypted = decrypt(cipher, key, Some(iv), &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // OFB is a stream cipher - output length matches input
    let one_byte = b"X";
    let one_ct = encrypt(cipher, key, Some(iv), one_byte).unwrap();
    assert_eq!(one_ct.len(), 1);

    let one_dec = decrypt(cipher, key, Some(iv), &one_ct).unwrap();
    assert_eq!(&one_dec[..], &one_byte[..]);
}

#[test]
fn test_des_ede3_ecb_encrypt_decrypt() {
    let cipher = Cipher::des_ede3_ecb();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17";
    let plaintext = b"3DES-ECB test data block!!!!!!!!" ; // 32 bytes

    assert_eq!(cipher.key_len(), 24);
    assert_eq!(cipher.iv_len(), None);
    assert_eq!(cipher.block_size(), 8);

    let ciphertext = encrypt(cipher, key, None, plaintext).unwrap();
    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..8], &plaintext[..8]);

    let decrypted = decrypt(cipher, key, None, &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // ECB determinism
    let ciphertext2 = encrypt(cipher, key, None, plaintext).unwrap();
    assert_eq!(ciphertext, ciphertext2);
}

#[test]
fn test_des_ede3_cfb8_encrypt_decrypt() {
    let cipher = Cipher::des_ede3_cfb8();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07";
    let plaintext = b"Triple DES CFB8 mode streaming cipher test";

    assert_eq!(cipher.key_len(), 24);
    assert_eq!(cipher.iv_len(), Some(8));

    let ciphertext = encrypt(cipher, key, Some(iv), plaintext).unwrap();
    assert_eq!(ciphertext.len(), plaintext.len());
    assert_ne!(&ciphertext[..], &plaintext[..]);

    let decrypted = decrypt(cipher, key, Some(iv), &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Different IV gives different ciphertext
    let iv2 = b"\x10\x11\x12\x13\x14\x15\x16\x17";
    let ciphertext2 = encrypt(cipher, key, Some(iv2), plaintext).unwrap();
    assert_ne!(ciphertext, ciphertext2);
}

#[test]
fn test_des_ede3_ofb_encrypt_decrypt() {
    let cipher = Cipher::des_ede3_ofb();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07";
    let plaintext = b"Triple DES OFB mode test data for streaming cipher";

    assert_eq!(cipher.key_len(), 24);
    assert_eq!(cipher.iv_len(), Some(8));

    let ciphertext = encrypt(cipher, key, Some(iv), plaintext).unwrap();
    assert_eq!(ciphertext.len(), plaintext.len());
    assert_ne!(&ciphertext[..], &plaintext[..]);

    let decrypted = decrypt(cipher, key, Some(iv), &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Stream cipher: arbitrary length
    let short = b"AB";
    let short_ct = encrypt(cipher, key, Some(iv), short).unwrap();
    assert_eq!(short_ct.len(), 2);

    let short_dec = decrypt(cipher, key, Some(iv), &short_ct).unwrap();
    assert_eq!(&short_dec[..], &short[..]);
}

#[test]
fn test_camellia_128_cbc_encrypt_decrypt() {
    let cipher = Cipher::camellia_128_cbc();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let plaintext = b"Camellia-128-CBC block cipher test data!";

    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), Some(16));
    assert_eq!(cipher.block_size(), 16);

    let ciphertext = encrypt(cipher, key, Some(iv), plaintext).unwrap();
    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..16], &plaintext[..16]);

    let decrypted = decrypt(cipher, key, Some(iv), &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Different IV
    let iv2 = b"\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f";
    let ciphertext2 = encrypt(cipher, key, Some(iv2), plaintext).unwrap();
    assert_ne!(ciphertext, ciphertext2);
}

#[test]
fn test_camellia_128_ecb_encrypt_decrypt() {
    let cipher = Cipher::camellia_128_ecb();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let plaintext = b"Camellia128ECB!!" ; // 16 bytes = one block

    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), None);
    assert_eq!(cipher.block_size(), 16);

    let ciphertext = encrypt(cipher, key, None, plaintext).unwrap();
    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..16], &plaintext[..]);

    let decrypted = decrypt(cipher, key, None, &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // ECB determinism
    let ciphertext2 = encrypt(cipher, key, None, plaintext).unwrap();
    assert_eq!(ciphertext, ciphertext2);
}

#[test]
fn test_camellia_128_ofb_encrypt_decrypt() {
    let cipher = Cipher::camellia_128_ofb();
    let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let iv = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let plaintext = b"Camellia-128-OFB stream cipher mode test data here";

    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), Some(16));

    let ciphertext = encrypt(cipher, key, Some(iv), plaintext).unwrap();
    assert_eq!(ciphertext.len(), plaintext.len());
    assert_ne!(&ciphertext[..], &plaintext[..]);

    let decrypted = decrypt(cipher, key, Some(iv), &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Stream cipher: single byte
    let one = b"Z";
    let one_ct = encrypt(cipher, key, Some(iv), one).unwrap();
    assert_eq!(one_ct.len(), 1);

    let one_dec = decrypt(cipher, key, Some(iv), &one_ct).unwrap();
    assert_eq!(&one_dec[..], &one[..]);
}

#[test]
fn test_crypter_incremental_aes_128_ecb() {
    let cipher = Cipher::aes_128_ecb();
    let key = b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10";
    let plaintext = b"Incremental encryption with Crypter API for AES-128-ECB mode!!";

    // Encrypt incrementally
    let mut encrypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    let block_size = cipher.block_size();
    let mut ciphertext = vec![0u8; plaintext.len() + block_size];
    let mut count = 0;
    // Feed in chunks
    count += encrypter.update(&plaintext[..16], &mut ciphertext[count..]).unwrap();
    count += encrypter.update(&plaintext[16..32], &mut ciphertext[count..]).unwrap();
    count += encrypter.update(&plaintext[32..], &mut ciphertext[count..]).unwrap();
    count += encrypter.finalize(&mut ciphertext[count..]).unwrap();
    ciphertext.truncate(count);

    assert!(!ciphertext.is_empty());
    assert_ne!(&ciphertext[..16], &plaintext[..16]);

    // Decrypt all at once
    let decrypted = decrypt(cipher, key, None, &ciphertext).unwrap();
    assert_eq!(decrypted.len(), plaintext.len());
    assert_eq!(&decrypted[..], &plaintext[..]);

    // Verify single-shot matches incremental
    let single_ct = encrypt(cipher, key, None, plaintext).unwrap();
    assert_eq!(ciphertext, single_ct);
}

#[test]
fn test_cross_cipher_independence() {
    let _legacy = load_legacy_provider();
    // Verify that different ciphers with same key material produce different outputs
    let key16 = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let iv8 = b"\x00\x01\x02\x03\x04\x05\x06\x07";
    let iv16 = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f";
    let plaintext = b"Cross cipher independence test!!"; // 32 bytes

    let ct_aes_ecb = encrypt(Cipher::aes_128_ecb(), key16, None, plaintext).unwrap();
    let ct_cam_ecb = encrypt(Cipher::camellia_128_ecb(), key16, None, plaintext).unwrap();
    let ct_bf_cbc = encrypt(Cipher::bf_cbc(), key16, Some(iv8), plaintext).unwrap();
    let ct_cam_cbc = encrypt(Cipher::camellia_128_cbc(), key16, Some(iv16), plaintext).unwrap();

    // All ciphertexts should differ from each other
    assert_ne!(ct_aes_ecb, ct_cam_ecb);
    assert_ne!(ct_aes_ecb, ct_bf_cbc);
    assert_ne!(ct_cam_ecb, ct_cam_cbc);
    assert_ne!(ct_bf_cbc, ct_cam_cbc);

    // All should decrypt back to original
    let dec1 = decrypt(Cipher::aes_128_ecb(), key16, None, &ct_aes_ecb).unwrap();
    let dec2 = decrypt(Cipher::camellia_128_ecb(), key16, None, &ct_cam_ecb).unwrap();
    let dec3 = decrypt(Cipher::bf_cbc(), key16, Some(iv8), &ct_bf_cbc).unwrap();
    let dec4 = decrypt(Cipher::camellia_128_cbc(), key16, Some(iv16), &ct_cam_cbc).unwrap();

    assert_eq!(&dec1[..], &plaintext[..]);
    assert_eq!(&dec2[..], &plaintext[..]);
    assert_eq!(&dec3[..], &plaintext[..]);
    assert_eq!(&dec4[..], &plaintext[..]);
}