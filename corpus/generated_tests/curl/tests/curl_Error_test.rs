use curl::easy::Easy;

fn all_error_flags(error: &curl::Error) -> Vec<(&'static str, bool)> {
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
        (
            "is_too_many_redirects",
            curl::Error::is_too_many_redirects(error),
        ),
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
        (
            "is_filesize_exceeded",
            curl::Error::is_filesize_exceeded(error),
        ),
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

#[test]
fn malformed_url_error_exposes_expected_predicate_and_descriptions() {
    curl::init();

    let mut easy = Easy::new();

    let error = match easy.url("http://[::1") {
        Ok(()) => easy
            .perform()
            .expect_err("an unterminated IPv6 host literal should fail as a malformed URL"),
        Err(error) => error,
    };

    assert!(
        curl::Error::is_url_malformed(&error),
        "expected URL_MALFORMAT, got code {:?} with description {:?}",
        curl::Error::code(&error),
        curl::Error::description(&error)
    );
    assert!(
        !curl::Error::is_unsupported_protocol(&error),
        "malformed URL should not be classified as an unsupported protocol"
    );
    assert!(
        !curl::Error::is_operation_timedout(&error),
        "local URL parsing failure should not be classified as a timeout"
    );
    assert!(
        !curl::Error::description(&error).is_empty(),
        "libcurl should provide a non-empty general description"
    );
    assert!(
        !format!("{:?}", curl::Error::code(&error)).is_empty(),
        "the underlying CURLcode should be available and printable"
    );

    let flags = all_error_flags(&error);
    let enabled: Vec<&str> = flags
        .iter()
        .filter_map(|(name, enabled)| enabled.then_some(*name))
        .collect();

    assert!(
        enabled.contains(&"is_url_malformed"),
        "all predicates were exercised and should include is_url_malformed; enabled={:?}",
        enabled
    );
    assert_eq!(
        enabled.iter().filter(|name| **name == "is_url_malformed").count(),
        1,
        "the malformed URL predicate should be reported exactly once"
    );

    let extra = curl::Error::extra_description(&error);
    if let Some(extra) = extra {
        assert!(
            !extra.is_empty(),
            "an extra error description, when present, should be non-empty"
        );
    }
}

#[test]
fn unsupported_protocol_from_perform_is_distinct_from_malformed_url(
) -> Result<(), Box<dyn std::error::Error>> {
    curl::init();

    let url = "nosuchprotocol://example.invalid/resource";

    let mut easy = Easy::new();
    let error = match easy.url(url) {
        Ok(()) => {
            easy.connect_timeout(std::time::Duration::from_secs(1))?;
            easy.timeout(std::time::Duration::from_secs(2))?;

            assert_eq!(easy.effective_url()?, Some(url));

            easy.perform()
                .expect_err("performing an unknown URL scheme should fail before any network access")
        }
        Err(error) => error,
    };

    assert!(
        curl::Error::is_unsupported_protocol(&error),
        "expected unsupported protocol, got {:?}: {}",
        curl::Error::code(&error),
        curl::Error::description(&error)
    );
    assert!(
        !curl::Error::is_url_malformed(&error),
        "a syntactically valid unknown scheme should not be classified as malformed"
    );
    assert!(
        !curl::Error::is_couldnt_resolve_host(&error),
        "unsupported protocol should be detected before host resolution"
    );
    assert!(
        !curl::Error::is_couldnt_connect(&error),
        "unsupported protocol should be detected before connecting"
    );
    assert!(
        !curl::Error::description(&error).is_empty(),
        "unsupported protocol errors should have a useful description"
    );

    let enabled: Vec<&str> = all_error_flags(&error)
        .into_iter()
        .filter_map(|(name, enabled)| enabled.then_some(name))
        .collect();

    assert!(
        enabled.contains(&"is_unsupported_protocol"),
        "all predicates were exercised and should include is_unsupported_protocol; enabled={:?}",
        enabled
    );

    Ok(())
}