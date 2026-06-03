use curl::easy::{Easy, SslOpt};

#[test]
fn ssl_options_builder_methods_chain_on_the_same_value_and_apply_to_easy(
) -> Result<(), Box<dyn std::error::Error>> {
    curl::init();

    let mut opts = SslOpt::new();
    let opts_addr = &mut opts as *mut SslOpt;

    {
        let returned = SslOpt::no_revoke(&mut opts, true);
        assert_eq!(returned as *mut SslOpt, opts_addr);

        let returned = SslOpt::allow_beast(returned, true);
        assert_eq!(returned as *mut SslOpt, opts_addr);

        let returned = SslOpt::no_revoke(returned, false);
        assert_eq!(returned as *mut SslOpt, opts_addr);

        let returned = SslOpt::allow_beast(returned, false);
        assert_eq!(returned as *mut SslOpt, opts_addr);
    }

    let mut easy = Easy::new();
    assert!(!easy.raw().is_null());

    let url = "https://example.invalid/ssl-options-test";
    easy.url(url)?;
    easy.ssl_options(&opts)?;
    easy.ssl_verify_peer(false)?;
    easy.ssl_verify_host(false)?;

    assert_eq!(easy.effective_url()?, Some(url));
    assert_eq!(easy.effective_url_bytes()?, Some(url.as_bytes()));

    Ok(())
}

#[test]
fn ssl_options_remain_reusable_across_reset_and_other_easy_workflows(
) -> Result<(), Box<dyn std::error::Error>> {
    curl::init();

    let mut opts = SslOpt::new();
    SslOpt::no_revoke(&mut opts, true)
        .allow_beast(true)
        .no_revoke(false)
        .allow_beast(true);

    let mut easy = Easy::new();

    let first_url = "https://example.invalid/first";
    easy.url(first_url)?;
    easy.ssl_options(&opts)?;
    assert_eq!(easy.effective_url()?, Some(first_url));
    assert_eq!(easy.effective_url_bytes()?, Some(first_url.as_bytes()));

    easy.reset();

    let second_url = "https://example.invalid/second%20path?q=rust%20curl";
    easy.url(second_url)?;
    easy.ssl_options(&opts)?;
    easy.useragent("curl-rust-integration-test/1.0")?;
    easy.follow_location(true)?;
    easy.max_redirections(3)?;

    let raw = b"second path?q=rust curl&x=1";
    let encoded = easy.url_encode(raw);
    assert_eq!(encoded, "second%20path%3Fq%3Drust%20curl%26x%3D1");

    let decoded = easy.url_decode(&encoded);
    assert_eq!(decoded, raw);

    assert_eq!(easy.effective_url()?, Some(second_url));
    assert_eq!(easy.effective_url_bytes()?, Some(second_url.as_bytes()));
    assert_eq!(easy.redirect_count()?, 0);

    Ok(())
}