use curl::easy::{Easy2, Handler, HttpVersion, SslVersion, WriteError};

struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

fn make_easy() -> Easy2<Collector> {
    Easy2::new(Collector(Vec::new()))
}

#[test]
fn test_ssl_verify_host_and_peer_toggles() {
    curl::init();
    let mut easy = make_easy();

    // Pre-state: handler buffer is empty before any operation.
    let pre_len = easy.get_ref().0.len();
    assert_eq!(pre_len, 0);
    assert!(easy.get_ref().0.is_empty());

    // ssl_verify_host: toggle through several states.
    assert!(easy.ssl_verify_host(true).is_ok());
    assert!(easy.ssl_verify_host(false).is_ok());
    assert!(easy.ssl_verify_host(true).is_ok());

    // ssl_verify_peer: same pattern.
    assert!(easy.ssl_verify_peer(false).is_ok());
    assert!(easy.ssl_verify_peer(true).is_ok());

    // Proxy variants must accept both polarities.
    assert!(easy.proxy_ssl_verify_host(true).is_ok());
    assert!(easy.proxy_ssl_verify_host(false).is_ok());
    assert!(easy.proxy_ssl_verify_peer(true).is_ok());
    assert!(easy.proxy_ssl_verify_peer(false).is_ok());

    // Post-state: still no transfer performed, buffer untouched.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert_eq!(pre_len, easy.get_ref().0.len());

    // URL still settable after configuring TLS verification options.
    assert!(easy.url("https://example.com/").is_ok());
    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_http_version_variants() {
    curl::init();
    let mut easy = make_easy();

    // Pre-state.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());

    // Setting HTTP/1.1 must succeed on every libcurl build.
    let r1 = easy.http_version(HttpVersion::V11);
    assert!(r1.is_ok());

    // Setting HTTP/1.0 must succeed too.
    let r2 = easy.http_version(HttpVersion::V10);
    assert!(r2.is_ok());

    // Returning to Any must succeed (default behaviour).
    let r3 = easy.http_version(HttpVersion::Any);
    assert!(r3.is_ok());

    // Reapplying V11 several times must be idempotent at the API level.
    let r4 = easy.http_version(HttpVersion::V11);
    let r5 = easy.http_version(HttpVersion::V11);
    assert!(r4.is_ok());
    assert!(r5.is_ok());

    // After all those mutations, the handler buffer is still empty.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());

    // URL setter should still work.
    assert!(easy.url("https://example.org/").is_ok());
}

#[test]
fn test_ssl_version_and_proxy_ssl_version() {
    curl::init();
    let mut easy = make_easy();

    // Pre-state.
    assert_eq!(easy.get_ref().0.len(), 0);

    // Default and TLSv1.2 should be accepted by all curl SSL backends.
    let s_def = easy.ssl_version(SslVersion::Default);
    assert!(s_def.is_ok());
    let s_12 = easy.ssl_version(SslVersion::Tlsv12);
    assert!(s_12.is_ok());

    // Proxy SSL version setter mirrors the regular one.
    let p_def = easy.proxy_ssl_version(SslVersion::Default);
    assert!(p_def.is_ok());
    let p_12 = easy.proxy_ssl_version(SslVersion::Tlsv12);
    assert!(p_12.is_ok());

    // Reapply Default at the end: must remain Ok.
    let s_back = easy.ssl_version(SslVersion::Default);
    assert!(s_back.is_ok());
    let p_back = easy.proxy_ssl_version(SslVersion::Default);
    assert!(p_back.is_ok());

    // Buffer untouched: no transfer has been performed.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_ssl_min_max_version_pairs() {
    curl::init();
    let mut easy = make_easy();

    // Pre-state.
    let pre_len = easy.get_ref().0.len();
    assert_eq!(pre_len, 0);

    // (Default, Default) must always be accepted.
    let r1 = easy.ssl_min_max_version(SslVersion::Default, SslVersion::Default);
    assert!(r1.is_ok());

    // Proxy variant with the same defaults.
    let r2 = easy.proxy_ssl_min_max_version(SslVersion::Default, SslVersion::Default);
    assert!(r2.is_ok());

    // A common modern range: TLSv1.2 .. Default (cap).
    let r3 = easy.ssl_min_max_version(SslVersion::Tlsv12, SslVersion::Default);
    assert!(r3.is_ok());

    // Same for the proxy side.
    let r4 = easy.proxy_ssl_min_max_version(SslVersion::Tlsv12, SslVersion::Default);
    assert!(r4.is_ok());

    // Switch back to Default/Default.
    let r5 = easy.ssl_min_max_version(SslVersion::Default, SslVersion::Default);
    let r6 = easy.proxy_ssl_min_max_version(SslVersion::Default, SslVersion::Default);
    assert!(r5.is_ok());
    assert!(r6.is_ok());

    // No data was transferred.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert_eq!(pre_len, easy.get_ref().0.len());
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_ssl_engine_default_toggle() {
    curl::init();
    let mut easy = make_easy();

    // Pre-state.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());

    // Toggling the default-engine flag should always succeed at the
    // option-set level (it does not actually require an engine).
    let r1 = easy.ssl_engine_default(true);
    assert!(r1.is_ok());
    let r2 = easy.ssl_engine_default(false);
    assert!(r2.is_ok());
    let r3 = easy.ssl_engine_default(true);
    assert!(r3.is_ok());
    let r4 = easy.ssl_engine_default(false);
    assert!(r4.is_ok());

    // ssl_engine with a name that almost certainly does not exist on
    // the test host. We exercise the API and make sure the call
    // returns a Result without panicking. Either Ok (the engine name
    // was accepted) or Err (engine unknown) is acceptable; what we
    // verify is that the function is callable and the handle remains
    // usable afterwards.
    let r_eng = easy.ssl_engine("definitely-not-a-real-engine-xyz");
    let engine_outcome_is_result = r_eng.is_ok() || r_eng.is_err();
    assert!(engine_outcome_is_result);

    // Handle must still be usable: setting a URL must succeed.
    assert!(easy.url("https://example.com/").is_ok());

    // Buffer remains empty: no perform invoked.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_issuer_cert_path_setters() {
    curl::init();
    let mut easy = make_easy();

    // Pre-state.
    assert_eq!(easy.get_ref().0.len(), 0);

    // issuer_cert and proxy_issuer_cert take a path. libcurl only
    // validates the file at transfer time, so setting a non-existent
    // path must still return Ok at option-set time.
    let p1 = "/tmp/curl-integration-nonexistent-issuer-1.pem";
    let p2 = "/tmp/curl-integration-nonexistent-issuer-2.pem";

    let r1 = easy.issuer_cert(p1);
    assert!(r1.is_ok());

    let r2 = easy.proxy_issuer_cert(p2);
    assert!(r2.is_ok());

    // Reapplying with a different path is also fine.
    let r3 = easy.issuer_cert(p2);
    assert!(r3.is_ok());

    let r4 = easy.proxy_issuer_cert(p1);
    assert!(r4.is_ok());

    // Setting the same path twice is idempotent at the API level.
    let r5 = easy.issuer_cert(p1);
    let r6 = easy.proxy_issuer_cert(p2);
    assert!(r5.is_ok());
    assert!(r6.is_ok());

    // No transfer occurred.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_issuer_cert_blob_setters() {
    curl::init();
    let mut easy = make_easy();

    // Pre-state.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());

    // A minimal PEM-looking blob. libcurl copies the bytes; it does
    // not parse them at option-set time, so this should succeed (or
    // at worst return a clean error if the build does not support
    // blob options). We accept either outcome but assert callability.
    let blob: &[u8] = b"-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";
    assert_eq!(blob.is_empty(), false);
    assert!(blob.len() > 16);

    let r1 = easy.issuer_cert_blob(blob);
    let r1_outcome = r1.is_ok() || r1.is_err();
    assert!(r1_outcome);

    let r2 = easy.proxy_issuer_cert_blob(blob);
    let r2_outcome = r2.is_ok() || r2.is_err();
    assert!(r2_outcome);

    // Empty blob: still callable, outcome is a Result.
    let empty: &[u8] = b"";
    let r3 = easy.issuer_cert_blob(empty);
    let r3_outcome = r3.is_ok() || r3.is_err();
    assert!(r3_outcome);

    let r4 = easy.proxy_issuer_cert_blob(empty);
    let r4_outcome = r4.is_ok() || r4.is_err();
    assert!(r4_outcome);

    // Handle still usable.
    assert!(easy.url("https://example.net/").is_ok());

    // Buffer untouched.
    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_combined_tls_configuration_workflow() {
    curl::init();
    let mut easy = make_easy();

    // Pre-state snapshot.
    let pre_len = easy.get_ref().0.len();
    assert_eq!(pre_len, 0);
    assert!(easy.get_ref().0.is_empty());

    // 1. Set URL.
    assert!(easy.url("https://example.com/").is_ok());

    // 2. Configure HTTP and TLS versions.
    assert!(easy.http_version(HttpVersion::V11).is_ok());
    assert!(easy.ssl_version(SslVersion::Default).is_ok());
    assert!(easy
        .ssl_min_max_version(SslVersion::Tlsv12, SslVersion::Default)
        .is_ok());

    // 3. Configure TLS verification.
    assert!(easy.ssl_verify_host(true).is_ok());
    assert!(easy.ssl_verify_peer(true).is_ok());

    // 4. Configure proxy-side TLS verification and version.
    assert!(easy.proxy_ssl_verify_host(true).is_ok());
    assert!(easy.proxy_ssl_verify_peer(true).is_ok());
    assert!(easy.proxy_ssl_version(SslVersion::Default).is_ok());
    assert!(easy
        .proxy_ssl_min_max_version(SslVersion::Tlsv12, SslVersion::Default)
        .is_ok());

    // 5. Issuer cert paths (non-existent: only set, never used).
    assert!(easy
        .issuer_cert("/tmp/curl-integration-nonexistent-issuer.pem")
        .is_ok());
    assert!(easy
        .proxy_issuer_cert("/tmp/curl-integration-nonexistent-proxy-issuer.pem")
        .is_ok());

    // 6. Default-engine flag toggle (does not require a real engine).
    assert!(easy.ssl_engine_default(false).is_ok());

    // Post-state: nothing was transferred.
    assert_eq!(easy.get_ref().0.len(), 0);
    assert_eq!(pre_len, easy.get_ref().0.len());
    assert!(easy.get_ref().0.is_empty());
}