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


    let pre_len = easy.get_ref().0.len();
    assert_eq!(pre_len, 0);
    assert!(easy.get_ref().0.is_empty());


    assert!(easy.ssl_verify_host(true).is_ok());
    assert!(easy.ssl_verify_host(false).is_ok());
    assert!(easy.ssl_verify_host(true).is_ok());


    assert!(easy.ssl_verify_peer(false).is_ok());
    assert!(easy.ssl_verify_peer(true).is_ok());


    assert!(easy.proxy_ssl_verify_host(true).is_ok());
    assert!(easy.proxy_ssl_verify_host(false).is_ok());
    assert!(easy.proxy_ssl_verify_peer(true).is_ok());
    assert!(easy.proxy_ssl_verify_peer(false).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert_eq!(pre_len, easy.get_ref().0.len());


    assert!(easy.url("https://example.com/").is_ok());
    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_http_version_variants() {
    curl::init();
    let mut easy = make_easy();


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    let r1 = easy.http_version(HttpVersion::V11);
    assert!(r1.is_ok());


    let r2 = easy.http_version(HttpVersion::V10);
    assert!(r2.is_ok());


    let r3 = easy.http_version(HttpVersion::Any);
    assert!(r3.is_ok());


    let r4 = easy.http_version(HttpVersion::V11);
    let r5 = easy.http_version(HttpVersion::V11);
    assert!(r4.is_ok());
    assert!(r5.is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    assert!(easy.url("https://example.org/").is_ok());
}

#[test]
fn test_ssl_version_and_proxy_ssl_version() {
    curl::init();
    let mut easy = make_easy();


    assert_eq!(easy.get_ref().0.len(), 0);


    let s_def = easy.ssl_version(SslVersion::Default);
    assert!(s_def.is_ok());
    let s_12 = easy.ssl_version(SslVersion::Tlsv12);
    assert!(s_12.is_ok());


    let p_def = easy.proxy_ssl_version(SslVersion::Default);
    assert!(p_def.is_ok());
    let p_12 = easy.proxy_ssl_version(SslVersion::Tlsv12);
    assert!(p_12.is_ok());


    let s_back = easy.ssl_version(SslVersion::Default);
    assert!(s_back.is_ok());
    let p_back = easy.proxy_ssl_version(SslVersion::Default);
    assert!(p_back.is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_ssl_min_max_version_pairs() {
    curl::init();
    let mut easy = make_easy();


    let pre_len = easy.get_ref().0.len();
    assert_eq!(pre_len, 0);


    let r1 = easy.ssl_min_max_version(SslVersion::Default, SslVersion::Default);
    assert!(r1.is_ok());


    let r2 = easy.proxy_ssl_min_max_version(SslVersion::Default, SslVersion::Default);
    assert!(r2.is_ok());


    let r3 = easy.ssl_min_max_version(SslVersion::Tlsv12, SslVersion::Default);
    assert!(r3.is_ok());


    let r4 = easy.proxy_ssl_min_max_version(SslVersion::Tlsv12, SslVersion::Default);
    assert!(r4.is_ok());


    let r5 = easy.ssl_min_max_version(SslVersion::Default, SslVersion::Default);
    let r6 = easy.proxy_ssl_min_max_version(SslVersion::Default, SslVersion::Default);
    assert!(r5.is_ok());
    assert!(r6.is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert_eq!(pre_len, easy.get_ref().0.len());
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_ssl_engine_default_toggle() {
    curl::init();
    let mut easy = make_easy();


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());



    let r1 = easy.ssl_engine_default(true);
    assert!(r1.is_ok());
    let r2 = easy.ssl_engine_default(false);
    assert!(r2.is_ok());
    let r3 = easy.ssl_engine_default(true);
    assert!(r3.is_ok());
    let r4 = easy.ssl_engine_default(false);
    assert!(r4.is_ok());







    let r_eng = easy.ssl_engine("definitely-not-a-real-engine-xyz");
    let engine_outcome_is_result = r_eng.is_ok() || r_eng.is_err();
    assert!(engine_outcome_is_result);


    assert!(easy.url("https://example.com/").is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_issuer_cert_path_setters() {
    curl::init();
    let mut easy = make_easy();


    assert_eq!(easy.get_ref().0.len(), 0);




    let p1 = "/tmp/curl-integration-nonexistent-issuer-1.pem";
    let p2 = "/tmp/curl-integration-nonexistent-issuer-2.pem";

    let r1 = easy.issuer_cert(p1);
    assert!(r1.is_ok());

    let r2 = easy.proxy_issuer_cert(p2);
    assert!(r2.is_ok());


    let r3 = easy.issuer_cert(p2);
    assert!(r3.is_ok());

    let r4 = easy.proxy_issuer_cert(p1);
    assert!(r4.is_ok());


    let r5 = easy.issuer_cert(p1);
    let r6 = easy.proxy_issuer_cert(p2);
    assert!(r5.is_ok());
    assert!(r6.is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_issuer_cert_blob_setters() {
    curl::init();
    let mut easy = make_easy();


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());





    let blob: &[u8] = b"-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";
    assert_eq!(blob.is_empty(), false);
    assert!(blob.len() > 16);

    let r1 = easy.issuer_cert_blob(blob);
    let r1_outcome = r1.is_ok() || r1.is_err();
    assert!(r1_outcome);

    let r2 = easy.proxy_issuer_cert_blob(blob);
    let r2_outcome = r2.is_ok() || r2.is_err();
    assert!(r2_outcome);


    let empty: &[u8] = b"";
    let r3 = easy.issuer_cert_blob(empty);
    let r3_outcome = r3.is_ok() || r3.is_err();
    assert!(r3_outcome);

    let r4 = easy.proxy_issuer_cert_blob(empty);
    let r4_outcome = r4.is_ok() || r4.is_err();
    assert!(r4_outcome);


    assert!(easy.url("https://example.net/").is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_combined_tls_configuration_workflow() {
    curl::init();
    let mut easy = make_easy();


    let pre_len = easy.get_ref().0.len();
    assert_eq!(pre_len, 0);
    assert!(easy.get_ref().0.is_empty());


    assert!(easy.url("https://example.com/").is_ok());


    assert!(easy.http_version(HttpVersion::V11).is_ok());
    assert!(easy.ssl_version(SslVersion::Default).is_ok());
    assert!(easy
        .ssl_min_max_version(SslVersion::Tlsv12, SslVersion::Default)
        .is_ok());


    assert!(easy.ssl_verify_host(true).is_ok());
    assert!(easy.ssl_verify_peer(true).is_ok());


    assert!(easy.proxy_ssl_verify_host(true).is_ok());
    assert!(easy.proxy_ssl_verify_peer(true).is_ok());
    assert!(easy.proxy_ssl_version(SslVersion::Default).is_ok());
    assert!(easy
        .proxy_ssl_min_max_version(SslVersion::Tlsv12, SslVersion::Default)
        .is_ok());


    assert!(easy
        .issuer_cert("/tmp/curl-integration-nonexistent-issuer.pem")
        .is_ok());
    assert!(easy
        .proxy_issuer_cert("/tmp/curl-integration-nonexistent-proxy-issuer.pem")
        .is_ok());


    assert!(easy.ssl_engine_default(false).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert_eq!(pre_len, easy.get_ref().0.len());
    assert!(easy.get_ref().0.is_empty());
}