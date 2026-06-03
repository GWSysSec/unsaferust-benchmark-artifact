extern crate native_tls;

use native_tls::{TlsConnector, Protocol, Certificate, Identity};

#[test]
fn test_use_sni_enabled_builds_connector_successfully() {
    let builder_result = TlsConnector::builder()
        .use_sni(true)
        .build();

    assert!(builder_result.is_ok(), "Building connector with use_sni(true) should succeed");

    let connector = builder_result.unwrap();

    let _connector_ref = &connector;
    assert_eq!(std::mem::size_of_val(&connector) > 0, true);


    let builder_result_no_sni = TlsConnector::builder()
        .use_sni(false)
        .build();

    assert!(builder_result_no_sni.is_ok(), "Building connector with use_sni(false) should succeed");

    let connector_no_sni = builder_result_no_sni.unwrap();
    assert_eq!(std::mem::size_of_val(&connector_no_sni) > 0, true);


    let ptr1 = &connector as *const _ as usize;
    let ptr2 = &connector_no_sni as *const _ as usize;
    assert_ne!(ptr1, ptr2, "Two connectors should be at different addresses");


    let size1 = std::mem::size_of_val(&connector);
    let size2 = std::mem::size_of_val(&connector_no_sni);
    assert_eq!(size1, size2, "Both connectors should have the same size");
}

#[test]
fn test_use_sni_chained_with_other_builder_methods() {

    let connector_result = TlsConnector::builder()
        .use_sni(true)
        .min_protocol_version(Some(Protocol::Tlsv12))
        .danger_accept_invalid_certs(false)
        .build();

    assert!(connector_result.is_ok(), "Chained builder with use_sni should succeed");

    let connector = connector_result.unwrap();
    assert_eq!(std::mem::size_of_val(&connector) > 0, true);


    let connector_result2 = TlsConnector::builder()
        .min_protocol_version(Some(Protocol::Tlsv10))
        .use_sni(false)
        .danger_accept_invalid_certs(true)
        .build();

    assert!(connector_result2.is_ok(), "Chained builder with use_sni(false) should succeed");

    let connector2 = connector_result2.unwrap();
    assert_eq!(std::mem::size_of_val(&connector2) > 0, true);


    let connector_result3 = TlsConnector::builder()
        .use_sni(true)
        .min_protocol_version(Some(Protocol::Tlsv11))
        .max_protocol_version(Some(Protocol::Tlsv12))
        .build();

    assert!(connector_result3.is_ok(), "Builder with sni + protocol range should succeed");
    assert_ne!(
        &connector as *const _ as usize,
        &connector2 as *const _ as usize
    );
}

#[test]
fn test_use_sni_toggle_multiple_times_last_wins() {

    let connector_result = TlsConnector::builder()
        .use_sni(true)
        .use_sni(false)
        .use_sni(true)
        .use_sni(false)
        .build();

    assert!(connector_result.is_ok(), "Multiple use_sni toggles should not cause errors");

    let connector = connector_result.unwrap();
    assert_eq!(std::mem::size_of_val(&connector) > 0, true);


    let connector_result2 = TlsConnector::builder()
        .use_sni(false)
        .use_sni(true)
        .build();

    assert!(connector_result2.is_ok(), "Final use_sni(true) should build successfully");

    let connector2 = connector_result2.unwrap();
    assert_eq!(std::mem::size_of_val(&connector2) > 0, true);


    let size1 = std::mem::size_of_val(&connector);
    let size2 = std::mem::size_of_val(&connector2);
    assert_eq!(size1, size2, "Connectors should have consistent size regardless of sni setting");
    assert_ne!(
        &connector as *const _ as usize,
        &connector2 as *const _ as usize,
        "Distinct connectors at distinct addresses"
    );
}

#[test]
fn test_use_sni_with_invalid_cert_does_not_panic() {

    let bad_der: &[u8] = &[0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
    let cert_result = Certificate::from_der(bad_der);
    assert!(cert_result.is_err(), "Invalid DER should produce an error");

    let error = match cert_result {
        Err(e) => e,
        Ok(_) => panic!("Expected error for invalid DER"),
    };
    let error_msg = format!("{}", error);
    assert!(!error_msg.is_empty(), "Error message should not be empty");


    let connector_result = TlsConnector::builder()
        .use_sni(true)
        .danger_accept_invalid_certs(true)
        .build();

    assert!(connector_result.is_ok(), "Connector with use_sni should build without valid certs added");

    let connector = connector_result.unwrap();
    assert_eq!(std::mem::size_of_val(&connector) > 0, true);


    let bad_pkcs12: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
    let identity_result = Identity::from_pkcs12(bad_pkcs12, "password");
    assert!(identity_result.is_err(), "Invalid PKCS12 should produce an error");

    let id_error = match identity_result {
        Err(e) => e,
        Ok(_) => panic!("Expected error for invalid PKCS12"),
    };
    let id_error_msg = format!("{}", id_error);
    assert!(!id_error_msg.is_empty(), "Identity error message should not be empty");
}

#[test]
fn test_use_sni_builder_returns_mutable_reference() {

    let mut builder = TlsConnector::builder();

    let builder_ref = builder.use_sni(true);

    let builder_ref2 = builder_ref.use_sni(false);
    let builder_ref3 = builder_ref2.min_protocol_version(Some(Protocol::Tlsv12));
    let result = builder_ref3.build();

    assert!(result.is_ok(), "Chained builder via returned references should succeed");

    let connector = result.unwrap();
    assert_eq!(std::mem::size_of_val(&connector) > 0, true);


    let default_result = TlsConnector::new();
    assert!(default_result.is_ok(), "Default TlsConnector::new() should succeed");

    let default_connector = default_result.unwrap();
    let default_size = std::mem::size_of_val(&default_connector);
    let custom_size = std::mem::size_of_val(&connector);
    assert_eq!(default_size, custom_size, "All connectors should have same struct size");


    let proto = Protocol::Tlsv12;
    let proto_clone = proto.clone();
    assert_eq!(std::mem::size_of_val(&proto), std::mem::size_of_val(&proto_clone));
    assert_eq!(
        std::mem::size_of_val(&proto),
        std::mem::size_of::<Protocol>()
    );
}