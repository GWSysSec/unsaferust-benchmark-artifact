use libsecp256k1::*;

const TEST_SECRET_KEY: [u8; 32] = [
    0xc9, 0xaf, 0xa9, 0xd8, 0x45, 0xba, 0x75, 0x16,
    0x6b, 0x5c, 0x21, 0x57, 0x67, 0xb1, 0xd6, 0x93,
    0x4e, 0x50, 0xc3, 0xdb, 0x36, 0xe8, 0x9b, 0x12,
    0x7b, 0x8a, 0x62, 0x2b, 0x12, 0x0f, 0x67, 0x21,
];

const TEST_MESSAGE: [u8; 32] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
];

#[test]
fn test_parse_overflowing_roundtrip_with_sign_verify() {
    let seckey = SecretKey::parse(&TEST_SECRET_KEY).unwrap();
    let message = Message::parse(&TEST_MESSAGE);

    let (signature, recovery_id) = sign(&message, &seckey);

    let serialized = signature.serialize();
    assert_eq!(serialized.len(), 64);

    let parsed = Signature::parse_overflowing(&serialized);
    let parsed_serialized = parsed.serialize();


    assert_eq!(serialized, parsed_serialized);


    let pubkey = PublicKey::from_secret_key(&seckey);
    assert!(verify(&message, &parsed, &pubkey));


    let recovered = recover(&message, &parsed, &recovery_id).unwrap();
    assert_eq!(recovered, pubkey);


    let other_message_bytes: [u8; 32] = [0xffu8; 32];
    let other_message = Message::parse(&other_message_bytes);
    assert!(!verify(&other_message, &parsed, &pubkey));


    let rid_val = recovery_id.serialize();
    assert!(rid_val < 4);
}

#[test]
fn test_parse_overflowing_zeros_signature() {
    let zero_sig_bytes: [u8; 64] = [0u8; 64];
    let sig = Signature::parse_overflowing(&zero_sig_bytes);
    let serialized = sig.serialize();


    assert_eq!(serialized, zero_sig_bytes);


    let seckey = SecretKey::parse(&TEST_SECRET_KEY).unwrap();
    let pubkey = PublicKey::from_secret_key(&seckey);
    let message = Message::parse(&TEST_MESSAGE);
    assert!(!verify(&message, &sig, &pubkey));


    let max_sig_bytes: [u8; 64] = [0xffu8; 64];
    let max_sig = Signature::parse_overflowing(&max_sig_bytes);
    let max_serialized = max_sig.serialize();



    assert_eq!(max_serialized.len(), 64);


    assert!(!verify(&message, &max_sig, &pubkey));
}

#[test]
fn test_parse_overflowing_slice_valid() {
    let seckey = SecretKey::parse(&TEST_SECRET_KEY).unwrap();
    let message = Message::parse(&TEST_MESSAGE);
    let (signature, _recovery_id) = sign(&message, &seckey);

    let serialized = signature.serialize();
    let slice: &[u8] = &serialized[..];


    let parsed_result = Signature::parse_overflowing_slice(slice);
    assert!(parsed_result.is_ok());

    let parsed = parsed_result.unwrap();
    let parsed_serialized = parsed.serialize();
    assert_eq!(serialized, parsed_serialized);


    let pubkey = PublicKey::from_secret_key(&seckey);
    assert!(verify(&message, &parsed, &pubkey));


    let short_slice: &[u8] = &serialized[..63];
    let short_result = Signature::parse_overflowing_slice(short_slice);
    assert!(short_result.is_err());


    let mut long_vec = serialized.to_vec();
    long_vec.push(0x00);
    let long_result = Signature::parse_overflowing_slice(&long_vec);
    assert!(long_result.is_err());


    let empty_slice: &[u8] = &[];
    let empty_result = Signature::parse_overflowing_slice(empty_slice);
    assert!(empty_result.is_err());
}

#[test]
fn test_parse_overflowing_slice_boundary_values() {

    let zero_bytes = vec![0u8; 64];
    let result = Signature::parse_overflowing_slice(&zero_bytes);
    assert!(result.is_ok());
    let sig = result.unwrap();
    let serialized = sig.serialize();
    assert_eq!(serialized, [0u8; 64]);


    let max_bytes = vec![0xffu8; 64];
    let result_max = Signature::parse_overflowing_slice(&max_bytes);
    assert!(result_max.is_ok());
    let sig_max = result_max.unwrap();
    let max_serialized = sig_max.serialize();


    assert_eq!(max_serialized.len(), 64);


    let one_byte: &[u8] = &[0x42];
    assert!(Signature::parse_overflowing_slice(one_byte).is_err());


    let thirty_two = vec![0xabu8; 32];
    assert!(Signature::parse_overflowing_slice(&thirty_two).is_err());
}

#[test]
fn test_parse_standard_slice_valid_signature() {
    let seckey = SecretKey::parse(&TEST_SECRET_KEY).unwrap();
    let message = Message::parse(&TEST_MESSAGE);
    let (signature, _recovery_id) = sign(&message, &seckey);

    let serialized = signature.serialize();
    let slice: &[u8] = &serialized[..];


    let parsed_result = Signature::parse_standard_slice(slice);
    assert!(parsed_result.is_ok());

    let parsed = parsed_result.unwrap();
    let pubkey = PublicKey::from_secret_key(&seckey);
    assert!(verify(&message, &parsed, &pubkey));


    let re_serialized = parsed.serialize();
    assert_eq!(serialized, re_serialized);


    let short: &[u8] = &serialized[..32];
    assert!(Signature::parse_standard_slice(short).is_err());


    let mut long = serialized.to_vec();
    long.extend_from_slice(&[0u8; 10]);
    assert!(Signature::parse_standard_slice(&long).is_err());


    assert!(Signature::parse_standard_slice(&[]).is_err());
}

#[test]
fn test_parse_standard_slice_rejects_overflowing() {





    let mut overflowing_r: [u8; 64] = [0u8; 64];

    for i in 0..32 {
        overflowing_r[i] = 0xff;
    }

    overflowing_r[63] = 0x01;

    let result = Signature::parse_standard_slice(&overflowing_r);

    assert!(result.is_err());


    let overflowing_result = Signature::parse_overflowing_slice(&overflowing_r);
    assert!(overflowing_result.is_ok());


    let mut overflowing_s: [u8; 64] = [0u8; 64];
    overflowing_s[0] = 0x01;
    for i in 32..64 {
        overflowing_s[i] = 0xff;
    }

    let result_s = Signature::parse_standard_slice(&overflowing_s);

    assert!(result_s.is_err());


    let overflowing_s_result = Signature::parse_overflowing_slice(&overflowing_s);
    assert!(overflowing_s_result.is_ok());
}

#[test]
fn test_parse_standard_slice_zero_components() {




    let zero_sig = [0u8; 64];
    let result = Signature::parse_standard_slice(&zero_sig);


    assert!(result.is_ok());
    let sig = result.unwrap();
    let seckey = SecretKey::parse(&TEST_SECRET_KEY).unwrap();
    let pubkey = PublicKey::from_secret_key(&seckey);
    let message = Message::parse(&TEST_MESSAGE);

    assert!(!verify(&message, &sig, &pubkey));


    let mut zero_r2: [u8; 64] = [0u8; 64];
    zero_r2[63] = 0x01;
    let result_zero_r = Signature::parse_standard_slice(&zero_r2);
    assert!(result_zero_r.is_ok());
    let sig_zero_r = result_zero_r.unwrap();
    assert!(!verify(&message, &sig_zero_r, &pubkey));


    let mut zero_s: [u8; 64] = [0u8; 64];
    zero_s[31] = 0x01;
    let result_zero_s = Signature::parse_standard_slice(&zero_s);
    assert!(result_zero_s.is_ok());
    let sig_zero_s = result_zero_s.unwrap();
    assert!(!verify(&message, &sig_zero_s, &pubkey));


    let mut one_one: [u8; 64] = [0u8; 64];
    one_one[31] = 0x01;
    one_one[63] = 0x01;
    let result_one_one = Signature::parse_standard_slice(&one_one);
    assert!(result_one_one.is_ok());
    let sig_one_one = result_one_one.unwrap();
    assert_eq!(sig_one_one.serialize().len(), 64);

    assert!(!verify(&message, &sig_one_one, &pubkey));


    let result_one_one_overflowing = Signature::parse_overflowing_slice(&one_one);
    assert!(result_one_one_overflowing.is_ok());
}

#[test]
fn test_parse_overflowing_vs_standard_equivalence_for_valid_sigs() {


    let seckey = SecretKey::parse(&TEST_SECRET_KEY).unwrap();
    let message = Message::parse(&TEST_MESSAGE);
    let (signature, recovery_id) = sign(&message, &seckey);

    let serialized = signature.serialize();

    let from_overflowing = Signature::parse_overflowing(&serialized);
    let from_overflowing_slice = Signature::parse_overflowing_slice(&serialized).unwrap();
    let from_standard_slice = Signature::parse_standard_slice(&serialized).unwrap();


    assert_eq!(from_overflowing.serialize(), serialized);
    assert_eq!(from_overflowing_slice.serialize(), serialized);
    assert_eq!(from_standard_slice.serialize(), serialized);


    let pubkey = PublicKey::from_secret_key(&seckey);
    assert!(verify(&message, &from_overflowing, &pubkey));
    assert!(verify(&message, &from_overflowing_slice, &pubkey));
    assert!(verify(&message, &from_standard_slice, &pubkey));


    let recovered1 = recover(&message, &from_overflowing, &recovery_id).unwrap();
    let recovered2 = recover(&message, &from_overflowing_slice, &recovery_id).unwrap();
    let recovered3 = recover(&message, &from_standard_slice, &recovery_id).unwrap();
    assert_eq!(recovered1, pubkey);
    assert_eq!(recovered2, pubkey);
    assert_eq!(recovered3, pubkey);
}

#[test]
fn test_parse_overflowing_multiple_messages_different_keys() {
    let key_bytes_1: [u8; 32] = [
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    ];
    let key_bytes_2: [u8; 32] = [
        0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
        0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
        0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
        0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    ];

    let sk1 = SecretKey::parse(&key_bytes_1).unwrap();
    let sk2 = SecretKey::parse(&key_bytes_2).unwrap();
    let pk1 = PublicKey::from_secret_key(&sk1);
    let pk2 = PublicKey::from_secret_key(&sk2);

    let msg = Message::parse(&TEST_MESSAGE);

    let (sig1, _rid1) = sign(&msg, &sk1);
    let (sig2, _rid2) = sign(&msg, &sk2);

    let ser1 = sig1.serialize();
    let ser2 = sig2.serialize();


    assert_ne!(ser1, ser2);


    let parsed1 = Signature::parse_overflowing(&ser1);
    let parsed2 = Signature::parse_overflowing(&ser2);

    assert!(verify(&msg, &parsed1, &pk1));
    assert!(!verify(&msg, &parsed1, &pk2));
    assert!(verify(&msg, &parsed2, &pk2));
    assert!(!verify(&msg, &parsed2, &pk1));
}

#[test]
fn test_parse_overflowing_slice_exact_boundary() {

    let seckey = SecretKey::parse(&TEST_SECRET_KEY).unwrap();
    let message = Message::parse(&TEST_MESSAGE);
    let (signature, _) = sign(&message, &seckey);
    let serialized = signature.serialize();


    assert!(Signature::parse_overflowing_slice(&serialized[..64]).is_ok());


    assert!(Signature::parse_overflowing_slice(&serialized[..63]).is_err());


    let mut extended = serialized.to_vec();
    extended.push(0x00);
    assert!(Signature::parse_overflowing_slice(&extended).is_err());


    assert!(Signature::parse_standard_slice(&serialized[..64]).is_ok());
    assert!(Signature::parse_standard_slice(&serialized[..63]).is_err());
    assert!(Signature::parse_standard_slice(&extended).is_err());


    let parsed = Signature::parse_overflowing_slice(&serialized[..64]).unwrap();
    let pubkey = PublicKey::from_secret_key(&seckey);
    assert!(verify(&message, &parsed, &pubkey));
}