use libsecp256k1::*;
use libsecp256k1::curve::ECMultContext;
use libsecp256k1::curve::ECMultGenContext;

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
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05,
];

const MESSAGE_BYTES: [u8; 32] = [
    0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11,
    0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
    0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11,
    0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
];

#[test]
fn test_tweak_add_assign_basic() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();

    let original_pubkey = PublicKey::from_secret_key(&sk1);
    let mut tweaked_pubkey = PublicKey::from_secret_key(&sk1);

    let original_serialized = original_pubkey.serialize();
    let pre_tweak_serialized = tweaked_pubkey.serialize();
    assert_eq!(original_serialized, pre_tweak_serialized);

    let result = tweaked_pubkey.tweak_add_assign(&sk2);
    assert!(result.is_ok());

    let post_tweak_serialized = tweaked_pubkey.serialize();
    assert_ne!(original_serialized, post_tweak_serialized);
    assert_ne!(pre_tweak_serialized, post_tweak_serialized);


    let message = Message::parse(&MESSAGE_BYTES);

    let (sig, _recid) = sign(&message, &sk1);
    let verify_original = verify(&message, &sig, &original_pubkey);
    assert!(verify_original);
    let verify_tweaked = verify(&message, &sig, &tweaked_pubkey);
    assert!(!verify_tweaked);


    let mut double_tweaked = PublicKey::from_secret_key(&sk1);
    double_tweaked.tweak_add_assign(&sk2).unwrap();
    double_tweaked.tweak_add_assign(&sk2).unwrap();
    let double_tweaked_serialized = double_tweaked.serialize();
    assert_ne!(post_tweak_serialized, double_tweaked_serialized);
}

#[test]
fn test_tweak_add_assign_with_context_matches_without_context() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();

    let context = ECMultContext::new_boxed();

    let mut pubkey_no_ctx = PublicKey::from_secret_key(&sk1);
    let mut pubkey_with_ctx = PublicKey::from_secret_key(&sk1);

    assert_eq!(pubkey_no_ctx.serialize(), pubkey_with_ctx.serialize());

    let result_no_ctx = pubkey_no_ctx.tweak_add_assign(&sk2);
    let result_with_ctx = pubkey_with_ctx.tweak_add_assign_with_context(&sk2, &context);

    assert!(result_no_ctx.is_ok());
    assert!(result_with_ctx.is_ok());

    let serialized_no_ctx = pubkey_no_ctx.serialize();
    let serialized_with_ctx = pubkey_with_ctx.serialize();
    assert_eq!(serialized_no_ctx, serialized_with_ctx);


    let message = Message::parse(&MESSAGE_BYTES);
    let verify_no_ctx = verify(&message, &sign(&message, &sk1).0, &pubkey_no_ctx);
    let verify_with_ctx = verify(&message, &sign(&message, &sk1).0, &pubkey_with_ctx);

    assert_eq!(verify_no_ctx, verify_with_ctx);
    assert!(!verify_no_ctx);
}

#[test]
fn test_tweak_mul_assign_basic() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();

    let original_pubkey = PublicKey::from_secret_key(&sk1);
    let mut tweaked_pubkey = PublicKey::from_secret_key(&sk1);

    let original_serialized = original_pubkey.serialize();
    assert_eq!(original_serialized, tweaked_pubkey.serialize());

    let result = tweaked_pubkey.tweak_mul_assign(&sk2);
    assert!(result.is_ok());

    let post_tweak_serialized = tweaked_pubkey.serialize();
    assert_ne!(original_serialized, post_tweak_serialized);


    let mut add_tweaked = PublicKey::from_secret_key(&sk1);
    add_tweaked.tweak_add_assign(&sk2).unwrap();
    let add_tweaked_serialized = add_tweaked.serialize();
    assert_ne!(post_tweak_serialized, add_tweaked_serialized);


    let compressed = tweaked_pubkey.serialize_compressed();
    assert_eq!(compressed.len(), 33);
    assert!(compressed[0] == 0x02 || compressed[0] == 0x03);
}

#[test]
fn test_tweak_mul_assign_with_context_matches_without_context() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();

    let context = ECMultContext::new_boxed();

    let mut pubkey_no_ctx = PublicKey::from_secret_key(&sk1);
    let mut pubkey_with_ctx = PublicKey::from_secret_key(&sk1);

    assert_eq!(pubkey_no_ctx.serialize(), pubkey_with_ctx.serialize());

    let result_no_ctx = pubkey_no_ctx.tweak_mul_assign(&sk2);
    let result_with_ctx = pubkey_with_ctx.tweak_mul_assign_with_context(&sk2, &context);

    assert!(result_no_ctx.is_ok());
    assert!(result_with_ctx.is_ok());

    let serialized_no_ctx = pubkey_no_ctx.serialize();
    let serialized_with_ctx = pubkey_with_ctx.serialize();
    assert_eq!(serialized_no_ctx, serialized_with_ctx);


    let compressed_no_ctx = pubkey_no_ctx.serialize_compressed();
    let compressed_with_ctx = pubkey_with_ctx.serialize_compressed();
    assert_eq!(compressed_no_ctx, compressed_with_ctx);
}

#[test]
fn test_tweak_mul_assign_deterministic() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk3 = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let mut pubkey_a = PublicKey::from_secret_key(&sk1);
    let mut pubkey_b = PublicKey::from_secret_key(&sk1);

    pubkey_a.tweak_mul_assign(&sk3).unwrap();
    pubkey_b.tweak_mul_assign(&sk3).unwrap();

    let serialized_a = pubkey_a.serialize();
    let serialized_b = pubkey_b.serialize();
    assert_eq!(serialized_a, serialized_b);


    let one_bytes: [u8; 32] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    ];
    let one_sk = SecretKey::parse(&one_bytes).unwrap();
    let mut pubkey_identity = PublicKey::from_secret_key(&sk1);
    let original_serialized = pubkey_identity.serialize();
    pubkey_identity.tweak_mul_assign(&one_sk).unwrap();
    let after_identity_mul = pubkey_identity.serialize();
    assert_eq!(original_serialized, after_identity_mul);
}

#[test]
fn test_tweak_add_assign_commutativity_of_tweaks() {


    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let sk3 = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let mut pubkey_ab = PublicKey::from_secret_key(&sk1);
    pubkey_ab.tweak_add_assign(&sk2).unwrap();
    pubkey_ab.tweak_add_assign(&sk3).unwrap();

    let mut pubkey_ba = PublicKey::from_secret_key(&sk1);
    pubkey_ba.tweak_add_assign(&sk3).unwrap();
    pubkey_ba.tweak_add_assign(&sk2).unwrap();

    let serialized_ab = pubkey_ab.serialize();
    let serialized_ba = pubkey_ba.serialize();
    assert_eq!(serialized_ab, serialized_ba);


    let mut pubkey_intermediate_a = PublicKey::from_secret_key(&sk1);
    pubkey_intermediate_a.tweak_add_assign(&sk2).unwrap();
    let intermediate_a = pubkey_intermediate_a.serialize();

    let mut pubkey_intermediate_b = PublicKey::from_secret_key(&sk1);
    pubkey_intermediate_b.tweak_add_assign(&sk3).unwrap();
    let intermediate_b = pubkey_intermediate_b.serialize();

    assert_ne!(intermediate_a, intermediate_b);
    assert_ne!(intermediate_a, serialized_ab);
    assert_ne!(intermediate_b, serialized_ab);
}

#[test]
fn test_tweak_add_then_sign_and_recover() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();

    let mut pubkey = PublicKey::from_secret_key(&sk1);
    let original_pubkey = PublicKey::from_secret_key(&sk1);

    pubkey.tweak_add_assign(&sk2).unwrap();



    let message = Message::parse(&MESSAGE_BYTES);
    let (sig, recid) = sign(&message, &sk1);

    assert!(verify(&message, &sig, &original_pubkey));
    assert!(!verify(&message, &sig, &pubkey));


    let recovered = recover(&message, &sig, &recid).unwrap();
    assert_eq!(recovered.serialize(), original_pubkey.serialize());
    assert_ne!(recovered.serialize(), pubkey.serialize());
}

#[test]
fn test_tweak_mul_then_verify_with_context() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk3 = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let context = ECMultContext::new_boxed();

    let original_pubkey = PublicKey::from_secret_key(&sk1);
    let mut tweaked_pubkey = PublicKey::from_secret_key(&sk1);
    tweaked_pubkey.tweak_mul_assign_with_context(&sk3, &context).unwrap();

    let message = Message::parse(&MESSAGE_BYTES);
    let (sig, _recid) = sign(&message, &sk1);


    let verify_original = verify_with_context(&message, &sig, &original_pubkey, &context);
    assert!(verify_original);


    let verify_tweaked = verify_with_context(&message, &sig, &tweaked_pubkey, &context);
    assert!(!verify_tweaked);


    assert_ne!(original_pubkey.serialize(), tweaked_pubkey.serialize());
    assert_ne!(original_pubkey.serialize_compressed(), tweaked_pubkey.serialize_compressed());
}

#[test]
fn test_tweak_mul_assign_associativity() {

    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let sk3 = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let mut pubkey_ab = PublicKey::from_secret_key(&sk1);
    pubkey_ab.tweak_mul_assign(&sk2).unwrap();
    pubkey_ab.tweak_mul_assign(&sk3).unwrap();

    let mut pubkey_ba = PublicKey::from_secret_key(&sk1);
    pubkey_ba.tweak_mul_assign(&sk3).unwrap();
    pubkey_ba.tweak_mul_assign(&sk2).unwrap();

    let serialized_ab = pubkey_ab.serialize();
    let serialized_ba = pubkey_ba.serialize();
    assert_eq!(serialized_ab, serialized_ba);


    let mut intermediate_after_2 = PublicKey::from_secret_key(&sk1);
    intermediate_after_2.tweak_mul_assign(&sk2).unwrap();

    let mut intermediate_after_3 = PublicKey::from_secret_key(&sk1);
    intermediate_after_3.tweak_mul_assign(&sk3).unwrap();

    assert_ne!(intermediate_after_2.serialize(), intermediate_after_3.serialize());
    assert_ne!(intermediate_after_2.serialize(), serialized_ab);
    assert_ne!(intermediate_after_3.serialize(), serialized_ab);
}

#[test]
fn test_tweak_add_and_mul_combined_workflow() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let sk3 = SecretKey::parse(&SECRET_KEY_3).unwrap();

    let context = ECMultContext::new_boxed();


    let mut pubkey = PublicKey::from_secret_key(&sk1);
    let original = pubkey.serialize();

    pubkey.tweak_add_assign_with_context(&sk2, &context).unwrap();
    let after_add = pubkey.serialize();
    assert_ne!(original, after_add);

    pubkey.tweak_mul_assign_with_context(&sk3, &context).unwrap();
    let after_mul = pubkey.serialize();
    assert_ne!(after_add, after_mul);
    assert_ne!(original, after_mul);


    let mut pubkey_reverse = PublicKey::from_secret_key(&sk1);
    pubkey_reverse.tweak_mul_assign_with_context(&sk3, &context).unwrap();
    pubkey_reverse.tweak_add_assign_with_context(&sk2, &context).unwrap();
    let reverse_result = pubkey_reverse.serialize();

    assert_ne!(after_mul, reverse_result);


    let compressed_1 = pubkey.serialize_compressed();
    let compressed_2 = pubkey_reverse.serialize_compressed();
    assert!(compressed_1[0] == 0x02 || compressed_1[0] == 0x03);
    assert!(compressed_2[0] == 0x02 || compressed_2[0] == 0x03);
    assert_eq!(compressed_1.len(), 33);
    assert_eq!(compressed_2.len(), 33);
}

#[test]
fn test_tweak_add_assign_with_different_base_keys() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let sk2 = SecretKey::parse(&SECRET_KEY_2).unwrap();
    let sk3 = SecretKey::parse(&SECRET_KEY_3).unwrap();


    let mut pubkey_from_1 = PublicKey::from_secret_key(&sk1);
    let mut pubkey_from_2 = PublicKey::from_secret_key(&sk2);

    pubkey_from_1.tweak_add_assign(&sk3).unwrap();
    pubkey_from_2.tweak_add_assign(&sk3).unwrap();

    let serialized_1 = pubkey_from_1.serialize();
    let serialized_2 = pubkey_from_2.serialize();
    assert_ne!(serialized_1, serialized_2);


    let mut pubkey_mul_1 = PublicKey::from_secret_key(&sk1);
    let mut pubkey_mul_2 = PublicKey::from_secret_key(&sk2);

    pubkey_mul_1.tweak_mul_assign(&sk3).unwrap();
    pubkey_mul_2.tweak_mul_assign(&sk3).unwrap();

    let serialized_mul_1 = pubkey_mul_1.serialize();
    let serialized_mul_2 = pubkey_mul_2.serialize();
    assert_ne!(serialized_mul_1, serialized_mul_2);


    assert_ne!(serialized_1, serialized_mul_1);
    assert_ne!(serialized_2, serialized_mul_2);
}

#[test]
fn test_tweak_mul_assign_with_context_explicit_context() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();
    let tweak_bytes: [u8; 32] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
    ];
    let tweak = SecretKey::parse(&tweak_bytes).unwrap();

    let context = ECMultContext::new_boxed();

    let mut pubkey = PublicKey::from_secret_key(&sk1);
    let before = pubkey.serialize();

    let result = pubkey.tweak_mul_assign_with_context(&tweak, &context);
    assert!(result.is_ok());

    let after = pubkey.serialize();
    assert_ne!(before, after);


    let mut pubkey_triple = PublicKey::from_secret_key(&sk1);
    pubkey_triple.tweak_mul_assign_with_context(&tweak, &context).unwrap();
    pubkey_triple.tweak_mul_assign_with_context(&tweak, &context).unwrap();
    pubkey_triple.tweak_mul_assign_with_context(&tweak, &context).unwrap();

    let triple_serialized = pubkey_triple.serialize();
    assert_ne!(after, triple_serialized);
    assert_ne!(before, triple_serialized);


    let compressed = pubkey_triple.serialize_compressed();
    assert!(compressed[0] == 0x02 || compressed[0] == 0x03);
    assert_eq!(compressed.len(), 33);
}

#[test]
fn test_tweak_add_assign_preserves_point_validity_across_multiple_tweaks() {
    let sk1 = SecretKey::parse(&SECRET_KEY_1).unwrap();

    let tweak_a_bytes: [u8; 32] = [
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
        0x90, 0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0, 0x01,
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
        0x90, 0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0, 0x01,
    ];
    let tweak_b_bytes: [u8; 32] = [
        0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, 0x80,
        0x70, 0x60, 0x50, 0x40, 0x30, 0x20, 0x10, 0x01,
        0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, 0x80,
        0x70, 0x60, 0x50, 0x40, 0x30, 0x20, 0x10, 0x01,
    ];

    let tweak_a = SecretKey::parse(&tweak_a_bytes).unwrap();
    let tweak_b = SecretKey::parse(&tweak_b_bytes).unwrap();

    let context = ECMultContext::new_boxed();
    let gen_context = ECMultGenContext::new_boxed();

    let mut pubkey = PublicKey::from_secret_key_with_context(&sk1, &gen_context);
    let states: Vec<[u8; 65]> = (0..4).map(|i| {
        if i > 0 {
            if i % 2 == 1 {
                pubkey.tweak_add_assign_with_context(&tweak_a, &context).unwrap();
            } else {
                pubkey.tweak_add_assign_with_context(&tweak_b, &context).unwrap();
            }
        }
        pubkey.serialize()
    }).collect();


    assert_ne!(states[0], states[1]);
    assert_ne!(states[1], states[2]);
    assert_ne!(states[2], states[3]);
    assert_ne!(states[0], states[2]);
    assert_ne!(states[0], states[3]);
    assert_ne!(states[1], states[3]);


    let final_compressed = pubkey.serialize_compressed();
    assert!(final_compressed[0] == 0x02 || final_compressed[0] == 0x03);
    assert_eq!(final_compressed.len(), 33);
}