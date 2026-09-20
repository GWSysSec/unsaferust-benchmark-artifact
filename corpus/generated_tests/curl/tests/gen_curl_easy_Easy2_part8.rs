use curl::easy::{Easy2, Handler, WriteError};
use std::time::Duration;

struct Sink(Vec<u8>);

impl Handler for Sink {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

#[test]
fn test_time_info_getters_defaults() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    // Pre-state
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());

    // Before any perform(), all timing getters should succeed and return zero
    let total = easy.total_time().expect("total_time");
    assert_eq!(total, Duration::from_secs(0));

    let namelookup = easy.namelookup_time().expect("namelookup_time");
    assert_eq!(namelookup, Duration::from_secs(0));

    let connect = easy.connect_time().expect("connect_time");
    assert_eq!(connect, Duration::from_secs(0));

    let appconnect = easy.appconnect_time().expect("appconnect_time");
    assert_eq!(appconnect, Duration::from_secs(0));

    let pretransfer = easy.pretransfer_time().expect("pretransfer_time");
    assert_eq!(pretransfer, Duration::from_secs(0));

    let starttransfer = easy.starttransfer_time().expect("starttransfer_time");
    assert_eq!(starttransfer, Duration::from_secs(0));

    let redirect = easy.redirect_time().expect("redirect_time");
    assert_eq!(redirect, Duration::from_secs(0));

    // All zero — sanity comparison
    assert_eq!(total, namelookup);
    assert_eq!(connect, appconnect);
    assert_eq!(pretransfer, starttransfer);

    // Handler buffer untouched
    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_size_and_errno_getters_defaults() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());

    // header_size / request_size default to 0 before any transfer
    let hdr_size = easy.header_size().expect("header_size");
    assert_eq!(hdr_size, 0);

    let req_size = easy.request_size().expect("request_size");
    assert_eq!(req_size, 0);

    // content_length_download default is -1.0 in libcurl when unknown
    let clen = easy.content_length_download().expect("content_length_download");
    assert!(clen <= 0.0);
    assert!(clen >= -1.0 || clen == 0.0);

    // os_errno defaults to 0
    let errno = easy.os_errno().expect("os_errno");
    assert_eq!(errno, 0);

    // Idempotent: call twice and verify same value
    let errno2 = easy.os_errno().expect("os_errno second");
    assert_eq!(errno, errno2);

    let hdr_size2 = easy.header_size().expect("header_size second");
    assert_eq!(hdr_size, hdr_size2);
}

#[test]
fn test_optional_info_getters_defaults() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);

    // filetime — before any transfer, should be Ok and likely None (or -1)
    let ft = easy.filetime().expect("filetime");
    // It must be either None or a non-positive sentinel — we just assert success
    // and that two consecutive calls return identical results.
    let ft2 = easy.filetime().expect("filetime second");
    assert_eq!(ft, ft2);
    assert!(ft.is_none() || ft.unwrap() <= 0);

    // redirect_url_bytes — none before a transfer
    let ru_is_none = easy.redirect_url_bytes().expect("redirect_url_bytes").is_none();
    assert!(ru_is_none);

    // content_type_bytes — none before a transfer
    let ct_is_none = easy.content_type_bytes().expect("content_type_bytes").is_none();
    assert!(ct_is_none);

    // primary_ip — none before a transfer (no connection yet)
    let ip = easy.primary_ip().expect("primary_ip");
    assert!(ip.is_none() || ip.unwrap().is_empty());

    // Idempotency
    let ru2_is_none = easy.redirect_url_bytes().expect("redirect_url_bytes 2").is_none();
    assert_eq!(ru_is_none, ru2_is_none);

    let ct2_is_none = easy.content_type_bytes().expect("content_type_bytes 2").is_none();
    assert_eq!(ct_is_none, ct2_is_none);

    // Handler buffer untouched
    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_combined_info_getter_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    // Capture a full snapshot of every info getter
    assert_eq!(easy.get_ref().0.len(), 0);

    let total = easy.total_time().expect("total_time");
    let nl = easy.namelookup_time().expect("namelookup_time");
    let conn = easy.connect_time().expect("connect_time");
    let app = easy.appconnect_time().expect("appconnect_time");
    let pre = easy.pretransfer_time().expect("pretransfer_time");
    let start = easy.starttransfer_time().expect("starttransfer_time");
    let redir = easy.redirect_time().expect("redirect_time");
    let hdr = easy.header_size().expect("header_size");
    let req = easy.request_size().expect("request_size");
    let clen = easy.content_length_download().expect("content_length_download");
    let errno = easy.os_errno().expect("os_errno");
    let ft = easy.filetime().expect("filetime");

    // For borrowing methods that return references, capture the derived value immediately
    let ru_is_none = easy.redirect_url_bytes().expect("redirect_url_bytes").is_none();
    let ct_is_none = easy.content_type_bytes().expect("content_type_bytes").is_none();
    let ip = easy.primary_ip().expect("primary_ip");
    let ip_is_none_or_empty = ip.is_none() || ip.unwrap().is_empty();

    // No transfer has happened, so all numeric values should be at their defaults
    assert_eq!(total, Duration::from_secs(0));
    assert_eq!(nl, Duration::from_secs(0));
    assert_eq!(conn, Duration::from_secs(0));
    assert_eq!(app, Duration::from_secs(0));
    assert_eq!(pre, Duration::from_secs(0));
    assert_eq!(start, Duration::from_secs(0));
    assert_eq!(redir, Duration::from_secs(0));
    assert_eq!(hdr, 0);
    assert_eq!(req, 0);
    assert_eq!(errno, 0);
    assert!(clen <= 0.0);

    // Optional getters
    assert!(ru_is_none);
    assert!(ct_is_none);
    assert!(ip_is_none_or_empty);
    assert!(ft.is_none() || ft.unwrap() <= 0);

    // Buffer state preserved
    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}