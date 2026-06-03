use curl::easy::{Easy2, Handler, ProxyType, WriteError};
use std::time::Duration;

struct Sink(Vec<u8>);

impl Handler for Sink {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

#[test]
fn test_proxy_configuration_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    let pt = ProxyType::Http;
    let pt2 = pt.clone();
    assert!(easy.proxy_type(pt).is_ok());
    assert!(easy.proxy_type(pt2).is_ok());
    assert!(easy.proxy_type(ProxyType::Http).is_ok());


    assert!(easy.proxy_key_password("hunter2").is_ok());
    assert!(easy.proxy_key_password("p@ss w/ spaces!").is_ok());
    assert!(easy.proxy_key_password("x").is_ok());


    assert!(easy.noproxy("localhost").is_ok());
    assert!(easy.noproxy("localhost,127.0.0.1,.internal").is_ok());
    assert!(easy.noproxy("*").is_ok());


    assert!(easy.http_proxy_tunnel(true).is_ok());
    assert!(easy.http_proxy_tunnel(false).is_ok());
    assert!(easy.http_proxy_tunnel(true).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);


    let v = curl::Version::num();
    assert!(!v.is_empty());
    assert!(v.contains('.'));
}

#[test]
fn test_interface_and_local_port_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    assert!(easy.interface("lo").is_ok());
    assert!(easy.interface("eth0").is_ok());
    assert!(easy.interface("any").is_ok());


    assert!(easy.set_local_port(1).is_ok());
    assert!(easy.set_local_port(1024).is_ok());
    assert!(easy.set_local_port(49152).is_ok());
    assert!(easy.set_local_port(65535).is_ok());


    assert!(easy.local_port_range(1).is_ok());
    assert!(easy.local_port_range(10).is_ok());
    assert!(easy.local_port_range(100).is_ok());
    assert!(easy.local_port_range(65535).is_ok());


    assert!(easy.address_scope(0).is_ok());
    assert!(easy.address_scope(1).is_ok());
    assert!(easy.address_scope(42).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_doh_configuration_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    assert!(easy.doh_url(Some("https://cloudflare-dns.com/dns-query")).is_ok());
    assert!(easy.doh_url(Some("https://dns.google/dns-query")).is_ok());
    assert!(easy.doh_url(None).is_ok());
    assert!(easy.doh_url(Some("https://example.test/dns-query")).is_ok());


    assert!(easy.doh_ssl_verify_peer(true).is_ok());
    assert!(easy.doh_ssl_verify_peer(false).is_ok());
    assert!(easy.doh_ssl_verify_peer(true).is_ok());


    assert!(easy.doh_ssl_verify_host(true).is_ok());
    assert!(easy.doh_ssl_verify_host(false).is_ok());
    assert!(easy.doh_ssl_verify_host(true).is_ok());


    assert!(easy.doh_ssl_verify_status(false).is_ok());
    assert!(easy.doh_ssl_verify_status(true).is_ok());
    assert!(easy.doh_ssl_verify_status(false).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_tcp_keepalive_and_combined_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);


    assert!(easy.tcp_keepalive(true).is_ok());
    assert!(easy.tcp_keepalive(false).is_ok());
    assert!(easy.tcp_keepalive(true).is_ok());


    assert!(easy.tcp_keepidle(Duration::from_secs(1)).is_ok());
    assert!(easy.tcp_keepidle(Duration::from_secs(60)).is_ok());
    assert!(easy.tcp_keepidle(Duration::from_secs(7200)).is_ok());

    assert!(easy.tcp_keepintvl(Duration::from_secs(1)).is_ok());
    assert!(easy.tcp_keepintvl(Duration::from_secs(30)).is_ok());
    assert!(easy.tcp_keepintvl(Duration::from_secs(600)).is_ok());


    assert!(easy.proxy_type(ProxyType::Http).is_ok());
    assert!(easy.http_proxy_tunnel(true).is_ok());
    assert!(easy.noproxy("localhost").is_ok());
    assert!(easy.proxy_key_password("secret").is_ok());
    assert!(easy.interface("lo").is_ok());
    assert!(easy.set_local_port(8080).is_ok());
    assert!(easy.local_port_range(10).is_ok());
    assert!(easy.address_scope(0).is_ok());
    assert!(easy.doh_url(None).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}