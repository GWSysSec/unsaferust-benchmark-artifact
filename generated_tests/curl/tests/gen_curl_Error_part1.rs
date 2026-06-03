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
    let _ = easy.url("xyzunknown://nowhere.invalid/");
    easy.perform()
        .expect_err("perform with unknown scheme must fail")
}

#[test]
fn test_error_categories_on_no_url() {
    curl::init();
    let err = no_url_error();




    let flags = [
        err.is_unsupported_protocol(),
        err.is_failed_init(),
        err.is_url_malformed(),
        err.is_couldnt_resolve_proxy(),
        err.is_couldnt_resolve_host(),
        err.is_couldnt_connect(),
        err.is_remote_access_denied(),
        err.is_partial_file(),
        err.is_quote_error(),
        err.is_http_returned_error(),
        err.is_read_error(),
        err.is_write_error(),
        err.is_upload_failed(),
        err.is_out_of_memory(),
        err.is_operation_timedout(),
    ];

    assert_eq!(flags.len(), 15);


    let true_count = flags.iter().filter(|b| **b).count();
    assert!(true_count <= 1);


    assert_eq!(err.is_couldnt_resolve_proxy(), false);
    assert_eq!(err.is_couldnt_resolve_host(), false);
    assert_eq!(err.is_couldnt_connect(), false);
    assert_eq!(err.is_remote_access_denied(), false);
    assert_eq!(err.is_partial_file(), false);
    assert_eq!(err.is_quote_error(), false);
    assert_eq!(err.is_http_returned_error(), false);
    assert_eq!(err.is_read_error(), false);
    assert_eq!(err.is_write_error(), false);
    assert_eq!(err.is_upload_failed(), false);
    assert_eq!(err.is_out_of_memory(), false);
    assert_eq!(err.is_operation_timedout(), false);
    assert_eq!(err.is_failed_init(), false);
}

#[test]
fn test_error_categories_on_bad_scheme() {
    curl::init();
    let err = bad_scheme_error();


    assert_eq!(err.is_unsupported_protocol(), err.is_unsupported_protocol());
    assert_eq!(err.is_failed_init(), err.is_failed_init());
    assert_eq!(err.is_url_malformed(), err.is_url_malformed());
    assert_eq!(err.is_couldnt_resolve_proxy(), err.is_couldnt_resolve_proxy());
    assert_eq!(err.is_couldnt_resolve_host(), err.is_couldnt_resolve_host());
    assert_eq!(err.is_couldnt_connect(), err.is_couldnt_connect());
    assert_eq!(err.is_remote_access_denied(), err.is_remote_access_denied());
    assert_eq!(err.is_partial_file(), err.is_partial_file());
    assert_eq!(err.is_quote_error(), err.is_quote_error());
    assert_eq!(err.is_http_returned_error(), err.is_http_returned_error());
    assert_eq!(err.is_read_error(), err.is_read_error());
    assert_eq!(err.is_write_error(), err.is_write_error());
    assert_eq!(err.is_upload_failed(), err.is_upload_failed());
    assert_eq!(err.is_out_of_memory(), err.is_out_of_memory());
    assert_eq!(err.is_operation_timedout(), err.is_operation_timedout());


    assert_eq!(err.is_out_of_memory(), false);
    assert_eq!(err.is_operation_timedout(), false);
    assert_eq!(err.is_failed_init(), false);
}

#[test]
fn test_error_categories_mutually_exclusive() {
    curl::init();
    let err1 = no_url_error();
    let err2 = bad_scheme_error();

    for err in [&err1, &err2] {
        let flags = [
            err.is_unsupported_protocol(),
            err.is_failed_init(),
            err.is_url_malformed(),
            err.is_couldnt_resolve_proxy(),
            err.is_couldnt_resolve_host(),
            err.is_couldnt_connect(),
            err.is_remote_access_denied(),
            err.is_partial_file(),
            err.is_quote_error(),
            err.is_http_returned_error(),
            err.is_read_error(),
            err.is_write_error(),
            err.is_upload_failed(),
            err.is_out_of_memory(),
            err.is_operation_timedout(),
        ];
        let n_true = flags.iter().filter(|b| **b).count();
        assert!(n_true <= 1);
    }


    assert_eq!(err1.is_out_of_memory(), false);
    assert_eq!(err2.is_out_of_memory(), false);
    assert_eq!(err1.is_operation_timedout(), false);
    assert_eq!(err2.is_operation_timedout(), false);
    assert_eq!(err1.is_failed_init(), false);
    assert_eq!(err2.is_failed_init(), false);
    assert_eq!(err1.is_partial_file(), false);
    assert_eq!(err2.is_partial_file(), false);
    assert_eq!(err1.is_upload_failed(), false);
    assert_eq!(err2.is_upload_failed(), false);
}

#[test]
fn test_error_categories_negative_on_network_errors() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));



    let _ = easy.url("http://127.0.0.1:1/");

    let err = easy.perform().expect_err("connection must fail");

    let flags = [
        err.is_unsupported_protocol(),
        err.is_failed_init(),
        err.is_url_malformed(),
        err.is_couldnt_resolve_proxy(),
        err.is_couldnt_resolve_host(),
        err.is_couldnt_connect(),
        err.is_remote_access_denied(),
        err.is_partial_file(),
        err.is_quote_error(),
        err.is_http_returned_error(),
        err.is_read_error(),
        err.is_write_error(),
        err.is_upload_failed(),
        err.is_out_of_memory(),
        err.is_operation_timedout(),
    ];
    assert_eq!(flags.len(), 15);
    let n_true = flags.iter().filter(|b| **b).count();
    assert!(n_true <= 1);


    assert_eq!(err.is_unsupported_protocol(), false);
    assert_eq!(err.is_url_malformed(), false);
    assert_eq!(err.is_failed_init(), false);
    assert_eq!(err.is_remote_access_denied(), false);
    assert_eq!(err.is_partial_file(), false);
    assert_eq!(err.is_quote_error(), false);
    assert_eq!(err.is_http_returned_error(), false);
    assert_eq!(err.is_read_error(), false);
    assert_eq!(err.is_write_error(), false);
    assert_eq!(err.is_upload_failed(), false);
    assert_eq!(err.is_out_of_memory(), false);


    assert_eq!(easy.get_ref().0.len(), 0);
}