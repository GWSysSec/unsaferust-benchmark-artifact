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


    easy.perform().expect_err("perform with no URL must fail")
}

#[test]
fn test_error_classifier_methods_on_url_malformat() {
    curl::init();
    let err = make_no_url_error();


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


    assert_eq!(flags[0], false);
    assert_eq!(flags[1], false);
    assert_eq!(flags[2], false);
    assert_eq!(flags[3], false);
    assert_eq!(flags[4], false);
    assert_eq!(flags[14], false);


    let v = curl::Version::num();
    assert!(!v.is_empty());
}

#[test]
fn test_error_classifier_methods_on_bad_scheme_url() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));


    let _ = easy.url("notarealscheme://example.invalid/");
    let err = easy
        .perform()
        .expect_err("perform with unsupported scheme must fail");


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


    assert_eq!(easy.get_ref().0.len(), 0);
}