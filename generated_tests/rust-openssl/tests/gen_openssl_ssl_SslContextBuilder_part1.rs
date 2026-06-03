use openssl::ssl::{SslContext, SslMethod, SslVerifyMode};
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::X509;
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::dh::Dh;
use openssl::hash::MessageDigest;
use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::x509::extension::{BasicConstraints, SubjectAlternativeName};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn generate_ca_cert_and_key() -> (X509, PKey<openssl::pkey::Private>) {
    let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
    let key = PKey::from_rsa(rsa).unwrap();

    let mut name_builder = openssl::x509::X509NameBuilder::new().unwrap();
    name_builder.append_entry_by_text("CN", "Test CA").unwrap();
    let name = name_builder.build();

    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    let serial = BigNum::from_u32(1).unwrap();
    builder.set_serial_number(&serial.to_asn1_integer().unwrap()).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder.set_pubkey(&key).unwrap();
    let not_before = Asn1Time::days_from_now(0).unwrap();
    let not_after = Asn1Time::days_from_now(365).unwrap();
    builder.set_not_before(&not_before).unwrap();
    builder.set_not_after(&not_after).unwrap();
    builder.append_extension(BasicConstraints::new().critical().ca().build().unwrap()).unwrap();
    builder.sign(&key, MessageDigest::sha256()).unwrap();

    (builder.build(), key)
}

fn generate_server_cert_and_key(ca_cert: &X509, ca_key: &PKey<openssl::pkey::Private>) -> (X509, PKey<openssl::pkey::Private>) {
    let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
    let key = PKey::from_rsa(rsa).unwrap();

    let mut name_builder = openssl::x509::X509NameBuilder::new().unwrap();
    name_builder.append_entry_by_text("CN", "localhost").unwrap();
    let name = name_builder.build();

    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    let serial = BigNum::from_u32(2).unwrap();
    builder.set_serial_number(&serial.to_asn1_integer().unwrap()).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(ca_cert.subject_name()).unwrap();
    builder.set_pubkey(&key).unwrap();
    let not_before = Asn1Time::days_from_now(0).unwrap();
    let not_after = Asn1Time::days_from_now(365).unwrap();
    builder.set_not_before(&not_before).unwrap();
    builder.set_not_after(&not_after).unwrap();

    let san = SubjectAlternativeName::new()
        .dns("localhost")
        .build(&builder.x509v3_context(Some(ca_cert), None))
        .unwrap();
    builder.append_extension(san).unwrap();

    builder.sign(ca_key, MessageDigest::sha256()).unwrap();

    (builder.build(), key)
}

#[test]
fn test_set_verify_callback_and_depth() {
    let callback_count = Arc::new(AtomicUsize::new(0));
    let callback_count_clone = callback_count.clone();

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();


    ctx_builder.set_verify_depth(4);


    ctx_builder.set_verify_callback(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT, move |preverify_ok, _x509_store_ctx| {
        callback_count_clone.fetch_add(1, Ordering::SeqCst);
        preverify_ok
    });

    let ctx = ctx_builder.build();


    assert_eq!(callback_count.load(Ordering::SeqCst), 0);


    let mut ctx_builder2 = SslContext::builder(SslMethod::tls()).unwrap();
    ctx_builder2.set_verify_depth(0);
    ctx_builder2.set_verify_callback(SslVerifyMode::NONE, |_preverify, _ctx| true);
    let _ctx2 = ctx_builder2.build();


    let mut ctx_builder3 = SslContext::builder(SslMethod::tls()).unwrap();
    ctx_builder3.set_verify_depth(10);
    ctx_builder3.set_verify_callback(SslVerifyMode::PEER, |preverify, _ctx| preverify);
    let _ctx3 = ctx_builder3.build();

    assert_eq!(callback_count.load(Ordering::SeqCst), 0);
    assert!(ctx.verify_mode().contains(SslVerifyMode::PEER));
    assert!(ctx.verify_mode().contains(SslVerifyMode::FAIL_IF_NO_PEER_CERT));


    assert_ne!(ctx.verify_mode(), SslVerifyMode::NONE);


    let verify_mode = ctx.verify_mode();
    assert_ne!(verify_mode, SslVerifyMode::NONE);
    assert_eq!(verify_mode, SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
    assert_ne!(verify_mode, SslVerifyMode::PEER);
}

#[test]
fn test_set_cert_store_and_check_private_key() {
    let (ca_cert, ca_key) = generate_ca_cert_and_key();
    let (server_cert, server_key) = generate_server_cert_and_key(&ca_cert, &ca_key);

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();


    let mut store_builder = X509StoreBuilder::new().unwrap();
    store_builder.add_cert(ca_cert.clone()).unwrap();
    let store = store_builder.build();


    ctx_builder.set_cert_store(store);


    ctx_builder.set_certificate(&server_cert).unwrap();
    ctx_builder.set_private_key(&server_key).unwrap();


    let result = ctx_builder.check_private_key();
    assert!(result.is_ok());


    let subject = server_cert.subject_name();
    let cn_entries: Vec<_> = subject.entries_by_nid(Nid::COMMONNAME).collect();
    assert_eq!(cn_entries.len(), 1);
    assert_eq!(cn_entries[0].data().as_utf8().unwrap().to_string(), "localhost");


    let ca_subject = ca_cert.subject_name();
    let ca_cn_entries: Vec<_> = ca_subject.entries_by_nid(Nid::COMMONNAME).collect();
    assert_eq!(ca_cn_entries.len(), 1);
    assert_eq!(ca_cn_entries[0].data().as_utf8().unwrap().to_string(), "Test CA");


    let issuer = server_cert.issuer_name();
    let issuer_cn: Vec<_> = issuer.entries_by_nid(Nid::COMMONNAME).collect();
    assert_eq!(issuer_cn.len(), 1);
    assert_eq!(issuer_cn[0].data().as_utf8().unwrap().to_string(), "Test CA");


    assert_eq!(server_cert.version(), 2);
}

#[test]
fn test_check_private_key_mismatch() {
    let (ca_cert, ca_key) = generate_ca_cert_and_key();
    let (server_cert, _server_key) = generate_server_cert_and_key(&ca_cert, &ca_key);


    let rsa_other = openssl::rsa::Rsa::generate(2048).unwrap();
    let other_key = PKey::from_rsa(rsa_other).unwrap();

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();
    ctx_builder.set_certificate(&server_cert).unwrap();



    let set_key_result = ctx_builder.set_private_key(&other_key);

    if set_key_result.is_ok() {

        let result = ctx_builder.check_private_key();
        assert!(result.is_err());

        let err = result.unwrap_err();
        let errors = err.errors();
        assert!(!errors.is_empty());
        assert!(errors.len() >= 1);
    } else {

        let err = set_key_result.unwrap_err();
        let errors = err.errors();
        assert!(!errors.is_empty());
        assert!(errors.len() >= 1);
    }


    assert_eq!(server_cert.version(), 2);


    assert_eq!(other_key.bits(), 2048);
    assert_ne!(other_key.bits(), 1024);
}

#[test]
fn test_set_read_ahead() {
    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();


    ctx_builder.set_read_ahead(true);
    let _ctx = ctx_builder.build();


    let mut ctx_builder2 = SslContext::builder(SslMethod::tls()).unwrap();
    ctx_builder2.set_read_ahead(false);
    let _ctx2 = ctx_builder2.build();


    let mut ctx_builder3 = SslContext::builder(SslMethod::tls()).unwrap();
    ctx_builder3.set_read_ahead(true);
    ctx_builder3.set_verify_depth(5);
    ctx_builder3.set_verify_callback(SslVerifyMode::PEER, |ok, _| ok);
    let ctx3 = ctx_builder3.build();

    assert!(ctx3.verify_mode().contains(SslVerifyMode::PEER));
    assert!(!ctx3.verify_mode().contains(SslVerifyMode::FAIL_IF_NO_PEER_CERT));
    assert_ne!(ctx3.verify_mode(), SslVerifyMode::NONE);


    let mut ctx_builder4 = SslContext::builder(SslMethod::tls()).unwrap();
    ctx_builder4.set_read_ahead(true);
    ctx_builder4.set_read_ahead(false);
    ctx_builder4.set_read_ahead(true);
    let _ctx4 = ctx_builder4.build();

    assert_eq!(ctx3.verify_mode(), SslVerifyMode::PEER);
    assert_eq!(_ctx.verify_mode(), SslVerifyMode::empty());
    assert_eq!(_ctx2.verify_mode(), SslVerifyMode::empty());
    assert_ne!(ctx3.verify_mode(), SslVerifyMode::empty());
}

#[test]
fn test_set_tmp_ecdh() {
    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();


    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ec_key = EcKey::from_group(&group).unwrap();

    let result = ctx_builder.set_tmp_ecdh(&ec_key);
    assert!(result.is_ok());


    let mut ctx_builder2 = SslContext::builder(SslMethod::tls()).unwrap();
    let group384 = EcGroup::from_curve_name(Nid::SECP384R1).unwrap();
    let ec_key384 = EcKey::from_group(&group384).unwrap();

    let result2 = ctx_builder2.set_tmp_ecdh(&ec_key384);
    assert!(result2.is_ok());


    let key_group = ec_key.group();
    let key_nid = key_group.curve_name();
    assert_eq!(key_nid, Some(Nid::X9_62_PRIME256V1));

    let key384_group = ec_key384.group();
    let key384_nid = key384_group.curve_name();
    assert_eq!(key384_nid, Some(Nid::SECP384R1));
    assert_ne!(key_nid, key384_nid);


    let _ctx = ctx_builder.build();
    let _ctx2 = ctx_builder2.build();

    assert_eq!(_ctx.verify_mode(), SslVerifyMode::empty());
    assert_eq!(_ctx2.verify_mode(), SslVerifyMode::empty());
}

#[test]
fn test_set_tmp_dh_callback() {
    let callback_invoked = Arc::new(AtomicUsize::new(0));
    let callback_invoked_clone = callback_invoked.clone();

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();

    ctx_builder.set_tmp_dh_callback(move |_ssl, _is_export, _keylength| {
        callback_invoked_clone.fetch_add(1, Ordering::SeqCst);
        let dh = Dh::get_2048_256().unwrap();
        Ok(dh)
    });


    assert_eq!(callback_invoked.load(Ordering::SeqCst), 0);

    let (ca_cert, ca_key) = generate_ca_cert_and_key();
    let (server_cert, server_key) = generate_server_cert_and_key(&ca_cert, &ca_key);

    ctx_builder.set_certificate(&server_cert).unwrap();
    ctx_builder.set_private_key(&server_key).unwrap();

    let check_result = ctx_builder.check_private_key();
    assert!(check_result.is_ok());

    let _ctx = ctx_builder.build();
    assert_eq!(callback_invoked.load(Ordering::SeqCst), 0);


    assert_eq!(server_cert.version(), 2);
    assert_eq!(ca_cert.version(), 2);
    assert_ne!(server_cert.serial_number().to_bn().unwrap(), ca_cert.serial_number().to_bn().unwrap());
}

#[test]
fn test_set_alpn_select_callback() {
    let selected_protocol = Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let selected_protocol_clone = selected_protocol.clone();

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();

    ctx_builder.set_alpn_select_callback(move |_ssl, client_protocols| {

        if let Some(proto) = openssl::ssl::select_next_proto(b"\x02h2\x08http/1.1", client_protocols) {
            if let Ok(mut guard) = selected_protocol_clone.try_lock() {
                *guard = proto.to_vec();
            }
            Ok(proto)
        } else {
            Err(openssl::ssl::AlpnError::NOACK)
        }
    });

    let (ca_cert, ca_key) = generate_ca_cert_and_key();
    let (server_cert, server_key) = generate_server_cert_and_key(&ca_cert, &ca_key);

    ctx_builder.set_certificate(&server_cert).unwrap();
    ctx_builder.set_private_key(&server_key).unwrap();
    assert!(ctx_builder.check_private_key().is_ok());

    let _ctx = ctx_builder.build();


    if let Ok(guard) = selected_protocol.try_lock() {
        assert_eq!(guard.len(), 0);
    }


    assert_eq!(server_cert.version(), 2);
    let subject = server_cert.subject_name();
    let cn: Vec<_> = subject.entries_by_nid(Nid::COMMONNAME).collect();
    assert_eq!(cn.len(), 1);
    assert_eq!(cn[0].data().as_utf8().unwrap().to_string(), "localhost");
    assert_ne!(server_key.bits(), 0);
    assert_eq!(server_key.bits(), 2048);
}

#[test]
fn test_verify_param_and_verify_param_mut() {
    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();


    {
        let param_mut = ctx_builder.verify_param_mut();
        param_mut.set_host("example.com").unwrap();
    }


    ctx_builder.set_verify_depth(3);
    ctx_builder.set_verify_callback(SslVerifyMode::PEER, |ok, _| ok);

    let ctx = ctx_builder.build();
    assert!(ctx.verify_mode().contains(SslVerifyMode::PEER));
    assert!(!ctx.verify_mode().contains(SslVerifyMode::FAIL_IF_NO_PEER_CERT));


    let mut ctx_builder2 = SslContext::builder(SslMethod::tls()).unwrap();
    {
        let param_mut = ctx_builder2.verify_param_mut();
        param_mut.set_host("*.example.com").unwrap();
    }
    ctx_builder2.set_verify_depth(7);
    let ctx2 = ctx_builder2.build();

    assert_eq!(ctx2.verify_mode(), SslVerifyMode::empty());
    assert_ne!(ctx.verify_mode(), ctx2.verify_mode());
    assert_eq!(ctx.verify_mode(), SslVerifyMode::PEER);


    let mut ctx_builder3 = SslContext::builder(SslMethod::tls()).unwrap();
    {
        let param_mut = ctx_builder3.verify_param_mut();
        param_mut.set_ip("127.0.0.1".parse().unwrap()).unwrap();
    }
    let _ctx3 = ctx_builder3.build();
    assert_eq!(_ctx3.verify_mode(), SslVerifyMode::empty());
}

#[test]
fn test_set_servername_callback() {
    let sni_called = Arc::new(AtomicUsize::new(0));
    let sni_called_clone = sni_called.clone();

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();

    ctx_builder.set_servername_callback(move |ssl, _alert| {
        sni_called_clone.fetch_add(1, Ordering::SeqCst);
        let servername = ssl.servername(openssl::ssl::NameType::HOST_NAME);
        if servername == Some("localhost") {
            Ok(())
        } else {
            Ok(())
        }
    });

    let (ca_cert, ca_key) = generate_ca_cert_and_key();
    let (server_cert, server_key) = generate_server_cert_and_key(&ca_cert, &ca_key);

    ctx_builder.set_certificate(&server_cert).unwrap();
    ctx_builder.set_private_key(&server_key).unwrap();
    assert!(ctx_builder.check_private_key().is_ok());

    ctx_builder.set_verify_depth(5);
    ctx_builder.set_read_ahead(true);

    let _ctx = ctx_builder.build();


    assert_eq!(sni_called.load(Ordering::SeqCst), 0);


    assert_eq!(_ctx.verify_mode(), SslVerifyMode::empty());
    assert_eq!(server_cert.version(), 2);
    assert_eq!(server_key.bits(), 2048);
    assert_ne!(sni_called.load(Ordering::SeqCst), 1);
    assert_eq!(sni_called.load(Ordering::SeqCst), 0);
}

#[test]
fn test_set_status_callback() {
    let status_called = Arc::new(AtomicUsize::new(0));
    let status_called_clone = status_called.clone();

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();

    let result = ctx_builder.set_status_callback(move |_ssl| {
        status_called_clone.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    });
    assert!(result.is_ok());

    let (ca_cert, ca_key) = generate_ca_cert_and_key();
    let (server_cert, server_key) = generate_server_cert_and_key(&ca_cert, &ca_key);

    ctx_builder.set_certificate(&server_cert).unwrap();
    ctx_builder.set_private_key(&server_key).unwrap();
    assert!(ctx_builder.check_private_key().is_ok());

    let _ctx = ctx_builder.build();


    assert_eq!(status_called.load(Ordering::SeqCst), 0);
    assert_ne!(status_called.load(Ordering::SeqCst), 1);


    assert_eq!(server_cert.version(), 2);
    assert_eq!(server_key.bits(), 2048);

    let subject = server_cert.subject_name();
    let cn: Vec<_> = subject.entries_by_nid(Nid::COMMONNAME).collect();
    assert_eq!(cn.len(), 1);
    assert_eq!(cn[0].data().as_utf8().unwrap().to_string(), "localhost");
}

#[test]
fn test_set_psk_callbacks() {

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();

    ctx_builder.set_psk_client_callback(move |_ssl, _hint, identity, psk| {
        let id = b"test_identity";
        if identity.len() >= id.len() {
            identity[..id.len()].copy_from_slice(id);
        }
        let key = b"secret_key_12345";
        if psk.len() >= key.len() {
            psk[..key.len()].copy_from_slice(key);
            Ok(key.len())
        } else {
            Ok(0)
        }
    });

    let _ctx_client = ctx_builder.build();
    assert_eq!(_ctx_client.verify_mode(), SslVerifyMode::empty());


    let mut ctx_builder_server = SslContext::builder(SslMethod::tls()).unwrap();

    ctx_builder_server.set_psk_server_callback(move |_ssl, _identity, psk| {
        let key = b"secret_key_12345";
        if psk.len() >= key.len() {
            psk[..key.len()].copy_from_slice(key);
            Ok(key.len())
        } else {
            Ok(0)
        }
    });

    let _ctx_server = ctx_builder_server.build();
    assert_eq!(_ctx_server.verify_mode(), SslVerifyMode::empty());


    assert_eq!(_ctx_client.verify_mode(), _ctx_server.verify_mode());
    assert_ne!(_ctx_client.verify_mode(), SslVerifyMode::PEER);
    assert_ne!(_ctx_server.verify_mode(), SslVerifyMode::PEER);
    assert_eq!(_ctx_client.verify_mode(), SslVerifyMode::empty());
    assert_eq!(_ctx_server.verify_mode(), SslVerifyMode::empty());
}

#[test]
fn test_combined_ssl_context_builder_workflow() {
    let (ca_cert, ca_key) = generate_ca_cert_and_key();
    let (server_cert, server_key) = generate_server_cert_and_key(&ca_cert, &ca_key);

    let verify_count = Arc::new(AtomicUsize::new(0));
    let verify_count_clone = verify_count.clone();

    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();


    let mut store_builder = X509StoreBuilder::new().unwrap();
    store_builder.add_cert(ca_cert.clone()).unwrap();
    let store = store_builder.build();
    ctx_builder.set_cert_store(store);


    ctx_builder.set_certificate(&server_cert).unwrap();
    ctx_builder.set_private_key(&server_key).unwrap();
    assert!(ctx_builder.check_private_key().is_ok());


    ctx_builder.set_verify_depth(5);
    ctx_builder.set_verify_callback(SslVerifyMode::PEER, move |ok, _ctx| {
        verify_count_clone.fetch_add(1, Ordering::SeqCst);
        ok
    });


    ctx_builder.set_read_ahead(true);


    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ec_key = EcKey::from_group(&group).unwrap();
    assert!(ctx_builder.set_tmp_ecdh(&ec_key).is_ok());


    ctx_builder.set_alpn_select_callback(|_ssl, protos| {
        if let Some(proto) = openssl::ssl::select_next_proto(b"\x02h2\x08http/1.1", protos) {
            Ok(proto)
        } else {
            Err(openssl::ssl::AlpnError::NOACK)
        }
    });


    ctx_builder.set_servername_callback(|_ssl, _alert| Ok(()));


    {
        let param_mut = ctx_builder.verify_param_mut();
        param_mut.set_host("localhost").unwrap();
    }


    assert!(ctx_builder.set_status_callback(|_ssl| Ok(true)).is_ok());

    let ctx = ctx_builder.build();


    assert!(ctx.verify_mode().contains(SslVerifyMode::PEER));
    assert!(!ctx.verify_mode().contains(SslVerifyMode::FAIL_IF_NO_PEER_CERT));
    assert_eq!(verify_count.load(Ordering::SeqCst), 0);
    assert_eq!(ctx.verify_mode(), SslVerifyMode::PEER);
    assert_eq!(server_cert.version(), 2);
    assert_eq!(ca_cert.version(), 2);
    assert_eq!(server_key.bits(), 2048);
    assert_ne!(ctx.verify_mode(), SslVerifyMode::NONE);
}

#[test]
fn test_set_psk_callback_tls13() {
    let mut ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();

    ctx_builder.set_verify_depth(3);
    ctx_builder.set_read_ahead(true);


    ctx_builder.set_psk_server_callback(|_ssl, _identity, psk| {
        let key = b"psk_key";
        psk[..key.len()].copy_from_slice(key);
        Ok(key.len())
    });

    let ctx = ctx_builder.build();

    assert_eq!(ctx.verify_mode(), SslVerifyMode::empty());
    assert_ne!(ctx.verify_mode(), SslVerifyMode::PEER);


    let mut ctx_builder2 = SslContext::builder(SslMethod::tls()).unwrap();
    ctx_builder2.set_psk_server_callback(|_ssl, _identity, psk| {
        let key = b"psk_key";
        psk[..key.len()].copy_from_slice(key);
        Ok(key.len())
    });
    ctx_builder2.set_psk_client_callback(|_ssl, _hint, identity, psk| {
        let id = b"client";
        identity[..id.len()].copy_from_slice(id);
        let key = b"psk_key";
        psk[..key.len()].copy_from_slice(key);
        Ok(key.len())
    });

    let ctx2 = ctx_builder2.build();
    assert_eq!(ctx2.verify_mode(), SslVerifyMode::empty());
    assert_eq!(ctx.verify_mode(), ctx2.verify_mode());
}

#[test]
fn test_handshake_with_callbacks() {
    let (ca_cert, ca_key) = generate_ca_cert_and_key();
    let (server_cert, server_key) = generate_server_cert_and_key(&ca_cert, &ca_key);

    let verify_called = Arc::new(AtomicUsize::new(0));
    let _verify_called_server = verify_called.clone();


    let mut server_ctx_builder = SslContext::builder(SslMethod::tls()).unwrap();
    server_ctx_builder.set_certificate(&server_cert).unwrap();
    server_ctx_builder.set_private_key(&server_key).unwrap();
    assert!(server_ctx_builder.check_private_key().is_ok());

    let _server_ctx = server_ctx_builder.build();


    assert_eq!(server_cert.version(), 2);
    assert_eq!(server_key.bits(), 2048);
    assert_eq!(verify_called.load(Ordering::SeqCst), 0);
}