use curl::easy::{Auth, Easy};

fn assert_auth_method_returns_same_builder(
    auth: &mut Auth,
    apply: fn(&mut Auth, bool) -> &mut Auth,
    enabled: bool,
) {
    let original = auth as *mut Auth as *const Auth;
    let returned = apply(auth, enabled) as *mut Auth as *const Auth;
    assert_eq!(returned, original);
}

#[test]
fn auth_builder_methods_are_chainable_and_toggleable() {
    curl::init();

    let mut auth = Auth::new();

    assert_auth_method_returns_same_builder(&mut auth, Auth::basic, true);
    assert_auth_method_returns_same_builder(&mut auth, Auth::digest, true);
    assert_auth_method_returns_same_builder(&mut auth, Auth::digest_ie, true);
    assert_auth_method_returns_same_builder(&mut auth, Auth::gssnegotiate, true);
    assert_auth_method_returns_same_builder(&mut auth, Auth::ntlm, true);
    assert_auth_method_returns_same_builder(&mut auth, Auth::ntlm_wb, true);
    assert_auth_method_returns_same_builder(&mut auth, Auth::aws_sigv4, true);

    let original = &auth as *const Auth;
    let returned = auth
        .basic(false)
        .digest(false)
        .digest_ie(false)
        .gssnegotiate(false)
        .ntlm(false)
        .ntlm_wb(false)
        .aws_sigv4(false) as *mut Auth as *const Auth;

    assert_eq!(returned, original);
}

#[test]
fn configured_auth_can_be_applied_to_easy_handle_for_http_and_proxy() {
    curl::init();

    let mut handle = Easy::new();

    handle
        .url("http://example.invalid/protected")
        .expect("HTTP URL should be accepted");
    handle
        .username("user")
        .expect("HTTP username should be accepted");
    handle
        .password("password")
        .expect("HTTP password should be accepted");
    handle
        .proxy("http://proxy.example.invalid:8080")
        .expect("proxy URL should be accepted");
    handle
        .proxy_username("proxy-user")
        .expect("proxy username should be accepted");
    handle
        .proxy_password("proxy-password")
        .expect("proxy password should be accepted");

    let mut http_auth = Auth::new();
    let http_auth_returned = http_auth.basic(true).digest(true).ntlm(true) as *mut Auth as *const Auth;
    assert_eq!(http_auth_returned, &http_auth as *const Auth);

    let mut proxy_auth = Auth::new();
    let proxy_auth_returned =
        proxy_auth.digest_ie(true).gssnegotiate(true).ntlm_wb(true) as *mut Auth as *const Auth;
    assert_eq!(proxy_auth_returned, &proxy_auth as *const Auth);

    handle
        .http_auth(&http_auth)
        .expect("configured HTTP auth mask should be applicable");
    handle
        .proxy_auth(&proxy_auth)
        .expect("configured proxy auth mask should be applicable");
}

#[test]
fn aws_sigv4_auth_overrides_can_be_configured_as_dedicated_workflow() {
    curl::init();

    let mut auth = Auth::new();
    let original = &auth as *const Auth;
    let returned = auth
        .basic(true)
        .digest(true)
        .ntlm(true)
        .aws_sigv4(true) as *mut Auth as *const Auth;

    assert_eq!(returned, original);

    let mut handle = Easy::new();
    handle
        .url("https://service.us-east-1.amazonaws.com/")
        .expect("AWS-style HTTPS URL should be accepted");
    handle
        .username("access-key")
        .expect("AWS access key should be accepted as username");
    handle
        .password("secret-key")
        .expect("AWS secret key should be accepted as password");
    handle
        .http_auth(&auth)
        .expect("AWS SigV4 auth mask should be applicable");
    handle
        .aws_sigv4("aws:amz:us-east-1:service")
        .expect("AWS SigV4 provider string should be accepted");

    let encoded = handle.url_encode(b"space and/slash");
    assert_eq!(encoded, "space%20and%2Fslash");

    let decoded = handle.url_decode(&encoded);
    assert_eq!(decoded, b"space and/slash");
}

#[test]
fn auth_objects_remain_reusable_after_easy_reset() {
    curl::init();

    let mut auth = Auth::new();
    let auth_returned = auth
        .basic(true)
        .digest(true)
        .digest_ie(false)
        .gssnegotiate(false)
        .ntlm(true)
        .ntlm_wb(false)
        .aws_sigv4(false) as *mut Auth as *const Auth;
    assert_eq!(auth_returned, &auth as *const Auth);

    let mut handle = Easy::new();

    handle
        .url("http://example.invalid/first")
        .expect("first URL should be accepted");
    handle
        .http_auth(&auth)
        .expect("auth object should be applicable before reset");

    handle.reset();

    handle
        .url("http://example.invalid/second")
        .expect("second URL should be accepted after reset");
    handle
        .http_auth(&auth)
        .expect("same auth object should be reusable after reset");
}