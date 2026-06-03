use curl::easy::{Auth, Easy2, Handler, NetRc, WriteError};

struct Collector {
    data: Vec<u8>,
    calls: usize,
}

impl Collector {
    fn new() -> Self {
        Collector {
            data: Vec::new(),
            calls: 0,
        }
    }
}

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.data.extend_from_slice(data);
        self.calls += 1;
        Ok(data.len())
    }
}

#[test]
fn test_easy2_auth_options() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);


    assert!(easy.password("hunter2").is_ok());
    assert!(easy.password("").is_ok());
    assert!(easy
        .password("a-very-long-password-!@#$%^&*()_+=-[]{};':\",./<>?")
        .is_ok());


    let auth_default = Auth::new();
    assert!(easy.http_auth(&auth_default).is_ok());


    let auth_second = Auth::new();
    assert!(easy.http_auth(&auth_second).is_ok());


    assert!(easy.unrestricted_auth(true).is_ok());
    assert!(easy.unrestricted_auth(false).is_ok());
    assert!(easy.unrestricted_auth(true).is_ok());



    let _aws_result = easy.aws_sigv4("aws:amz:us-east-1:s3");


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);
}

#[test]
fn test_easy2_proxy_auth_options() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);


    assert!(easy.proxy_username("proxy_user").is_ok());
    assert!(easy.proxy_username("").is_ok());
    assert!(easy.proxy_username("admin@corp.example").is_ok());


    assert!(easy.proxy_password("proxy_pass").is_ok());
    assert!(easy.proxy_password("").is_ok());
    assert!(easy.proxy_password("p@$$w0rd!#%").is_ok());


    let pauth_a = Auth::new();
    assert!(easy.proxy_auth(&pauth_a).is_ok());

    let pauth_b = Auth::new();
    assert!(easy.proxy_auth(&pauth_b).is_ok());


    assert!(easy.proxy_username("again").is_ok());
    assert!(easy.proxy_password("again").is_ok());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);
}

#[test]
fn test_easy2_redirects_and_referer() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);


    assert!(easy.autoreferer(true).is_ok());
    assert!(easy.autoreferer(false).is_ok());
    assert!(easy.autoreferer(true).is_ok());


    assert!(easy.max_redirections(0).is_ok());
    assert!(easy.max_redirections(1).is_ok());
    assert!(easy.max_redirections(5).is_ok());
    assert!(easy.max_redirections(50).is_ok());
    assert!(easy.max_redirections(u32::MAX).is_ok());


    assert!(easy.autoreferer(false).is_ok());
    assert!(easy.max_redirections(7).is_ok());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);
}

#[test]
fn test_easy2_cookie_options() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);

    let tmp_dir = std::env::temp_dir();
    let cookie_path = tmp_dir.join("curl_integration_test_cookies.txt");


    assert!(easy.cookie_jar(&cookie_path).is_ok());
    assert!(easy.cookie_jar(cookie_path.as_path()).is_ok());


    assert!(easy.cookie_jar("/tmp/curl_integration_test_cookies2.txt").is_ok());


    assert!(easy.cookie_session(true).is_ok());
    assert!(easy.cookie_session(false).is_ok());
    assert!(easy.cookie_session(true).is_ok());


    assert!(easy
        .cookie_list("Set-Cookie: foo=bar; path=/; domain=example.com")
        .is_ok());
    assert!(easy.cookie_list("ALL").is_ok());
    assert!(easy.cookie_list("SESS").is_ok());
    assert!(easy.cookie_list("FLUSH").is_ok());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);
}

#[test]
fn test_easy2_misc_options() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);


    assert!(easy.netrc(NetRc::Ignored).is_ok());
    assert!(easy.netrc(NetRc::Optional).is_ok());
    assert!(easy.netrc(NetRc::Required).is_ok());
    assert!(easy.netrc(NetRc::Ignored).is_ok());


    assert!(easy.transfer_encoding(true).is_ok());
    assert!(easy.transfer_encoding(false).is_ok());
    assert!(easy.transfer_encoding(true).is_ok());


    assert!(easy.ignore_content_length(true).is_ok());
    assert!(easy.ignore_content_length(false).is_ok());
    assert!(easy.ignore_content_length(true).is_ok());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);
}

#[test]
fn test_easy2_combined_workflow() {
    curl::init();
    let mut easy = Easy2::new(Collector::new());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);


    assert!(easy.password("secret").is_ok());
    assert!(easy.proxy_username("upstream").is_ok());
    assert!(easy.proxy_password("upstream-secret").is_ok());

    let auth = Auth::new();
    assert!(easy.http_auth(&auth).is_ok());

    let pauth = Auth::new();
    assert!(easy.proxy_auth(&pauth).is_ok());

    assert!(easy.netrc(NetRc::Optional).is_ok());
    assert!(easy.unrestricted_auth(false).is_ok());
    assert!(easy.autoreferer(true).is_ok());
    assert!(easy.max_redirections(3).is_ok());
    assert!(easy.transfer_encoding(true).is_ok());
    assert!(easy.ignore_content_length(false).is_ok());

    let cookie_file = std::env::temp_dir().join("curl_combined_cookies.txt");
    assert!(easy.cookie_jar(&cookie_file).is_ok());
    assert!(easy.cookie_session(true).is_ok());
    assert!(easy.cookie_list("ALL").is_ok());


    assert!(easy.password("changed").is_ok());
    assert!(easy.max_redirections(0).is_ok());
    assert!(easy.autoreferer(false).is_ok());


    assert_eq!(easy.get_ref().data.len(), 0);
    assert_eq!(easy.get_ref().calls, 0);
}