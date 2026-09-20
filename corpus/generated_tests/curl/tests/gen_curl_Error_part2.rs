use curl::easy::{Easy2, Handler, WriteError};

struct Sink(Vec<u8>);

impl Handler for Sink {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

fn make_no_url_error() -> curl::Error {
    let mut easy = Easy2::new(Sink(Vec::new()));
    // Performing with no URL set yields CURLE_URL_MALFORMAT (or similar) — an Error
    // that is NOT any of the specific kinds we are testing for.
    easy.perform().expect_err("perform with no URL must fail")
}

#[test]
fn test_error_classifier_methods_on_url_malformat() {
    curl::init();
    let err = make_no_url_error();

    // The URL-malformat error should not match any of the specific categories below
    assert_eq!(err.is_range_error(), false);
    assert_eq!(err.is_http_post_error(), false);
    assert_eq!(err.is_ssl_connect_error(), false);
    assert_eq!(err.is_bad_download_resume(), false);
    assert_eq!(err.is_file_couldnt_read_file(), false);
    assert_eq!(err.is_function_not_found(), false);
    assert_eq!(err.is_bad_function_argument(), false);
    assert_eq!(err.is_interface_failed(), false);
    assert_eq!(err.is_too_many_redirects(), false);
    assert_eq!(err.is_unknown_option(), false);
    assert_eq!(err.is_peer_failed_verification(), false);
    assert_eq!(err.is_got_nothing(), false);
    assert_eq!(err.is_ssl_engine_notfound(), false);
    assert_eq!(err.is_ssl_engine_setfailed(), false);
    assert_eq!(err.is_send_error(), false);
}

#[test]
fn test_error_classifier_methods_idempotent() {
    curl::init();
    let err = make_no_url_error();

    // Calling each classifier twice must return the same value (no internal state)
    assert_eq!(err.is_range_error(), err.is_range_error());
    assert_eq!(err.is_http_post_error(), err.is_http_post_error());
    assert_eq!(err.is_ssl_connect_error(), err.is_ssl_connect_error());
    assert_eq!(err.is_bad_download_resume(), err.is_bad_download_resume());
    assert_eq!(err.is_file_couldnt_read_file(), err.is_file_couldnt_read_file());
    assert_eq!(err.is_function_not_found(), err.is_function_not_found());
    assert_eq!(err.is_bad_function_argument(), err.is_bad_function_argument());
    assert_eq!(err.is_interface_failed(), err.is_interface_failed());
    assert_eq!(err.is_too_many_redirects(), err.is_too_many_redirects());
    assert_eq!(err.is_unknown_option(), err.is_unknown_option());
    assert_eq!(err.is_peer_failed_verification(), err.is_peer_failed_verification());
    assert_eq!(err.is_got_nothing(), err.is_got_nothing());
    assert_eq!(err.is_ssl_engine_notfound(), err.is_ssl_engine_notfound());
    assert_eq!(err.is_ssl_engine_setfailed(), err.is_ssl_engine_setfailed());
    assert_eq!(err.is_send_error(), err.is_send_error());
}

#[test]
fn test_error_classifier_methods_mutually_exclusive_for_single_code() {
    curl::init();
    let err = make_no_url_error();

    // For any single libcurl error code, at most ONE of these distinct categories
    // can match. Count how many return true — must be 0 or 1.
    let flags = [
        err.is_range_error(),
        err.is_http_post_error(),
        err.is_ssl_connect_error(),
        err.is_bad_download_resume(),
        err.is_file_couldnt_read_file(),
        err.is_function_not_found(),
        err.is_bad_function_argument(),
        err.is_interface_failed(),
        err.is_too_many_redirects(),
        err.is_unknown_option(),
        err.is_peer_failed_verification(),
        err.is_got_nothing(),
        err.is_ssl_engine_notfound(),
        err.is_ssl_engine_setfailed(),
        err.is_send_error(),
    ];
    assert_eq!(flags.len(), 15);
    let true_count = flags.iter().filter(|b| **b).count();
    assert!(true_count <= 1);
    assert_eq!(true_count, 0);

    // None of these specific categories should match the URL-malformat error.
    assert_eq!(flags[0], false);
    assert_eq!(flags[1], false);
    assert_eq!(flags[2], false);
    assert_eq!(flags[3], false);
    assert_eq!(flags[4], false);
    assert_eq!(flags[14], false);

    // Version sanity (ensures we're really linking against libcurl)
    let v = curl::Version::num();
    assert!(!v.is_empty());
}

#[test]
fn test_error_classifier_methods_on_bad_scheme_url() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    // A clearly unsupported scheme should fail at perform() time.
    let _ = easy.url("notarealscheme://example.invalid/");
    let err = easy
        .perform()
        .expect_err("perform with unsupported scheme must fail");

    // None of these specific kinds match the unsupported-protocol / malformat errors
    assert_eq!(err.is_range_error(), false);
    assert_eq!(err.is_http_post_error(), false);
    assert_eq!(err.is_ssl_connect_error(), false);
    assert_eq!(err.is_bad_download_resume(), false);
    assert_eq!(err.is_function_not_found(), false);
    assert_eq!(err.is_bad_function_argument(), false);
    assert_eq!(err.is_interface_failed(), false);
    assert_eq!(err.is_too_many_redirects(), false);
    assert_eq!(err.is_unknown_option(), false);
    assert_eq!(err.is_peer_failed_verification(), false);
    assert_eq!(err.is_got_nothing(), false);
    assert_eq!(err.is_ssl_engine_notfound(), false);
    assert_eq!(err.is_ssl_engine_setfailed(), false);
    assert_eq!(err.is_send_error(), false);
    assert_eq!(err.is_file_couldnt_read_file(), false);

    // Handler buffer was never written to (no successful transfer)
    assert_eq!(easy.get_ref().0.len(), 0);
}