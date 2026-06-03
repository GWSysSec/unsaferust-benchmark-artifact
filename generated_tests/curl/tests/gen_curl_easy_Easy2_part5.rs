use curl::easy::{Easy2, Handler, IpResolve, WriteError};
use std::time::Duration;

struct Sink(Vec<u8>);

impl Handler for Sink {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

#[test]
fn test_connection_lifecycle_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    assert!(easy.maxage_conn(Duration::from_secs(0)).is_ok());
    assert!(easy.maxage_conn(Duration::from_secs(1)).is_ok());
    assert!(easy.maxage_conn(Duration::from_secs(60)).is_ok());
    assert!(easy.maxage_conn(Duration::from_secs(3600)).is_ok());


    assert!(easy.fresh_connect(true).is_ok());
    assert!(easy.fresh_connect(false).is_ok());
    assert!(easy.fresh_connect(true).is_ok());

    assert!(easy.forbid_reuse(true).is_ok());
    assert!(easy.forbid_reuse(false).is_ok());
    assert!(easy.forbid_reuse(true).is_ok());


    assert!(easy.connect_timeout(Duration::from_secs(1)).is_ok());
    assert!(easy.connect_timeout(Duration::from_secs(10)).is_ok());
    assert!(easy.connect_timeout(Duration::from_secs(60)).is_ok());


    assert!(easy.connect_only(true).is_ok());
    assert!(easy.connect_only(false).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_ip_resolve_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    let r = IpResolve::Any;
    let r2 = r.clone();
    let r3 = r2.clone();
    assert!(easy.ip_resolve(r).is_ok());
    assert!(easy.ip_resolve(r2).is_ok());
    assert!(easy.ip_resolve(r3).is_ok());
    assert!(easy.ip_resolve(IpResolve::Any).is_ok());
    assert!(easy.ip_resolve(IpResolve::Any).is_ok());


    assert!(easy.connect_timeout(Duration::from_secs(5)).is_ok());
    assert!(easy.fresh_connect(false).is_ok());
    assert!(easy.forbid_reuse(false).is_ok());
    assert!(easy.maxage_conn(Duration::from_secs(120)).is_ok());


    let v = curl::Version::num();
    assert!(!v.is_empty());
    assert!(v.contains('.'));

    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_ssl_client_cert_and_key_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);


    assert!(easy.ssl_cert("/tmp/nonexistent-client.pem").is_ok());
    assert!(easy.ssl_cert("/tmp/another-client.pem").is_ok());


    assert!(easy.ssl_cert_type("PEM").is_ok());
    assert!(easy.ssl_cert_type("DER").is_ok());


    let cert_blob: &[u8] = b"-----BEGIN CERTIFICATE-----\nMIIFAKE\n-----END CERTIFICATE-----\n";
    assert!(easy.ssl_cert_blob(cert_blob).is_ok());
    assert!(easy.ssl_cert_blob(b"").is_ok());


    assert!(easy.ssl_key("/tmp/nonexistent-key.pem").is_ok());
    assert!(easy.ssl_key("/tmp/another-key.pem").is_ok());


    assert!(easy.ssl_key_type("PEM").is_ok());
    assert!(easy.ssl_key_type("DER").is_ok());


    let key_blob: &[u8] = b"-----BEGIN PRIVATE KEY-----\nFAKEKEY\n-----END PRIVATE KEY-----\n";
    assert!(easy.ssl_key_blob(key_blob).is_ok());


    assert!(easy.key_password("hunter2").is_ok());
    assert!(easy.key_password("p@ss w/ spaces!").is_ok());
    assert!(easy.key_password("x").is_ok());

    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_ca_blob_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());

    let ca_a: &[u8] = b"-----BEGIN CERTIFICATE-----\nFAKECA1\n-----END CERTIFICATE-----\n";
    let ca_b: &[u8] = b"-----BEGIN CERTIFICATE-----\nFAKECA2\n-----END CERTIFICATE-----\n";
    let ca_chain: Vec<u8> = {
        let mut v = Vec::new();
        v.extend_from_slice(ca_a);
        v.extend_from_slice(ca_b);
        v
    };
    assert_eq!(ca_chain.len(), ca_a.len() + ca_b.len());
    assert!(ca_chain.len() > 0);


    assert!(easy.ssl_cainfo_blob(ca_a).is_ok());
    assert!(easy.ssl_cainfo_blob(ca_b).is_ok());
    assert!(easy.ssl_cainfo_blob(&ca_chain).is_ok());


    assert!(easy.proxy_ssl_cainfo_blob(ca_a).is_ok());
    assert!(easy.proxy_ssl_cainfo_blob(ca_b).is_ok());
    assert!(easy.proxy_ssl_cainfo_blob(&ca_chain).is_ok());


    assert!(easy.maxage_conn(Duration::from_secs(30)).is_ok());
    assert!(easy.fresh_connect(true).is_ok());
    assert!(easy.forbid_reuse(false).is_ok());
    assert!(easy.connect_timeout(Duration::from_secs(15)).is_ok());
    assert!(easy.ip_resolve(IpResolve::Any).is_ok());
    assert!(easy.connect_only(false).is_ok());

    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}