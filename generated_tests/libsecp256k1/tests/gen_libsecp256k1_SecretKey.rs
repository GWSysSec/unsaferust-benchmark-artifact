use libsecp256k1::*;

const SECRET_KEY_1: [u8; 32] = [
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
];

const SECRET_KEY_2: [u8; 32] = [
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
];

const SECRET_KEY_3: [u8; 32] = [
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
];

const MESSAGE_BYTES: [u8; 32] = [
    0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11,
    0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
    0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11,
    0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
];

#[test]
fn test_tweak_add_assign_basic() {
    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak = SecretKey::parse(&SECRET_KEY_2).unwrap();

    let original_serialized = sk.serialize();
    assert_eq!(original_serialized, SECRET_KEY_1);

    let result = sk.tweak_add_assign(&tweak);
    assert!(result.is_ok());

    let tweaked_serialized = sk.serialize();
    assert_ne!(tweaked_serialized, SECRET_KEY_1);
    assert_ne!(tweaked_serialized, SECRET_KEY_2);


    assert_ne!(tweaked_serialized[0], SECRET_KEY_1[0]);


    let parsed = SecretKey::parse(&tweaked_serialized);
    assert!(parsed.is_ok());


    let pubkey = PublicKey::from_secret_key(&sk);
    let serialized_pub = pubkey.serialize();
    assert_eq!(serialized_pub.len(), 65);
    assert_eq!(serialized_pub[0], 0x04);
}

#[test]
fn test_tweak_add_assign_sign_and_verify() {
    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak = SecretKey::parse(&SECRET_KEY_3).unwrap();


    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig_original, recid_original) = sign(&msg, &sk);
    let pubkey_original = PublicKey::from_secret_key(&sk);
    assert!(verify(&msg, &sig_original, &pubkey_original));


    let result = sk.tweak_add_assign(&tweak);
    assert!(result.is_ok());


    let pubkey_tweaked = PublicKey::from_secret_key(&sk);
    assert_ne!(pubkey_original.serialize(), pubkey_tweaked.serialize());


    let (sig_tweaked, recid_tweaked) = sign(&msg, &sk);


    assert!(verify(&msg, &sig_tweaked, &pubkey_tweaked));


    assert!(!verify(&msg, &sig_tweaked, &pubkey_original));


    let recovered = recover(&msg, &sig_tweaked, &recid_tweaked).unwrap();
    assert_eq!(recovered.serialize(), pubkey_tweaked.serialize());
}

#[test]
fn test_tweak_add_assign_commutativity() {

    let mut sk_a = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak_a = SecretKey::parse(&SECRET_KEY_2).unwrap();
    sk_a.tweak_add_assign(&tweak_a).unwrap();

    let mut sk_b = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let tweak_b = SecretKey::parse(&SECRET_KEY_1).unwrap();
    sk_b.tweak_add_assign(&tweak_b).unwrap();

    let serialized_a = sk_a.serialize();
    let serialized_b = sk_b.serialize();
    assert_eq!(serialized_a, serialized_b);


    let pub_a = PublicKey::from_secret_key(&sk_a);
    let pub_b = PublicKey::from_secret_key(&sk_b);
    assert_eq!(pub_a.serialize(), pub_b.serialize());


    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig_a, _) = sign(&msg, &sk_a);
    let (sig_b, _) = sign(&msg, &sk_b);
    assert!(verify(&msg, &sig_a, &pub_a));
    assert!(verify(&msg, &sig_b, &pub_b));
}

#[test]
fn test_tweak_add_assign_multiple_tweaks() {
    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak1 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let tweak2 = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let original = sk.serialize();


    sk.tweak_add_assign(&tweak1).unwrap();
    let after_first = sk.serialize();
    assert_ne!(original, after_first);


    sk.tweak_add_assign(&tweak2).unwrap();
    let after_second = sk.serialize();
    assert_ne!(after_first, after_second);
    assert_ne!(original, after_second);


    let parsed = SecretKey::parse(&after_second);
    assert!(parsed.is_ok());


    let msg = Message::parse(&MESSAGE_BYTES);
    let pubkey = PublicKey::from_secret_key(&sk);
    let (sig, recid) = sign(&msg, &sk);
    assert!(verify(&msg, &sig, &pubkey));

    let recovered = recover(&msg, &sig, &recid).unwrap();
    assert_eq!(recovered.serialize(), pubkey.serialize());
}

#[test]
fn test_tweak_mul_assign_basic() {
    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak = SecretKey::parse(&SECRET_KEY_2).unwrap();

    let original_serialized = sk.serialize();
    assert_eq!(original_serialized, SECRET_KEY_1);

    let result = sk.tweak_mul_assign(&tweak);
    assert!(result.is_ok());

    let tweaked_serialized = sk.serialize();
    assert_ne!(tweaked_serialized, SECRET_KEY_1);
    assert_ne!(tweaked_serialized, SECRET_KEY_2);


    let parsed = SecretKey::parse(&tweaked_serialized);
    assert!(parsed.is_ok());


    let pubkey = PublicKey::from_secret_key(&sk);
    let serialized_pub = pubkey.serialize();
    assert_eq!(serialized_pub.len(), 65);
    assert_eq!(serialized_pub[0], 0x04);
}

#[test]
fn test_tweak_mul_assign_sign_and_verify() {
    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let pubkey_original = PublicKey::from_secret_key(&sk);


    sk.tweak_mul_assign(&tweak).unwrap();

    let pubkey_tweaked = PublicKey::from_secret_key(&sk);
    assert_ne!(pubkey_original.serialize(), pubkey_tweaked.serialize());


    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig, recid) = sign(&msg, &sk);


    assert!(verify(&msg, &sig, &pubkey_tweaked));


    assert!(!verify(&msg, &sig, &pubkey_original));


    let recovered = recover(&msg, &sig, &recid).unwrap();
    assert_eq!(recovered.serialize(), pubkey_tweaked.serialize());
}

#[test]
fn test_tweak_mul_assign_not_commutative_with_add() {

    let mut sk_add = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let mut sk_mul = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak = SecretKey::parse(&SECRET_KEY_2).unwrap();

    sk_add.tweak_add_assign(&tweak).unwrap();
    sk_mul.tweak_mul_assign(&tweak).unwrap();

    let add_serialized = sk_add.serialize();
    let mul_serialized = sk_mul.serialize();
    assert_ne!(add_serialized, mul_serialized);


    assert!(SecretKey::parse(&add_serialized).is_ok());
    assert!(SecretKey::parse(&mul_serialized).is_ok());


    let pub_add = PublicKey::from_secret_key(&sk_add);
    let pub_mul = PublicKey::from_secret_key(&sk_mul);
    assert_ne!(pub_add.serialize(), pub_mul.serialize());


    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig_add, _) = sign(&msg, &sk_add);
    let (sig_mul, _) = sign(&msg, &sk_mul);
    assert!(verify(&msg, &sig_add, &pub_add));
    assert!(verify(&msg, &sig_mul, &pub_mul));
}

#[test]
fn test_tweak_mul_assign_multiple_tweaks() {
    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak1 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let tweak2 = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let original = sk.serialize();


    sk.tweak_mul_assign(&tweak1).unwrap();
    let after_first = sk.serialize();
    assert_ne!(original, after_first);


    sk.tweak_mul_assign(&tweak2).unwrap();
    let after_second = sk.serialize();
    assert_ne!(after_first, after_second);
    assert_ne!(original, after_second);


    let parsed = SecretKey::parse(&after_second);
    assert!(parsed.is_ok());


    let msg = Message::parse(&MESSAGE_BYTES);
    let pubkey = PublicKey::from_secret_key(&sk);
    let (sig, recid) = sign(&msg, &sk);
    assert!(verify(&msg, &sig, &pubkey));

    let recovered = recover(&msg, &sig, &recid).unwrap();
    assert_eq!(recovered.serialize(), pubkey.serialize());
}

#[test]
fn test_tweak_mul_assign_associativity() {


    let mut sk_a = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let mut sk_b = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak1 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let tweak2 = SecretKey::parse(&SECRET_KEY_3).unwrap();


    sk_a.tweak_mul_assign(&tweak1).unwrap();
    sk_a.tweak_mul_assign(&tweak2).unwrap();


    sk_b.tweak_mul_assign(&tweak2).unwrap();
    sk_b.tweak_mul_assign(&tweak1).unwrap();


    assert_eq!(sk_a.serialize(), sk_b.serialize());

    let pub_a = PublicKey::from_secret_key(&sk_a);
    let pub_b = PublicKey::from_secret_key(&sk_b);
    assert_eq!(pub_a.serialize(), pub_b.serialize());


    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig_a, _) = sign(&msg, &sk_a);
    let (sig_b, _) = sign(&msg, &sk_b);
    assert!(verify(&msg, &sig_a, &pub_a));
    assert!(verify(&msg, &sig_b, &pub_b));
}

#[test]
fn test_tweak_add_then_mul_workflow() {

    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let add_tweak = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let mul_tweak = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let original_pub = PublicKey::from_secret_key(&sk);


    sk.tweak_add_assign(&add_tweak).unwrap();
    let after_add_pub = PublicKey::from_secret_key(&sk);
    assert_ne!(original_pub.serialize(), after_add_pub.serialize());


    sk.tweak_mul_assign(&mul_tweak).unwrap();
    let after_mul_pub = PublicKey::from_secret_key(&sk);
    assert_ne!(after_add_pub.serialize(), after_mul_pub.serialize());
    assert_ne!(original_pub.serialize(), after_mul_pub.serialize());


    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig, recid) = sign(&msg, &sk);
    assert!(verify(&msg, &sig, &after_mul_pub));
    assert!(!verify(&msg, &sig, &original_pub));
    assert!(!verify(&msg, &sig, &after_add_pub));


    let recovered = recover(&msg, &sig, &recid).unwrap();
    assert_eq!(recovered.serialize(), after_mul_pub.serialize());
}

#[test]
fn test_tweak_add_assign_self_tweak() {

    let sk_original = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak = SecretKey::parse(&SECRET_KEY_1).unwrap();

    sk.tweak_add_assign(&tweak).unwrap();

    let doubled = sk.serialize();
    assert_ne!(doubled, SECRET_KEY_1);


    let parsed = SecretKey::parse(&doubled);
    assert!(parsed.is_ok());


    let pub_original = PublicKey::from_secret_key(&sk_original);
    let pub_doubled = PublicKey::from_secret_key(&sk);
    assert_ne!(pub_original.serialize(), pub_doubled.serialize());


    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig, recid) = sign(&msg, &sk);
    assert!(verify(&msg, &sig, &pub_doubled));
    assert!(!verify(&msg, &sig, &pub_original));

    let recovered = recover(&msg, &sig, &recid).unwrap();
    assert_eq!(recovered.serialize(), pub_doubled.serialize());
}

#[test]
fn test_tweak_mul_assign_self_tweak() {

    let sk_original = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let mut sk = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak = SecretKey::parse(&SECRET_KEY_1).unwrap();

    sk.tweak_mul_assign(&tweak).unwrap();

    let squared = sk.serialize();
    assert_ne!(squared, SECRET_KEY_1);


    let parsed = SecretKey::parse(&squared);
    assert!(parsed.is_ok());


    let pub_original = PublicKey::from_secret_key(&sk_original);
    let pub_squared = PublicKey::from_secret_key(&sk);
    assert_ne!(pub_original.serialize(), pub_squared.serialize());


    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig, recid) = sign(&msg, &sk);
    assert!(verify(&msg, &sig, &pub_squared));
    assert!(!verify(&msg, &sig, &pub_original));

    let recovered = recover(&msg, &sig, &recid).unwrap();
    assert_eq!(recovered.serialize(), pub_squared.serialize());
}

#[test]
fn test_tweak_add_deterministic() {

    let mut sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let mut sk2 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak1 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let tweak2 = SecretKey::parse(&SECRET_KEY_2).unwrap();

    sk1.tweak_add_assign(&tweak1).unwrap();
    sk2.tweak_add_assign(&tweak2).unwrap();

    assert_eq!(sk1.serialize(), sk2.serialize());

    let pub1 = PublicKey::from_secret_key(&sk1);
    let pub2 = PublicKey::from_secret_key(&sk2);
    assert_eq!(pub1.serialize(), pub2.serialize());

    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig1, recid1) = sign(&msg, &sk1);
    let (sig2, recid2) = sign(&msg, &sk2);
    assert_eq!(sig1.serialize(), sig2.serialize());
    assert_eq!(recid1.serialize(), recid2.serialize());
}

#[test]
fn test_tweak_mul_deterministic() {

    let mut sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let mut sk2 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak1 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let tweak2 = SecretKey::parse(&SECRET_KEY_2).unwrap();

    sk1.tweak_mul_assign(&tweak1).unwrap();
    sk2.tweak_mul_assign(&tweak2).unwrap();

    assert_eq!(sk1.serialize(), sk2.serialize());

    let pub1 = PublicKey::from_secret_key(&sk1);
    let pub2 = PublicKey::from_secret_key(&sk2);
    assert_eq!(pub1.serialize(), pub2.serialize());

    let msg = Message::parse(&MESSAGE_BYTES);
    let (sig1, recid1) = sign(&msg, &sk1);
    let (sig2, recid2) = sign(&msg, &sk2);
    assert_eq!(sig1.serialize(), sig2.serialize());
    assert_eq!(recid1.serialize(), recid2.serialize());
}