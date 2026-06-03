use std::collections::HashSet;

use curl::Version;

#[test]
fn version_strings_and_protocols_are_consistent() {
    curl::init();

    let version = Version::get();

    let numeric = Version::num();
    assert!(!numeric.is_empty(), "numeric libcurl version should not be empty");
    assert!(
        numeric.chars().any(|c| c.is_ascii_digit()),
        "numeric libcurl version should contain at least one digit: {:?}",
        numeric
    );

    let human = version.version();
    assert!(
        !human.trim().is_empty(),
        "human-readable libcurl version should not be empty"
    );
    assert!(
        human.chars().any(|c| c.is_ascii_digit()),
        "human-readable libcurl version should contain a version number: {:?}",
        human
    );
    assert!(
        human.is_ascii(),
        "human-readable libcurl version should be an ASCII C string: {:?}",
        human
    );

    let host = version.host();
    assert!(
        !host.trim().is_empty(),
        "libcurl host string should not be empty"
    );

    let protocols: Vec<&str> = version.protocols().collect();
    assert!(
        !protocols.is_empty(),
        "libcurl should report at least one supported protocol"
    );
    assert!(
        protocols.iter().all(|protocol| !protocol.trim().is_empty()),
        "protocol names should never be empty: {:?}",
        protocols
    );
    assert!(
        protocols.iter().all(|protocol| {
            protocol.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '+' || c == '-' || c == '.'
            })
        }),
        "libcurl protocol names should be lowercase ASCII URI-scheme identifiers: {:?}",
        protocols
    );

    let unique_protocols: HashSet<&str> = protocols.iter().copied().collect();
    assert_eq!(
        unique_protocols.len(),
        protocols.len(),
        "protocol list should not contain duplicates: {:?}",
        protocols
    );

    let protocols_again: Vec<&str> = version.protocols().collect();
    assert_eq!(
        protocols, protocols_again,
        "protocol iteration should be repeatable for the same Version value"
    );

    let _vendored: bool = version.vendored();
}

#[test]
fn feature_flags_are_callable_and_match_core_optional_version_fields() {
    curl::init();

    let version = Version::get();

    let feature_ipv6 = version.feature_ipv6();
    let feature_ssl = version.feature_ssl();
    let feature_libz = version.feature_libz();
    let feature_ntlm = version.feature_ntlm();
    let feature_gss_negotiate = version.feature_gss_negotiate();
    let feature_debug = version.feature_debug();
    let feature_spnego = version.feature_spnego();
    let feature_largefile = version.feature_largefile();
    let feature_idn = version.feature_idn();
    let feature_sspi = version.feature_sspi();
    let feature_async_dns = version.feature_async_dns();
    let feature_conv = version.feature_conv();
    let feature_tlsauth_srp = version.feature_tlsauth_srp();
    let feature_ntlm_wb = version.feature_ntlm_wb();
    let feature_unix_domain_socket = version.feature_unix_domain_socket();
    let feature_http2 = version.feature_http2();
    let feature_http3 = version.feature_http3();
    let feature_brotli = version.feature_brotli();
    let feature_altsvc = version.feature_altsvc();
    let feature_zstd = version.feature_zstd();
    let feature_unicode = version.feature_unicode();
    let feature_hsts = version.feature_hsts();
    let feature_gsasl = version.feature_gsasl();

    let feature_flags = [
        feature_ipv6,
        feature_ssl,
        feature_libz,
        feature_ntlm,
        feature_gss_negotiate,
        feature_debug,
        feature_spnego,
        feature_largefile,
        feature_idn,
        feature_sspi,
        feature_async_dns,
        feature_conv,
        feature_tlsauth_srp,
        feature_ntlm_wb,
        feature_unix_domain_socket,
        feature_http2,
        feature_http3,
        feature_brotli,
        feature_altsvc,
        feature_zstd,
        feature_unicode,
        feature_hsts,
        feature_gsasl,
    ];

    assert_eq!(
        feature_flags.len(),
        23,
        "the test should exercise every documented feature flag in this group"
    );

    assert!(
        feature_flags.iter().any(|flag| *flag),
        "at least one libcurl feature flag should be enabled"
    );

    match (feature_ssl, version.ssl_version()) {
        (true, Some(ssl)) => assert!(
            !ssl.trim().is_empty(),
            "SSL version should not be empty when SSL is enabled"
        ),
        (false, None) => {}
        (true, None) => panic!("SSL feature should report an SSL backend version"),
        (false, Some(ssl)) => panic!(
            "SSL version should be absent when SSL support is disabled, got {:?}",
            ssl
        ),
    }

    match (feature_libz, version.libz_version()) {
        (true, Some(libz)) => assert!(
            !libz.trim().is_empty(),
            "libz version should not be empty when libz is enabled"
        ),
        (false, None) => {}
        (true, None) => panic!("libz feature should report a libz version"),
        (false, Some(libz)) => panic!(
            "libz version should be absent when libz support is disabled, got {:?}",
            libz
        ),
    }

    if feature_brotli {
        let brotli_text_is_present = version
            .brotli_version()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let brotli_num_is_present = version
            .brotli_version_num()
            .map(|num| num != 0)
            .unwrap_or(false);

        assert!(
            brotli_text_is_present || brotli_num_is_present,
            "brotli support should expose a non-empty human-readable or non-zero numeric brotli version"
        );
    } else {
        assert!(
            version.brotli_version().is_none()
                && version
                    .brotli_version_num()
                    .map(|num| num == 0)
                    .unwrap_or(true),
            "brotli version fields should be absent or zero when brotli support is disabled"
        );
    }

    if feature_http2 {
        let nghttp2_text_is_present = version
            .nghttp2_version()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let nghttp2_num_is_present = version
            .nghttp2_version_num()
            .map(|num| num != 0)
            .unwrap_or(false);

        assert!(
            nghttp2_text_is_present || nghttp2_num_is_present,
            "HTTP/2 support should expose a non-empty human-readable or non-zero numeric nghttp2 version"
        );
    } else {
        assert!(
            version.nghttp2_version().is_none()
                && version
                    .nghttp2_version_num()
                    .map(|num| num == 0)
                    .unwrap_or(true),
            "nghttp2 version fields should be absent or zero when HTTP/2 support is disabled"
        );
    }

    if feature_zstd {
        let zstd_text_is_present = version
            .zstd_version()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let zstd_num_is_present = version.zstd_ver_num().map(|num| num != 0).unwrap_or(false);

        assert!(
            zstd_text_is_present || zstd_num_is_present,
            "zstd support should expose a non-empty human-readable or non-zero numeric zstd version"
        );
    } else {
        assert!(
            version.zstd_version().is_none()
                && version.zstd_ver_num().map(|num| num == 0).unwrap_or(true),
            "zstd version fields should be absent or zero when zstd support is disabled"
        );
    }

    if feature_gsasl {
        let gsasl = version
            .gsasl_version()
            .expect("GSASL feature should report a gsasl version");
        assert!(
            !gsasl.trim().is_empty(),
            "gsasl version should not be empty when gsasl is enabled"
        );
    } else {
        assert!(
            version.gsasl_version().is_none(),
            "gsasl version should be absent when GSASL support is disabled"
        );
    }
}

#[test]
fn optional_dependency_versions_are_well_formed_when_present() {
    curl::init();

    let version = Version::get();

    if let Some(ssl) = version.ssl_version() {
        assert!(!ssl.trim().is_empty(), "SSL version should not be blank");
    }

    if let Some(libz) = version.libz_version() {
        assert!(!libz.trim().is_empty(), "libz version should not be blank");
    }

    let ares_version = version.ares_version();
    let ares_version_num = version.ares_version_num();
    if let Some(ares) = ares_version {
        assert!(!ares.trim().is_empty(), "ares version should not be blank");
    }
    if let Some(num) = ares_version_num {
        if ares_version.is_some() {
            assert_ne!(
                num, 0,
                "ares numeric version should be non-zero when an ares version string is present"
            );
        }
    }

    if let Some(libidn) = version.libidn_version() {
        assert!(
            !libidn.trim().is_empty(),
            "libidn version should not be blank"
        );
    }

    if let Some(num) = version.iconv_version_num() {
        assert!(
            num == 0 || version.feature_conv(),
            "iconv numeric version should be zero unless iconv conversion support is enabled; got {num}"
        );
    }

    if let Some(libssh) = version.libssh_version() {
        assert!(
            !libssh.trim().is_empty(),
            "libssh version should not be blank"
        );
    }

    let brotli_version = version.brotli_version();
    let brotli_version_num = version.brotli_version_num();
    if let Some(brotli) = brotli_version {
        assert!(
            !brotli.trim().is_empty(),
            "brotli version should not be blank"
        );
    }
    if let Some(num) = brotli_version_num {
        if version.feature_brotli() || brotli_version.is_some() {
            assert_ne!(
                num, 0,
                "brotli numeric version should be non-zero when brotli support is enabled or a brotli version string is present"
            );
        } else {
            assert_eq!(
                num, 0,
                "brotli numeric version should be zero when brotli support is disabled"
            );
        }
    }

    let nghttp2_version = version.nghttp2_version();
    let nghttp2_version_num = version.nghttp2_version_num();
    if let Some(nghttp2) = nghttp2_version {
        assert!(
            !nghttp2.trim().is_empty(),
            "nghttp2 version should not be blank"
        );
    }
    if let Some(num) = nghttp2_version_num {
        if version.feature_http2() || nghttp2_version.is_some() {
            assert_ne!(
                num, 0,
                "nghttp2 numeric version should be non-zero when HTTP/2 support is enabled or an nghttp2 version string is present"
            );
        } else {
            assert_eq!(
                num, 0,
                "nghttp2 numeric version should be zero when HTTP/2 support is disabled"
            );
        }
    }

    if let Some(quic) = version.quic_version() {
        assert!(!quic.trim().is_empty(), "quic version should not be blank");
    }

    let zstd_version = version.zstd_version();
    let zstd_version_num = version.zstd_ver_num();
    if let Some(zstd) = zstd_version {
        assert!(!zstd.trim().is_empty(), "zstd version should not be blank");
    }
    if let Some(num) = zstd_version_num {
        if version.feature_zstd() || zstd_version.is_some() {
            assert_ne!(
                num, 0,
                "zstd numeric version should be non-zero when zstd support is enabled or a zstd version string is present"
            );
        } else {
            assert_eq!(
                num, 0,
                "zstd numeric version should be zero when zstd support is disabled"
            );
        }
    }

    if let Some(hyper) = version.hyper_version() {
        assert!(
            !hyper.trim().is_empty(),
            "hyper version should not be blank"
        );
    }

    if let Some(gsasl) = version.gsasl_version() {
        assert!(
            !gsasl.trim().is_empty(),
            "gsasl version should not be blank"
        );
    }
}