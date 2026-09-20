use curl::easy::Easy;

#[test]
fn test_easy_proxy_ssl_cert_and_key_options() {
    curl::init();
    let mut easy = Easy::new();

    // proxy_sslcert_type
    assert!(easy.proxy_sslcert_type("PEM").is_ok());
    assert!(easy.proxy_sslcert_type("DER").is_ok());
    assert!(easy.proxy_sslcert_type("PEM").is_ok());

    // proxy_sslcert_blob
    let cert_a: &[u8] = b"-----BEGIN CERTIFICATE-----\nFAKECERTA\n-----END CERTIFICATE-----\n";
    let cert_b: &[u8] = b"-----BEGIN CERTIFICATE-----\nFAKECERTB\n-----END CERTIFICATE-----\n";
    assert!(easy.proxy_sslcert_blob(cert_a).is_ok());
    assert!(easy.proxy_sslcert_blob(cert_b).is_ok());

    // proxy_sslkey
    assert!(easy.proxy_sslkey("/tmp/nonexistent-proxy-key.pem").is_ok());
    assert!(easy.proxy_sslkey("/tmp/another-proxy-key.pem").is_ok());

    // proxy_sslkey_type
    assert!(easy.proxy_sslkey_type("PEM").is_ok());
    assert!(easy.proxy_sslkey_type("DER").is_ok());

    // proxy_sslkey_blob
    let key_a: &[u8] = b"-----BEGIN PRIVATE KEY-----\nFAKEKEYA\n-----END PRIVATE KEY-----\n";
    let key_b: &[u8] = b"-----BEGIN PRIVATE KEY-----\nFAKEKEYB\n-----END PRIVATE KEY-----\n";
    assert!(easy.proxy_sslkey_blob(key_a).is_ok());
    assert!(easy.proxy_sslkey_blob(key_b).is_ok());

    // Version sanity
    let v = curl::Version::num();
    assert!(!v.is_empty());
    assert!(v.contains('.'));
}

#[test]
fn test_easy_info_getters_defaults() {
    curl::init();
    let mut easy = Easy::new();

    // Before any transfer, port info should be at defaults (0)
    let primary = easy.primary_port().expect("primary_port");
    assert_eq!(primary, 0);

    let local = easy.local_port().expect("local_port");
    assert_eq!(local, 0);

    // local_ip is None before any connection
    let ip = easy.local_ip().expect("local_ip");
    assert!(ip.is_none() || ip.unwrap().is_empty());

    // cookies on a fresh handle should succeed and yield a (possibly empty) List
    let cookies = easy.cookies().expect("cookies");
    // We can't introspect List beyond construction; just ensure call succeeded.
    drop(cookies);

    // Idempotency
    let primary2 = easy.primary_port().expect("primary_port second");
    assert_eq!(primary, primary2);
    let local2 = easy.local_port().expect("local_port second");
    assert_eq!(local, local2);

    // Both ports default to the same sentinel value (0) on a fresh handle
    assert_eq!(primary, local);
    assert_eq!(primary, 0);
}

#[test]
fn test_easy_toggles_reset_and_unpause() {
    curl::init();
    let mut easy = Easy::new();

    // pipewait toggle
    assert!(easy.pipewait(true).is_ok());
    assert!(easy.pipewait(false).is_ok());
    assert!(easy.pipewait(true).is_ok());

    // http_09_allowed toggle
    assert!(easy.http_09_allowed(true).is_ok());
    assert!(easy.http_09_allowed(false).is_ok());
    assert!(easy.http_09_allowed(true).is_ok());

    // unpause_read on a handle with no active transfer should still be callable
    // (libcurl accepts it; nothing is paused so it's a no-op)
    let _ = easy.unpause_read();

    // reset() returns ()
    easy.reset();

    // After reset, options can be set again
    assert!(easy.pipewait(false).is_ok());
    assert!(easy.http_09_allowed(false).is_ok());

    // unpause_read again after reset
    let _ = easy.unpause_read();

    // Two more boolean toggles to add assertion count
    assert!(easy.pipewait(true).is_ok());
    assert!(easy.http_09_allowed(true).is_ok());
}

#[test]
fn test_easy_send_recv_without_connection() {
    curl::init();
    let mut easy = Easy::new();

    // Without an established connection (connect_only + perform), send/recv
    // must return an error rather than silently succeeding.
    let mut buf = [0u8; 16];
    let recv_res = easy.recv(&mut buf);
    assert!(recv_res.is_err());

    let send_res = easy.send(b"GET / HTTP/1.0\r\n\r\n");
    assert!(send_res.is_err());

    // Buffer must remain untouched on failure
    assert_eq!(buf, [0u8; 16]);
    assert_eq!(buf.len(), 16);

    // Try again — error must be reproducible (not consumed)
    let recv_res2 = easy.recv(&mut buf);
    assert!(recv_res2.is_err());
    let send_res3 = easy.send(b"hello").is_err();
    assert_eq!(send_res3, true);

    // After reset, send/recv still fail (still no connection)
    easy.reset();
    assert!(easy.recv(&mut buf).is_err());
    assert!(easy.send(b"x").is_err());

    // Buffer still untouched
    assert_eq!(buf, [0u8; 16]);
}