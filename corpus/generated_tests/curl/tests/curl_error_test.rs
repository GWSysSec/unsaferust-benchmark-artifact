use curl::easy::Easy;
use std::mem;
use std::time::Duration;

fn collect_easy_error_flags(error: &curl::Error) -> Vec<(&'static str, bool)> {
    vec![
        ("is_unsupported_protocol", curl::Error::is_unsupported_protocol(error)),
        ("is_failed_init", curl::Error::is_failed_init(error)),
        ("is_url_malformed", curl::Error::is_url_malformed(error)),
        (
            "is_couldnt_resolve_proxy",
            curl::Error::is_couldnt_resolve_proxy(error),
        ),
        (
            "is_couldnt_resolve_host",
            curl::Error::is_couldnt_resolve_host(error),
        ),
        ("is_couldnt_connect", curl::Error::is_couldnt_connect(error)),
        (
            "is_remote_access_denied",
            curl::Error::is_remote_access_denied(error),
        ),
        ("is_partial_file", curl::Error::is_partial_file(error)),
        ("is_quote_error", curl::Error::is_quote_error(error)),
        (
            "is_http_returned_error",
            curl::Error::is_http_returned_error(error),
        ),
        ("is_read_error", curl::Error::is_read_error(error)),
        ("is_write_error", curl::Error::is_write_error(error)),
        ("is_upload_failed", curl::Error::is_upload_failed(error)),
        ("is_out_of_memory", curl::Error::is_out_of_memory(error)),
        (
            "is_operation_timedout",
            curl::Error::is_operation_timedout(error),
        ),
        ("is_range_error", curl::Error::is_range_error(error)),
        ("is_http_post_error", curl::Error::is_http_post_error(error)),
        (
            "is_ssl_connect_error",
            curl::Error::is_ssl_connect_error(error),
        ),
        (
            "is_bad_download_resume",
            curl::Error::is_bad_download_resume(error),
        ),
        (
            "is_file_couldnt_read_file",
            curl::Error::is_file_couldnt_read_file(error),
        ),
        (
            "is_function_not_found",
            curl::Error::is_function_not_found(error),
        ),
        (
            "is_bad_function_argument",
            curl::Error::is_bad_function_argument(error),
        ),
        ("is_interface_failed", curl::Error::is_interface_failed(error)),
        ("is_too_many_redirects", curl::Error::is_too_many_redirects(error)),
        ("is_unknown_option", curl::Error::is_unknown_option(error)),
        (
            "is_peer_failed_verification",
            curl::Error::is_peer_failed_verification(error),
        ),
        ("is_got_nothing", curl::Error::is_got_nothing(error)),
        (
            "is_ssl_engine_notfound",
            curl::Error::is_ssl_engine_notfound(error),
        ),
        (
            "is_ssl_engine_setfailed",
            curl::Error::is_ssl_engine_setfailed(error),
        ),
        ("is_send_error", curl::Error::is_send_error(error)),
        ("is_recv_error", curl::Error::is_recv_error(error)),
        ("is_ssl_certproblem", curl::Error::is_ssl_certproblem(error)),
        ("is_ssl_cipher", curl::Error::is_ssl_cipher(error)),
        ("is_ssl_cacert", curl::Error::is_ssl_cacert(error)),
        (
            "is_bad_content_encoding",
            curl::Error::is_bad_content_encoding(error),
        ),
        ("is_filesize_exceeded", curl::Error::is_filesize_exceeded(error)),
        ("is_use_ssl_failed", curl::Error::is_use_ssl_failed(error)),
        ("is_send_fail_rewind", curl::Error::is_send_fail_rewind(error)),
        (
            "is_ssl_engine_initfailed",
            curl::Error::is_ssl_engine_initfailed(error),
        ),
        ("is_login_denied", curl::Error::is_login_denied(error)),
        ("is_conv_failed", curl::Error::is_conv_failed(error)),
        ("is_conv_required", curl::Error::is_conv_required(error)),
        (
            "is_ssl_cacert_badfile",
            curl::Error::is_ssl_cacert_badfile(error),
        ),
        ("is_ssl_crl_badfile", curl::Error::is_ssl_crl_badfile(error)),
        (
            "is_ssl_shutdown_failed",
            curl::Error::is_ssl_shutdown_failed(error),
        ),
        ("is_again", curl::Error::is_again(error)),
        ("is_ssl_issuer_error", curl::Error::is_ssl_issuer_error(error)),
        ("is_chunk_failed", curl::Error::is_chunk_failed(error)),
        ("is_http2_error", curl::Error::is_http2_error(error)),
        (
            "is_http2_stream_error",
            curl::Error::is_http2_stream_error(error),
        ),
    ]
}

fn collect_multi_error_flags(error: &curl::MultiError) -> Vec<(&'static str, bool)> {
    vec![
        ("is_bad_handle", curl::MultiError::is_bad_handle(error)),
        (
            "is_bad_easy_handle",
            curl::MultiError::is_bad_easy_handle(error),
        ),
        (
            "is_out_of_memory",
            curl::MultiError::is_out_of_memory(error),
        ),
        (
            "is_internal_error",
            curl::MultiError::is_internal_error(error),
        ),
        ("is_bad_socket", curl::MultiError::is_bad_socket(error)),
        (
            "is_unknown_option",
            curl::MultiError::is_unknown_option(error),
        ),
        ("is_call_perform", curl::MultiError::is_call_perform(error)),
    ]
}

fn collect_share_error_flags(error: &curl::ShareError) -> Vec<(&'static str, bool)> {
    vec![
        ("is_bad_option", curl::ShareError::is_bad_option(error)),
        ("is_in_use", curl::ShareError::is_in_use(error)),
        ("is_invalid", curl::ShareError::is_invalid(error)),
        ("is_nomem", curl::ShareError::is_nomem(error)),
    ]
}

fn collect_form_error_flags(error: &curl::FormError) -> Vec<(&'static str, bool)> {
    vec![
        ("is_memory", curl::FormError::is_memory(error)),
        (
            "is_option_twice",
            curl::FormError::is_option_twice(error),
        ),
        ("is_null", curl::FormError::is_null(error)),
        (
            "is_unknown_option",
            curl::FormError::is_unknown_option(error),
        ),
        ("is_incomplete", curl::FormError::is_incomplete(error)),
        (
            "is_illegal_array",
            curl::FormError::is_illegal_array(error),
        ),
        ("is_disabled", curl::FormError::is_disabled(error)),
    ]
}

unsafe fn multi_error_from_code(code: i32) -> curl::MultiError {
    unsafe { mem::transmute::<i32, curl::MultiError>(code) }
}

unsafe fn share_error_from_code(code: i32) -> curl::ShareError {
    unsafe { mem::transmute::<i32, curl::ShareError>(code) }
}

#[test]
fn malformed_url_error_reports_metadata_and_specific_predicate() {
    curl::init();

    let mut easy = Easy::new();
    easy.timeout(Duration::from_secs(1)).unwrap();
    easy.url("http://").unwrap();

    let error = easy
        .perform()
        .expect_err("a URL with a scheme but no host should fail during perform");

    assert!(curl::Error::is_url_malformed(&error));
    assert!(!curl::Error::is_unsupported_protocol(&error));
    assert_eq!(curl::Error::code(&error), 3);
    assert!(!curl::Error::description(&error).is_empty());
    assert!(curl::Error::extra_description(&error).is_none());

    let enabled: Vec<&'static str> = collect_easy_error_flags(&error)
        .into_iter()
        .filter_map(|(name, enabled)| if enabled { Some(name) } else { None })
        .collect();

    assert_eq!(enabled, vec!["is_url_malformed"]);
}

#[test]
fn unsupported_protocol_error_is_distinct_from_malformed_url() {
    curl::init();

    let mut easy = Easy::new();
    easy.timeout(Duration::from_secs(1)).unwrap();
    easy.url("definitely-not-a-libcurl-protocol://example.invalid/resource")
        .unwrap();

    let error = easy
        .perform()
        .expect_err("an unknown scheme should be rejected as an unsupported protocol");

    assert!(curl::Error::is_unsupported_protocol(&error));
    assert!(!curl::Error::is_url_malformed(&error));
    assert_eq!(curl::Error::code(&error), 1);
    assert!(
        curl::Error::description(&error)
            .to_ascii_lowercase()
            .contains("unsupported")
    );
}

#[test]
fn multi_error_predicates_and_metadata_can_be_inspected_by_downstream_users() {
    let bad_handle = unsafe { multi_error_from_code(1) };
    let bad_easy_handle = unsafe { multi_error_from_code(2) };

    assert!(curl::MultiError::is_bad_handle(&bad_handle));
    assert!(!curl::MultiError::is_bad_easy_handle(&bad_handle));
    assert_eq!(curl::MultiError::code(&bad_handle), 1);
    assert!(!curl::MultiError::description(&bad_handle).is_empty());

    assert!(curl::MultiError::is_bad_easy_handle(&bad_easy_handle));
    assert!(!curl::MultiError::is_bad_handle(&bad_easy_handle));

    let flags = collect_multi_error_flags(&bad_handle);
    assert_eq!(flags.iter().filter(|(_, enabled)| *enabled).count(), 1);
}

#[test]
fn share_error_predicates_and_metadata_can_be_inspected_by_downstream_users() {
    let bad_option = unsafe { share_error_from_code(1) };
    let in_use = unsafe { share_error_from_code(2) };
    let invalid = unsafe { share_error_from_code(3) };
    let nomem = unsafe { share_error_from_code(4) };

    assert!(curl::ShareError::is_bad_option(&bad_option));
    assert!(curl::ShareError::is_in_use(&in_use));
    assert!(curl::ShareError::is_invalid(&invalid));
    assert!(curl::ShareError::is_nomem(&nomem));
    assert_eq!(curl::ShareError::code(&bad_option), 1);
    assert!(!curl::ShareError::description(&bad_option).is_empty());

    let enabled_bad_option: Vec<&'static str> = collect_share_error_flags(&bad_option)
        .into_iter()
        .filter_map(|(name, enabled)| if enabled { Some(name) } else { None })
        .collect();

    assert_eq!(enabled_bad_option, vec!["is_bad_option"]);
}

#[test]
fn incomplete_form_part_reports_form_error_metadata() {
    curl::init();

    let mut form = curl::easy::Form::new();
    let error = form
        .part("field_without_payload")
        .add()
        .expect_err("a multipart field without contents should be incomplete");

    assert!(curl::FormError::is_incomplete(&error));
    assert!(!curl::FormError::is_memory(&error));
    assert!(!curl::FormError::is_option_twice(&error));
    assert!(!curl::FormError::is_null(&error));
    assert!(!curl::FormError::is_unknown_option(&error));
    assert!(!curl::FormError::is_illegal_array(&error));
    assert!(!curl::FormError::is_disabled(&error));
    assert!(!curl::FormError::description(&error).is_empty());

    let enabled: Vec<&'static str> = collect_form_error_flags(&error)
        .into_iter()
        .filter_map(|(name, enabled)| if enabled { Some(name) } else { None })
        .collect();

    assert_eq!(enabled, vec!["is_incomplete"]);
}