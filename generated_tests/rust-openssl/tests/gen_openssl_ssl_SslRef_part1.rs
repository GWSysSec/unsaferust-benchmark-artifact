use openssl::asn1::Asn1Time;
use openssl::bn::BigNum;
use openssl::dh::Dh;
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::ssl::{
    Ssl, SslConnector, SslContext, SslContextBuilder, SslMethod, SslVerifyMode, SslVersion,
};
use openssl::x509::{X509Builder, X509NameBuilder, X509};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn make_self_signed() -> (X509, PKey<openssl::pkey::Private>) {
    let rsa = Rsa::generate(2048).unwrap();
    let pkey = PKey::from_rsa(rsa).unwrap();

    let mut name_builder = X509NameBuilder::new().unwrap();
    name_builder.append_entry_by_text("CN", "localhost").unwrap();
    let name = name_builder.build();

    let mut builder = X509Builder::new().unwrap();
    builder.set_version(2).unwrap();
    let serial = BigNum::from_u32(1).unwrap().to_asn1_integer().unwrap();
    builder.set_serial_number(&serial).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder.set_pubkey(&pkey).unwrap();
    builder
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::days_from_now(1).unwrap())
        .unwrap();
    builder.sign(&pkey, MessageDigest::sha256()).unwrap();
    (builder.build(), pkey)
}

#[test]
fn test_ssl_ref_state_and_settings() {
    let ctx_builder = SslContextBuilder::new(SslMethod::tls()).unwrap();
    let ctx: SslContext = ctx_builder.build();

    let mut ssl = Ssl::new(&ctx).unwrap();


    let initial_mode = ssl.verify_mode();
    assert_eq!(initial_mode, SslVerifyMode::NONE);


    ssl.set_verify_callback(SslVerifyMode::PEER, |preverify, _store| {

        let _ = preverify;
        true
    });
    let after_mode = ssl.verify_mode();
    assert_eq!(after_mode, SslVerifyMode::PEER);
    assert_ne!(after_mode, initial_mode);


    ssl.set_connect_state();
    ssl.set_accept_state();

    assert_eq!(ssl.verify_mode(), SslVerifyMode::PEER);


    ssl.set_tmp_dh_callback(|_ssl_ref, _is_export, _keylen| {
        Dh::get_2048_256()
    });


    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ec_key = EcKey::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    assert_eq!(ec_key.group().curve_name(), Some(Nid::X9_62_PRIME256V1));
    let res = ssl.set_tmp_ecdh(&ec_key);
    assert!(res.is_ok(), "set_tmp_ecdh failed: {:?}", res.err());
    let _ = group;


    let vr = ssl.verify_result();
    assert_eq!(vr, openssl::x509::X509VerifyResult::OK);


    assert!(ssl.current_cipher().is_none());
    assert!(ssl.peer_cert_chain().is_none());


    let _v2 = ssl.version2();
}

#[test]
fn test_full_handshake_introspection() {
    let (cert, pkey) = make_self_signed();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_cert = cert.clone();
    let server_key = pkey.clone();

    let server = thread::spawn(move || {
        let mut sb = SslContextBuilder::new(SslMethod::tls_server()).unwrap();
        sb.set_private_key(&server_key).unwrap();
        sb.set_certificate(&server_cert).unwrap();
        let server_ctx = sb.build();

        let (tcp, _) = listener.accept().unwrap();
        let ssl = Ssl::new(&server_ctx).unwrap();
        let mut stream = ssl.accept(tcp).unwrap();


        let s = stream.ssl();
        let mut srand = [0u8; 32];
        let n = s.server_random(&mut srand);
        assert_eq!(n, 32);
        assert!(srand.iter().any(|b| *b != 0));

        let mut crand = [0u8; 32];
        let nc = s.client_random(&mut crand);
        assert_eq!(nc, 32);
        assert_ne!(srand, crand);

        assert!(s.current_cipher().is_some());
        assert!(s.version2().is_some());


        let mut buf = [0u8; 5];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"hello");
        stream.write_all(b"world").unwrap();
        stream.flush().unwrap();
    });

    let mut connector = SslConnector::builder(SslMethod::tls()).unwrap();
    connector.set_verify(SslVerifyMode::NONE);
    let connector = connector.build();

    let tcp = TcpStream::connect(addr).unwrap();
    let mut stream = connector
        .configure()
        .unwrap()
        .verify_hostname(false)
        .use_server_name_indication(false)
        .connect("localhost", tcp)
        .unwrap();

    let s = stream.ssl();


    let cipher = s.current_cipher().expect("cipher set after handshake");
    let cname = cipher.name();
    assert!(!cname.is_empty(), "cipher name should not be empty");


    let mut crand = [0u8; 32];
    let nc = s.client_random(&mut crand);
    assert_eq!(nc, 32);
    assert!(crand.iter().any(|b| *b != 0), "client_random should be populated");


    let mut small = [0u8; 8];
    let n_small = s.client_random(&mut small);
    assert_eq!(n_small, 8);
    assert_eq!(&small[..], &crand[..8]);

    let mut srand = [0u8; 32];
    let ns = s.server_random(&mut srand);
    assert_eq!(ns, 32);
    assert_ne!(srand, crand);


    let v = s.version2().expect("version after handshake");
    assert!(
        v == SslVersion::TLS1_2 || v == SslVersion::TLS1_3 || v == SslVersion::TLS1_1,
        "unexpected version {:?}",
        v
    );
    let vstr = s.version_str();
    assert!(vstr.starts_with("TLS"), "version_str: {}", vstr);


    let chain = s.peer_cert_chain().expect("peer chain present");
    assert!(chain.len() >= 1);


    let _vc = s.verified_chain();



    let _vr = s.verify_result();


    let mut keymat = [0u8; 16];
    let _ = s.export_keying_material_early(&mut keymat, "EXPERIMENTAL test label", b"ctx");


    stream.write_all(b"hello").unwrap();
    stream.flush().unwrap();
    let mut buf = [0u8; 5];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"world");

    server.join().unwrap();
}