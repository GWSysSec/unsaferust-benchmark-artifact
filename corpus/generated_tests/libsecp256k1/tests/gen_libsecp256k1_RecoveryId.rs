use libsecp256k1::*;

#[test]
fn test_recovery_id_parse_rpc_valid_values() {
    // RPC recovery IDs are typically 27 and 28 (corresponding to 0 and 1 internally)
    let rid_27 = RecoveryId::parse_rpc(27);
    assert!(rid_27.is_ok(), "parse_rpc(27) should succeed");

    let rid_28 = RecoveryId::parse_rpc(28);
    assert!(rid_28.is_ok(), "parse_rpc(28) should succeed");

    // Serialize both to check their internal value
    let rid_27_val = rid_27.unwrap();
    let rid_28_val = rid_28.unwrap();

    // The serialized value of RPC 27 should correspond to standard recovery id 0
    let rid_0 = RecoveryId::parse(0).unwrap();
    let rid_1 = RecoveryId::parse(1).unwrap();

    assert_eq!(rid_27_val.serialize(), rid_0.serialize(), "RPC 27 should map to recovery id 0");
    assert_eq!(rid_28_val.serialize(), rid_1.serialize(), "RPC 28 should map to recovery id 1");

    assert_eq!(rid_27_val.serialize(), 0u8);
    assert_eq!(rid_28_val.serialize(), 1u8);
}

#[test]
fn test_recovery_id_parse_rpc_invalid_values() {
    // Values outside the valid RPC range should fail
    let rid_0 = RecoveryId::parse_rpc(0);
    assert!(rid_0.is_err(), "parse_rpc(0) should fail");

    let rid_1 = RecoveryId::parse_rpc(1);
    assert!(rid_1.is_err(), "parse_rpc(1) should fail");

    let rid_26 = RecoveryId::parse_rpc(26);
    assert!(rid_26.is_err(), "parse_rpc(26) should fail");

    // parse_rpc accepts values 27..=30 (mapping to recovery ids 0..=3)
    // So 30 is actually valid (maps to recovery id 3)
    let rid_30 = RecoveryId::parse_rpc(30);
    // 30 maps to recovery id 3, which is valid in libsecp256k1
    let _ = rid_30;

    let rid_255 = RecoveryId::parse_rpc(255);
    assert!(rid_255.is_err(), "parse_rpc(255) should fail");

    let rid_100 = RecoveryId::parse_rpc(100);
    assert!(rid_100.is_err(), "parse_rpc(100) should fail");

    let rid_31 = RecoveryId::parse_rpc(31);
    assert!(rid_31.is_err(), "parse_rpc(31) should fail");

    let rid_29 = RecoveryId::parse_rpc(29);
    // 29 maps to recovery id 2, which may or may not be valid depending on implementation
    // We just check it doesn't panic
    let _ = rid_29;

    // Verify error type is cloneable (exercises Error::clone from API)
    if let Err(e) = rid_0 {
        let e2 = e.clone();
        assert_eq!(format!("{:?}", e), format!("{:?}", e2));
    }
}

#[test]
fn test_recovery_id_parse_rpc_sign_recover_roundtrip() {
    // Create a secret key
    let seckey_bytes: [u8; 32] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
    ];
    let seckey = SecretKey::parse(&seckey_bytes).unwrap();
    let pubkey = PublicKey::from_secret_key(&seckey);

    // Create a message
    let msg_bytes: [u8; 32] = [0xab; 32];
    let message = Message::parse(&msg_bytes);

    // Sign the message
    let (signature, recovery_id) = sign(&message, &seckey);

    // Verify the signature
    assert!(verify(&message, &signature, &pubkey));

    // Get the standard recovery id value
    let rid_standard = recovery_id.serialize();
    assert!(rid_standard < 4, "Recovery ID should be 0-3");

    // Convert to RPC format (add 27)
    let rpc_value = rid_standard + 27;

    // Parse back using parse_rpc
    let parsed_rpc = RecoveryId::parse_rpc(rpc_value).unwrap();
    assert_eq!(parsed_rpc.serialize(), rid_standard, "Round-trip through RPC format should preserve value");

    // Recover the public key using the RPC-parsed recovery id
    let recovered = recover(&message, &signature, &parsed_rpc).unwrap();
    assert_eq!(recovered, pubkey, "Recovered public key should match original");

    // Also verify with the original recovery id
    let recovered_original = recover(&message, &signature, &recovery_id).unwrap();
    assert_eq!(recovered_original, pubkey);
    assert_eq!(recovered, recovered_original);
}

#[test]
fn test_recovery_id_parse_rpc_all_boundary_values() {
    // Test all values around the expected valid range
    let results: Vec<(u8, bool)> = (0u8..=35)
        .map(|v| (v, RecoveryId::parse_rpc(v).is_ok()))
        .collect();

    // Values below 27 should all fail
    for &(v, ok) in results.iter().filter(|(v, _)| *v < 27) {
        assert!(!ok, "parse_rpc({}) should fail", v);
    }

    // 27 and 28 should succeed (standard Ethereum-style RPC recovery IDs)
    assert!(results[27].1, "parse_rpc(27) should succeed");
    assert!(results[28].1, "parse_rpc(28) should succeed");

    // Values above 30 should fail (parse_rpc accepts 27..=30 mapping to 0..=3)
    for &(v, ok) in results.iter().filter(|(v, _)| *v > 30) {
        assert!(!ok, "parse_rpc({}) should fail", v);
    }

    // Verify the parsed values are usable in recovery
    let seckey_bytes: [u8; 32] = [0x42; 32];
    let seckey = SecretKey::parse(&seckey_bytes).unwrap();
    let pubkey = PublicKey::from_secret_key(&seckey);
    let msg = Message::parse(&[0x99; 32]);
    let (sig, rid) = sign(&msg, &seckey);

    let rpc_rid = RecoveryId::parse_rpc(rid.serialize() + 27).unwrap();
    let recovered_pk = recover(&msg, &sig, &rpc_rid).unwrap();
    assert_eq!(recovered_pk, pubkey);
}

#[test]
fn test_recovery_id_parse_rpc_multiple_messages() {
    let seckey = SecretKey::parse(&[0x77; 32]).unwrap();
    let pubkey = PublicKey::from_secret_key(&seckey);

    // Sign multiple different messages and verify RPC recovery id roundtrip for each
    let messages: Vec<[u8; 32]> = vec![
        [0x01; 32],
        [0xff; 32],
        [0x00; 32],
        [0x80; 32],
    ];

    for (i, msg_bytes) in messages.iter().enumerate() {
        let message = Message::parse(msg_bytes);
        let (signature, recovery_id) = sign(&message, &seckey);

        // Verify signature is valid
        assert!(verify(&message, &signature, &pubkey), "Signature {} should verify", i);

        // Convert to RPC and back
        let rpc_val = recovery_id.serialize() + 27;
        let rpc_rid = RecoveryId::parse_rpc(rpc_val).unwrap();
        assert_eq!(rpc_rid.serialize(), recovery_id.serialize(),
            "Message {} RPC roundtrip failed", i);

        // Recover with RPC-parsed recovery id
        let recovered = recover(&message, &signature, &rpc_rid).unwrap();
        assert_eq!(recovered, pubkey, "Message {} recovery failed", i);
    }

    // Verify wrong RPC values still fail
    assert!(RecoveryId::parse_rpc(0).is_err());
    assert!(RecoveryId::parse_rpc(26).is_err());
}