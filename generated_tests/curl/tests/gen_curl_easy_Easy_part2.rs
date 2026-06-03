use curl::easy::Easy;

#[test]
fn test_easy_proxy_ssl_cert_and_key_options() {
    curl::init();
    let mut easy = Easy::new();


    assert!(easy.proxy_sslcert_type("PEM").is_ok());
    assert!(easy.proxy_sslcert_type("DER").is_ok());
    assert!(easy.proxy_sslcert_type("PEM").is_ok());


    let cert_a: &[u8] = b"-----BEGIN CERTIFICATE-----\nFAKECERTA\n-----END CERTIFICATE-----\n";
    let cert_b: &[u8] = b"-----BEGIN CERTIFICATE-----\nFAKECERTB\n-----END CERTIFICATE-----\n";
    assert!(easy.proxy_sslcert_blob(cert_a).is_ok());
    assert!(easy.proxy_sslcert_blob(cert_b).is_ok());


    assert!(easy.proxy_sslkey("/tmp/nonexistent-proxy-key.pem").is_ok());
    assert!(easy.proxy_sslkey("/tmp/another-proxy-key.pem").is_ok());


    assert!(easy.proxy_sslkey_type("PEM").is_ok());
    assert!(easy.proxy_sslkey_type("DER").is_ok());


    let key_a: &[u8] = b"-----BEGIN PRIVATE KEY-----\nFAKEKEYA\n-----END PRIVATE KEY-----\n";
    let key_b: &[u8] = b"-----BEGIN PRIVATE KEY-----\nFAKEKEYB\n-----END PRIVATE KEY-----\n";
    assert!(easy.proxy_sslkey_blob(key_a).is_ok());
    assert!(easy.proxy_sslkey_blob(key_b).is_ok());


    let v = curl::Version::num();
    assert!(!v.is_empty());
    assert!(v.contains('.'));
}

#[test]
fn test_easy_info_getters_defaults() {
    curl::init();
    let mut easy = Easy::new();


    let primary = easy.primary_port().expect("primary_port");
    assert_eq!(primary, 0);

    let local = easy.local_port().expect("local_port");
    assert_eq!(local, 0);


    let ip = easy.local_ip().expect("local_ip");
    assert!(ip.is_none() || ip.unwrap().is_empty());


    let cookies = easy.cookies().expect("cookies");

    drop(cookies);


    let primary2 = easy.primary_port().expect("primary_port second");
    assert_eq!(primary, primary2);
    let local2 = easy.local_port().expect("local_port second");
    assert_eq!(local, local2);


    assert_eq!(primary, local);
    assert_eq!(primary, 0);
}

#[test]
fn test_easy_toggles_reset_and_unpause() {
    curl::init();
    let mut easy = Easy::new();


    assert!(easy.pipewait(true).is_ok());
    assert!(easy.pipewait(false).is_ok());
    assert!(easy.pipewait(true).is_ok());


    assert!(easy.http_09_allowed(true).is_ok());
    assert!(easy.http_09_allowed(false).is_ok());
    assert!(easy.http_09_allowed(true).is_ok());



    let _ = easy.unpause_read();


    easy.reset();


    assert!(easy.pipewait(false).is_ok());
    assert!(easy.http_09_allowed(false).is_ok());


    let _ = easy.unpause_read();


    assert!(easy.pipewait(true).is_ok());
    assert!(easy.http_09_allowed(true).is_ok());
}

#[test]
fn test_easy_send_recv_without_connection() {
    curl::init();
    let mut easy = Easy::new();



    let mut buf = [0u8; 16];
    let recv_res = easy.recv(&mut buf);
    assert!(recv_res.is_err());

    let send_res = easy.send(b"GET / HTTP/1.0\r\n\r\n");
    assert!(send_res.is_err());


    assert_eq!(buf, [0u8; 16]);
    assert_eq!(buf.len(), 16);


    let recv_res2 = easy.recv(&mut buf);
    assert!(recv_res2.is_err());
    let send_res3 = easy.send(b"hello").is_err();
    assert_eq!(send_res3, true);


    easy.reset();
    assert!(easy.recv(&mut buf).is_err());
    assert!(easy.send(b"x").is_err());


    assert_eq!(buf, [0u8; 16]);
}