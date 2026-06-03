use curl::easy::{Easy2, Handler, WriteError};

struct Sink(Vec<u8>);

impl Handler for Sink {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

fn no_url_error() -> curl::Error {
    let mut easy = Easy2::new(Sink(Vec::new()));
    easy.perform().expect_err("perform with no URL must fail")
}

fn bad_scheme_error() -> curl::Error {
    let mut easy = Easy2::new(Sink(Vec::new()));
    let _ = easy.url("zzunknown://nowhere.invalid/");
    easy.perform().expect_err("perform with bad scheme must fail")
}

#[test]
fn test_ssl_and_misc_error_classifiers_all_false() {
    curl::init();
    let err = no_url_error();



    assert_eq!(err.is_recv_error(), false);
    assert_eq!(err.is_ssl_certproblem(), false);
    assert_eq!(err.is_ssl_cipher(), false);
    assert_eq!(err.is_ssl_cacert(), false);
    assert_eq!(err.is_bad_content_encoding(), false);
    assert_eq!(err.is_filesize_exceeded(), false);
    assert_eq!(err.is_use_ssl_failed(), false);
    assert_eq!(err.is_send_fail_rewind(), false);
    assert_eq!(err.is_ssl_engine_initfailed(), false);
    assert_eq!(err.is_login_denied(), false);
    assert_eq!(err.is_conv_failed(), false);
    assert_eq!(err.is_conv_required(), false);
    assert_eq!(err.is_ssl_cacert_badfile(), false);
    assert_eq!(err.is_ssl_crl_badfile(), false);
    assert_eq!(err.is_ssl_shutdown_failed(), false);
}

#[test]
fn test_classifiers_idempotent_on_bad_scheme() {
    curl::init();
    let err = bad_scheme_error();


    assert_eq!(err.is_recv_error(), err.is_recv_error());
    assert_eq!(err.is_ssl_certproblem(), err.is_ssl_certproblem());
    assert_eq!(err.is_ssl_cipher(), err.is_ssl_cipher());
    assert_eq!(err.is_ssl_cacert(), err.is_ssl_cacert());
    assert_eq!(err.is_bad_content_encoding(), err.is_bad_content_encoding());
    assert_eq!(err.is_filesize_exceeded(), err.is_filesize_exceeded());
    assert_eq!(err.is_use_ssl_failed(), err.is_use_ssl_failed());
    assert_eq!(err.is_send_fail_rewind(), err.is_send_fail_rewind());
    assert_eq!(err.is_ssl_engine_initfailed(), err.is_ssl_engine_initfailed());
    assert_eq!(err.is_login_denied(), err.is_login_denied());
    assert_eq!(err.is_conv_failed(), err.is_conv_failed());
    assert_eq!(err.is_conv_required(), err.is_conv_required());
    assert_eq!(err.is_ssl_cacert_badfile(), err.is_ssl_cacert_badfile());
    assert_eq!(err.is_ssl_crl_badfile(), err.is_ssl_crl_badfile());
    assert_eq!(err.is_ssl_shutdown_failed(), err.is_ssl_shutdown_failed());
}

#[test]
fn test_classifiers_mutually_exclusive() {
    curl::init();
    let err1 = no_url_error();
    let err2 = bad_scheme_error();

    for err in [&err1, &err2] {
        let flags = [
            err.is_recv_error(),
            err.is_ssl_certproblem(),
            err.is_ssl_cipher(),
            err.is_ssl_cacert(),
            err.is_bad_content_encoding(),
            err.is_filesize_exceeded(),
            err.is_use_ssl_failed(),
            err.is_send_fail_rewind(),
            err.is_ssl_engine_initfailed(),
            err.is_login_denied(),
            err.is_conv_failed(),
            err.is_conv_required(),
            err.is_ssl_cacert_badfile(),
            err.is_ssl_crl_badfile(),
            err.is_ssl_shutdown_failed(),
        ];
        assert_eq!(flags.len(), 15);
        let n_true = flags.iter().filter(|b| **b).count();
        assert_eq!(n_true, 0);
    }


    assert_eq!(err1.is_ssl_certproblem(), false);
    assert_eq!(err2.is_ssl_certproblem(), false);
    assert_eq!(err1.is_login_denied(), false);
    assert_eq!(err2.is_login_denied(), false);
    assert_eq!(err1.is_filesize_exceeded(), false);
    assert_eq!(err2.is_filesize_exceeded(), false);
}

#[test]
fn test_classifiers_on_localhost_connection_failure() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));
    let _ = easy.url("http://127.0.0.1:1/");

    let err = easy.perform().expect_err("connection to 127.0.0.1:1 must fail");



    assert_eq!(err.is_recv_error(), false);
    assert_eq!(err.is_ssl_certproblem(), false);
    assert_eq!(err.is_ssl_cipher(), false);
    assert_eq!(err.is_ssl_cacert(), false);
    assert_eq!(err.is_bad_content_encoding(), false);
    assert_eq!(err.is_filesize_exceeded(), false);
    assert_eq!(err.is_use_ssl_failed(), false);
    assert_eq!(err.is_send_fail_rewind(), false);
    assert_eq!(err.is_ssl_engine_initfailed(), false);
    assert_eq!(err.is_login_denied(), false);
    assert_eq!(err.is_conv_failed(), false);
    assert_eq!(err.is_conv_required(), false);
    assert_eq!(err.is_ssl_cacert_badfile(), false);
    assert_eq!(err.is_ssl_crl_badfile(), false);
    assert_eq!(err.is_ssl_shutdown_failed(), false);


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}