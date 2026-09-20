use curl::easy::{Easy2, Handler, WriteError, SslOpt};
use std::time::Duration;
use std::path::Path;

struct Collector {
    data: Vec<u8>,
    writes: usize,
}

impl Collector {
    fn new() -> Self {
        Collector { data: Vec::new(), writes: 0 }
    }
}

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.data.extend_from_slice(data);
        self.writes += 1;
        Ok(data.len())
    }
}

#[test]
fn test_ssl_file_setters_workflow() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());

    // Pre-state: handler unused
    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().writes, 0);

    // crlfile via &str
    let r1 = easy.crlfile("/tmp/test_crlfile_int_a.pem");
    assert!(r1.is_ok(), "crlfile &str failed: {:?}", r1.err());

    // crlfile via &Path - re-set is idempotent
    let r2 = easy.crlfile(Path::new("/tmp/test_crlfile_int_b.pem"));
    assert!(r2.is_ok(), "crlfile &Path failed: {:?}", r2.err());

    // proxy_crlfile
    let r3 = easy.proxy_crlfile("/tmp/test_proxy_crlfile_a.pem");
    assert!(r3.is_ok(), "proxy_crlfile failed: {:?}", r3.err());

    let r4 = easy.proxy_crlfile(Path::new("/tmp/test_proxy_crlfile_b.pem"));
    assert!(r4.is_ok(), "proxy_crlfile re-set failed");

    // random_file (deprecated since 7.84 but still accepted)
    let r5 = easy.random_file("/dev/urandom");
    assert!(r5.is_ok(), "random_file failed: {:?}", r5.err());

    // egd_socket (deprecated, still accepted)
    let r6 = easy.egd_socket("/tmp/no_such_egd_sock");
    assert!(r6.is_ok(), "egd_socket failed: {:?}", r6.err());

    // Post-state: handler still untouched (just setopt calls)
    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().writes, 0);

    // URL setting still works after all option calls
    assert!(easy.url("https://example.invalid/").is_ok());
}

#[test]
fn test_ssl_cipher_lists_workflow() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());

    // Pre-state
    assert_eq!(easy.get_ref().writes, 0);
    assert_eq!(easy.get_ref().data.len(), 0);

    let ciphers = "ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-GCM-SHA256";

    // ssl_cipher_list
    let r1 = easy.ssl_cipher_list(ciphers);
    assert!(r1.is_ok(), "ssl_cipher_list failed: {:?}", r1.err());

    // proxy_ssl_cipher_list
    let r2 = easy.proxy_ssl_cipher_list(ciphers);
    assert!(r2.is_ok(), "proxy_ssl_cipher_list failed: {:?}", r2.err());

    // Single cipher reset
    let r3 = easy.ssl_cipher_list("AES256-SHA");
    assert!(r3.is_ok());

    // Long list
    let big = "ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-GCM-SHA256:\
               ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES128-GCM-SHA256:\
               DHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES128-GCM-SHA256";
    let r4 = easy.proxy_ssl_cipher_list(big);
    assert!(r4.is_ok());

    let r5 = easy.ssl_cipher_list(big);
    assert!(r5.is_ok());

    // Reset to a different value
    let r6 = easy.ssl_cipher_list("HIGH:!aNULL:!MD5");
    assert!(r6.is_ok());

    // Handler untouched
    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().writes, 0);

    // Followup config still works
    assert!(easy.url("https://example.com/").is_ok());
}

#[test]
fn test_ssl_bool_and_misc_options() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());

    // Pre-state
    assert_eq!(easy.get_ref().writes, 0);

    // certinfo on/off
    let r1 = easy.certinfo(true);
    assert!(r1.is_ok(), "certinfo(true) failed: {:?}", r1.err());

    let r2 = easy.certinfo(false);
    assert!(r2.is_ok(), "certinfo(false) failed: {:?}", r2.err());

    // toggle again to ensure repeatable
    let r2b = easy.certinfo(true);
    assert!(r2b.is_ok());

    // ssl_sessionid_cache on/off
    let r3 = easy.ssl_sessionid_cache(true);
    assert!(r3.is_ok(), "ssl_sessionid_cache(true) failed");

    let r4 = easy.ssl_sessionid_cache(false);
    assert!(r4.is_ok(), "ssl_sessionid_cache(false) failed");

    // pinned_public_key - sha256 hash form
    let r5 = easy.pinned_public_key(
        "sha256//YhKJKSzoTt2b5FP18fvpHo7fJYqQCjAa3HWY3tvRMwE=",
    );
    assert!(r5.is_ok(), "pinned_public_key sha256 form failed: {:?}", r5.err());

    // pinned_public_key - file form
    let r6 = easy.pinned_public_key("/tmp/pinned_pub_int.pem");
    assert!(r6.is_ok(), "pinned_public_key file form failed");

    // expect_100_timeout - typical value
    let r7 = easy.expect_100_timeout(Duration::from_secs(2));
    assert!(r7.is_ok(), "expect_100_timeout 2s failed");

    // expect_100_timeout - sub-second
    let r8 = easy.expect_100_timeout(Duration::from_millis(500));
    assert!(r8.is_ok(), "expect_100_timeout 500ms failed");

    // expect_100_timeout - longer
    let r9 = easy.expect_100_timeout(Duration::from_secs(10));
    assert!(r9.is_ok(), "expect_100_timeout 10s failed");

    // Handler still untouched
    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().writes, 0);
}

#[test]
fn test_ssl_opt_workflow() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());

    // Pre-state
    assert_eq!(easy.get_ref().writes, 0);

    // Default SslOpt - apply to ssl_options
    let opt = SslOpt::new();
    let r1 = easy.ssl_options(&opt);
    assert!(r1.is_ok(), "ssl_options default failed: {:?}", r1.err());

    // Same opt to proxy variant
    let r2 = easy.proxy_ssl_options(&opt);
    assert!(r2.is_ok(), "proxy_ssl_options default failed: {:?}", r2.err());

    // Construct another SslOpt and apply
    let opt2 = SslOpt::new();
    let r3 = easy.ssl_options(&opt2);
    assert!(r3.is_ok());

    let r4 = easy.proxy_ssl_options(&opt2);
    assert!(r4.is_ok());

    // Reuse opt again (passed by reference, not consumed)
    let r5 = easy.ssl_options(&opt);
    assert!(r5.is_ok());

    let r6 = easy.proxy_ssl_options(&opt);
    assert!(r6.is_ok());

    // One more apply
    let r7 = easy.ssl_options(&opt);
    assert!(r7.is_ok());

    // Handler untouched
    assert_eq!(easy.get_ref().writes, 0);
    assert_eq!(easy.get_ref().data.len(), 0);

    // URL still settable
    assert!(easy.url("https://example.org/").is_ok());
}

#[test]
fn test_getinfo_before_perform() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());

    // Pre-state
    assert_eq!(easy.get_ref().writes, 0);
    assert_eq!(easy.get_ref().data.len(), 0);

    // Configure some options first
    assert!(easy.url("https://example.invalid:1/").is_ok());
    assert!(easy.certinfo(true).is_ok());

    // time_condition_unmet before perform - default is false
    let tcu = easy.time_condition_unmet();
    assert!(tcu.is_ok(), "time_condition_unmet errored: {:?}", tcu.err());
    assert_eq!(
        tcu.unwrap(),
        false,
        "time_condition_unmet default should be false"
    );

    // http_connectcode before perform - default is 0
    let hcc = easy.http_connectcode();
    assert!(hcc.is_ok(), "http_connectcode errored: {:?}", hcc.err());
    assert_eq!(hcc.unwrap(), 0, "http_connectcode default should be 0");

    // effective_url_bytes before perform - may be None or echo back set URL
    let eub = easy.effective_url_bytes();
    assert!(eub.is_ok(), "effective_url_bytes errored: {:?}", eub.err());
    let bytes_opt = eub.unwrap();
    let saw_url = match bytes_opt {
        Some(b) => {
            let s = std::str::from_utf8(b).expect("effective_url should be valid utf8");
            assert!(
                s.contains("example.invalid") || s.is_empty(),
                "unexpected effective_url contents: {}",
                s
            );
            !s.is_empty()
        }
        None => false,
    };
    // Either way the call must not panic and the type must be Option<&[u8]>
    let _ = saw_url;

    // Confirm handler still hasn't been written to
    assert_eq!(easy.get_ref().writes, 0);
    assert_eq!(easy.get_ref().data.len(), 0);
}

#[test]
fn test_full_configuration_pipeline() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());

    // Pre-state
    assert_eq!(easy.get_ref().writes, 0);

    // Apply a comprehensive set of target options
    assert!(easy.url("https://example.invalid/").is_ok());
    assert!(easy.crlfile("/tmp/full_crl.pem").is_ok());
    assert!(easy.proxy_crlfile("/tmp/full_proxy_crl.pem").is_ok());
    assert!(easy.certinfo(true).is_ok());
    assert!(easy
        .pinned_public_key("sha256//AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
        .is_ok());
    assert!(easy.ssl_cipher_list("HIGH:!aNULL").is_ok());
    assert!(easy.proxy_ssl_cipher_list("HIGH:!aNULL").is_ok());
    assert!(easy.ssl_sessionid_cache(true).is_ok());
    assert!(easy.expect_100_timeout(Duration::from_secs(1)).is_ok());
    assert!(easy.random_file("/dev/urandom").is_ok());
    assert!(easy.egd_socket("/tmp/no_egd_full").is_ok());

    let opt = SslOpt::new();
    assert!(easy.ssl_options(&opt).is_ok());
    assert!(easy.proxy_ssl_options(&opt).is_ok());

    // Inspect getinfo before any perform
    let unmet = easy.time_condition_unmet().expect("tcu ok");
    assert_eq!(unmet, false, "fresh handle reports condition met=false");

    let connect_code = easy.http_connectcode().expect("hcc ok");
    assert_eq!(connect_code, 0, "fresh handle has no CONNECT response");

    // Reconfiguration after inspection still works
    assert!(easy.url("https://other.invalid/").is_ok());
    assert!(easy.certinfo(false).is_ok());
    assert!(easy.ssl_sessionid_cache(false).is_ok());

    // Final invariants
    assert_eq!(easy.get_ref().writes, 0);
    assert_eq!(easy.get_ref().data.len(), 0);
}