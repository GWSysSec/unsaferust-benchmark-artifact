use openssl::symm::{decrypt, encrypt, Cipher, Crypter, Mode};
use openssl::provider::Provider;

fn roundtrip(cipher: Cipher, key: &[u8], iv: Option<&[u8]>, plaintext: &[u8]) {
    let ct = encrypt(cipher, key, iv, plaintext).expect("encrypt");
    let pt = decrypt(cipher, key, iv, &ct).expect("decrypt");
    assert_eq!(pt, plaintext);
    assert_ne!(ct, plaintext);
    assert_eq!(cipher.key_len(), key.len());
    if let Some(iv_bytes) = iv {
        assert_eq!(cipher.iv_len().unwrap_or(0), iv_bytes.len());
    }
}

#[test]
fn test_camellia_128_cfb128() {
    let _legacy = Provider::load(None, "legacy").expect("legacy provider");
    let _default = Provider::load(None, "default").expect("default provider");

    let cipher = Cipher::camellia_128_cfb128();
    assert_eq!(cipher.key_len(), 16);
    assert_eq!(cipher.iv_len(), Some(16));
    let key = [0x11u8; 16];
    let iv = [0x22u8; 16];
    let pt = b"Hello, camellia 128 CFB128 mode!";
    roundtrip(cipher, &key, Some(&iv), pt);

    let ct = encrypt(cipher, &key, Some(&iv), pt).unwrap();
    assert_eq!(ct.len(), pt.len());
}

#[test]
fn test_camellia_192_modes() {
    let _legacy = Provider::load(None, "legacy").expect("legacy provider");
    let _default = Provider::load(None, "default").expect("default provider");

    let key = [0xAAu8; 24];
    let iv = [0xBBu8; 16];
    let pt = b"camellia-192 multi-mode payload!";

    let cbc = Cipher::camellia_192_cbc();
    assert_eq!(cbc.key_len(), 24);
    assert_eq!(cbc.block_size(), 16);
    roundtrip(cbc, &key, Some(&iv), pt);

    let ecb = Cipher::camellia_192_ecb();
    assert_eq!(ecb.key_len(), 24);
    assert_eq!(ecb.iv_len(), None);
    roundtrip(ecb, &key, None, pt);

    let ofb = Cipher::camellia_192_ofb();
    assert_eq!(ofb.key_len(), 24);
    let ct_ofb = encrypt(ofb, &key, Some(&iv), pt).unwrap();
    assert_eq!(ct_ofb.len(), pt.len());
    assert_eq!(decrypt(ofb, &key, Some(&iv), &ct_ofb).unwrap(), pt);

    let cfb = Cipher::camellia_192_cfb128();
    assert_eq!(cfb.key_len(), 24);
    let ct_cfb = encrypt(cfb, &key, Some(&iv), pt).unwrap();
    assert_eq!(ct_cfb.len(), pt.len());
    assert_ne!(ct_cfb, ct_ofb);
}

#[test]
fn test_camellia_256_modes() {
    let _legacy = Provider::load(None, "legacy").expect("legacy provider");
    let _default = Provider::load(None, "default").expect("default provider");

    let key = [0xCCu8; 32];
    let iv = [0xDDu8; 16];
    let pt = b"camellia-256 secret content here";

    let cbc = Cipher::camellia_256_cbc();
    assert_eq!(cbc.key_len(), 32);
    assert_eq!(cbc.iv_len(), Some(16));
    roundtrip(cbc, &key, Some(&iv), pt);

    let ecb = Cipher::camellia_256_ecb();
    assert_eq!(ecb.key_len(), 32);
    assert_eq!(ecb.iv_len(), None);
    roundtrip(ecb, &key, None, pt);

    let ofb = Cipher::camellia_256_ofb();
    assert_eq!(ofb.key_len(), 32);
    roundtrip(ofb, &key, Some(&iv), pt);

    let cfb = Cipher::camellia_256_cfb128();
    assert_eq!(cfb.key_len(), 32);
    roundtrip(cfb, &key, Some(&iv), pt);


    let c_cbc = encrypt(cbc, &key, Some(&iv), pt).unwrap();
    let c_ofb = encrypt(ofb, &key, Some(&iv), pt).unwrap();
    let c_cfb = encrypt(cfb, &key, Some(&iv), pt).unwrap();
    assert_ne!(c_cbc, c_ofb);
    assert_ne!(c_ofb, c_cfb);
    assert_ne!(c_cbc, c_cfb);
}

#[test]
fn test_cast5_modes() {
    let _legacy = Provider::load(None, "legacy").expect("legacy provider");
    let _default = Provider::load(None, "default").expect("default provider");

    let key = [0x01u8; 16];
    let iv = [0x02u8; 8];
    let pt = b"cast5 block cipher test payload!";

    let cbc = Cipher::cast5_cbc();
    assert_eq!(cbc.block_size(), 8);
    assert_eq!(cbc.iv_len(), Some(8));
    roundtrip(cbc, &key, Some(&iv), pt);

    let ecb = Cipher::cast5_ecb();
    assert_eq!(ecb.iv_len(), None);
    assert_eq!(ecb.block_size(), 8);
    roundtrip(ecb, &key, None, pt);

    let ofb = Cipher::cast5_ofb();
    assert_eq!(ofb.iv_len(), Some(8));
    let ct = encrypt(ofb, &key, Some(&iv), pt).unwrap();
    assert_eq!(ct.len(), pt.len());
    assert_eq!(decrypt(ofb, &key, Some(&iv), &ct).unwrap(), pt);

    let cfb = Cipher::cast5_cfb64();
    assert_eq!(cfb.iv_len(), Some(8));
    let ct2 = encrypt(cfb, &key, Some(&iv), pt).unwrap();
    assert_eq!(ct2.len(), pt.len());
    assert_ne!(ct2, ct);
}

#[test]
fn test_seed_modes_streaming() {
    let _legacy = Provider::load(None, "legacy").expect("legacy provider");
    let _default = Provider::load(None, "default").expect("default provider");

    let key = [0x33u8; 16];
    let iv = [0x44u8; 16];
    let pt = b"SEED cipher round-trip test data";

    let cbc = Cipher::seed_cbc();
    assert_eq!(cbc.key_len(), 16);
    assert_eq!(cbc.block_size(), 16);
    assert_eq!(cbc.iv_len(), Some(16));
    roundtrip(cbc, &key, Some(&iv), pt);

    let cfb = Cipher::seed_cfb128();
    assert_eq!(cfb.key_len(), 16);
    assert_eq!(cfb.iv_len(), Some(16));


    let mut enc = Crypter::new(cfb, Mode::Encrypt, &key, Some(&iv)).unwrap();
    enc.pad(false);
    let mut out = vec![0u8; pt.len() + cfb.block_size()];
    let n = enc.update(pt, &mut out).unwrap();
    let n2 = enc.finalize(&mut out[n..]).unwrap();
    out.truncate(n + n2);
    assert_eq!(out.len(), pt.len());

    let mut dec = Crypter::new(cfb, Mode::Decrypt, &key, Some(&iv)).unwrap();
    dec.pad(false);
    let mut back = vec![0u8; out.len() + cfb.block_size()];
    let m = dec.update(&out, &mut back).unwrap();
    let m2 = dec.finalize(&mut back[m..]).unwrap();
    back.truncate(m + m2);
    assert_eq!(back, pt);
}