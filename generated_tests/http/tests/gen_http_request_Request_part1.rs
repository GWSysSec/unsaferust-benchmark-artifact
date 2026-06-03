use http::{Request, Method, Version, StatusCode};
use http::header::{HeaderMap, HeaderValue};
use http::uri::Uri;

#[test]
fn test_request_put_builder() {
    let uri_str = "https://api.example.com/resources/42";
    let req = Request::put(uri_str)
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer token123")
        .body("{ \"name\": \"updated\" }")
        .unwrap();

    assert_eq!(req.method(), &Method::PUT);
    assert_eq!(req.uri(), uri_str);
    assert_eq!(req.version(), Version::HTTP_11);
    assert_eq!(req.headers().len(), 2);
    assert_eq!(
        req.headers().get("Content-Type").unwrap(),
        HeaderValue::from_static("application/json")
    );
    assert_eq!(
        req.headers().get("Authorization").unwrap(),
        HeaderValue::from_static("Bearer token123")
    );
    assert_eq!(*req.body(), "{ \"name\": \"updated\" }");
    assert_ne!(req.method(), &Method::POST);
}

#[test]
fn test_request_post_builder() {
    let uri_str = "https://api.example.com/resources";
    let req = Request::post(uri_str)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "text/html")
        .header("X-Request-Id", "abc-123")
        .body(Vec::<u8>::new())
        .unwrap();

    assert_eq!(req.method(), &Method::POST);
    assert_eq!(req.uri(), uri_str);
    assert_eq!(req.headers().len(), 3);
    assert_eq!(
        req.headers().get("Content-Type").unwrap(),
        "application/x-www-form-urlencoded"
    );
    assert_eq!(req.headers().get("Accept").unwrap(), "text/html");
    assert_eq!(req.headers().get("X-Request-Id").unwrap(), "abc-123");
    assert!(req.body().is_empty());
    assert_eq!(req.version(), Version::HTTP_11);
}

#[test]
fn test_request_delete_builder() {
    let uri_str = "https://api.example.com/resources/99";
    let req = Request::delete(uri_str)
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .body(())
        .unwrap();

    assert_eq!(req.method(), &Method::DELETE);
    assert_eq!(req.uri(), uri_str);
    assert_eq!(req.headers().len(), 1);
    assert_eq!(
        req.headers().get("Authorization").unwrap(),
        "Basic dXNlcjpwYXNz"
    );
    assert_ne!(req.method(), &Method::GET);
    assert_ne!(req.method(), &Method::PUT);
    assert_eq!(req.version(), Version::HTTP_11);
    assert_eq!(*req.body(), ());
}

#[test]
fn test_request_options_builder() {
    let uri_str = "https://api.example.com/resources";
    let req = Request::options(uri_str)
        .header("Origin", "https://example.com")
        .header("Access-Control-Request-Method", "POST")
        .body(())
        .unwrap();

    assert_eq!(req.method(), &Method::OPTIONS);
    assert_eq!(req.uri(), uri_str);
    assert_eq!(req.headers().len(), 2);
    assert_eq!(req.headers().get("Origin").unwrap(), "https://example.com");
    assert_eq!(
        req.headers().get("Access-Control-Request-Method").unwrap(),
        "POST"
    );
    assert_ne!(req.method(), &Method::GET);
    assert_eq!(req.version(), Version::HTTP_11);
    assert_eq!(*req.body(), ());
}

#[test]
fn test_request_head_builder() {
    let uri_str = "https://api.example.com/status";
    let req = Request::head(uri_str)
        .header("Accept", "*/*")
        .body(())
        .unwrap();

    assert_eq!(req.method(), &Method::HEAD);
    assert_eq!(req.uri(), uri_str);
    assert_eq!(req.headers().len(), 1);
    assert_eq!(req.headers().get("Accept").unwrap(), "*/*");
    assert_ne!(req.method(), &Method::GET);
    assert_ne!(req.method(), &Method::OPTIONS);
    assert_eq!(req.version(), Version::HTTP_11);
    assert_eq!(*req.body(), ());
}

#[test]
fn test_request_connect_builder() {
    let uri_str = "proxy.example.com:443";
    let req = Request::connect(uri_str)
        .header("Host", "proxy.example.com:443")
        .header("Proxy-Authorization", "Basic cHJveHk6cGFzcw==")
        .body(())
        .unwrap();

    assert_eq!(req.method(), &Method::CONNECT);
    assert_eq!(req.uri(), uri_str);
    assert_eq!(req.headers().len(), 2);
    assert_eq!(
        req.headers().get("Host").unwrap(),
        "proxy.example.com:443"
    );
    assert_eq!(
        req.headers().get("Proxy-Authorization").unwrap(),
        "Basic cHJveHk6cGFzcw=="
    );
    assert_ne!(req.method(), &Method::GET);
    assert_eq!(req.version(), Version::HTTP_11);
    assert_eq!(*req.body(), ());
}

#[test]
fn test_request_patch_builder() {
    let uri_str = "https://api.example.com/resources/7";
    let req = Request::patch(uri_str)
        .header("Content-Type", "application/json-patch+json")
        .header("If-Match", "\"etag-value\"")
        .body("[ { \"op\": \"replace\", \"path\": \"/name\", \"value\": \"new\" } ]")
        .unwrap();

    assert_eq!(req.method(), &Method::PATCH);
    assert_eq!(req.uri(), uri_str);
    assert_eq!(req.headers().len(), 2);
    assert_eq!(
        req.headers().get("Content-Type").unwrap(),
        "application/json-patch+json"
    );
    assert_eq!(req.headers().get("If-Match").unwrap(), "\"etag-value\"");
    assert_ne!(req.method(), &Method::PUT);
    assert_ne!(req.method(), &Method::POST);
    assert!(!req.body().is_empty());
}

#[test]
fn test_request_trace_builder() {
    let uri_str = "https://api.example.com/debug";
    let req = Request::trace(uri_str)
        .header("Max-Forwards", "5")
        .body(())
        .unwrap();

    assert_eq!(req.method(), &Method::TRACE);
    assert_eq!(req.uri(), uri_str);
    assert_eq!(req.headers().len(), 1);
    assert_eq!(req.headers().get("Max-Forwards").unwrap(), "5");
    assert_ne!(req.method(), &Method::GET);
    assert_ne!(req.method(), &Method::OPTIONS);
    assert_eq!(req.version(), Version::HTTP_11);
    assert_eq!(*req.body(), ());
}

#[test]
fn test_request_method_and_method_mut() {
    let mut req = Request::post("https://example.com/submit")
        .body("data")
        .unwrap();

    assert_eq!(req.method(), &Method::POST);
    assert_ne!(req.method(), &Method::GET);

    *req.method_mut() = Method::PUT;
    assert_eq!(req.method(), &Method::PUT);
    assert_ne!(req.method(), &Method::POST);

    *req.method_mut() = Method::DELETE;
    assert_eq!(req.method(), &Method::DELETE);

    *req.method_mut() = Method::GET;
    assert_eq!(req.method(), &Method::GET);
}

#[test]
fn test_request_uri_and_uri_mut() {
    let original_uri = "https://example.com/original";
    let new_uri_str = "https://other.example.com/new-path?key=value";

    let mut req = Request::get(original_uri)
        .body(())
        .unwrap();

    assert_eq!(req.uri(), original_uri);
    assert_eq!(req.uri().host(), Some("example.com"));
    assert_eq!(req.uri().path(), "/original");

    *req.uri_mut() = Uri::from_static(new_uri_str);
    assert_eq!(req.uri(), new_uri_str);
    assert_eq!(req.uri().host(), Some("other.example.com"));
    assert_eq!(req.uri().path(), "/new-path");
    assert_eq!(req.uri().query(), Some("key=value"));
    assert_ne!(req.uri(), original_uri);
}

#[test]
fn test_request_version_and_version_mut() {
    let mut req = Request::get("https://example.com/")
        .body(())
        .unwrap();

    assert_eq!(req.version(), Version::HTTP_11);
    assert_ne!(req.version(), Version::HTTP_2);

    *req.version_mut() = Version::HTTP_2;
    assert_eq!(req.version(), Version::HTTP_2);
    assert_ne!(req.version(), Version::HTTP_11);

    *req.version_mut() = Version::HTTP_10;
    assert_eq!(req.version(), Version::HTTP_10);
    assert_ne!(req.version(), Version::HTTP_2);

    *req.version_mut() = Version::HTTP_11;
    assert_eq!(req.version(), Version::HTTP_11);
}

#[test]
fn test_request_headers_accessor() {
    let req = Request::post("https://example.com/api")
        .header("Content-Type", "application/json")
        .header("X-Custom-One", "value1")
        .header("X-Custom-Two", "value2")
        .header("Authorization", "Bearer xyz")
        .body(())
        .unwrap();

    let headers: &HeaderMap<HeaderValue> = req.headers();
    assert_eq!(headers.len(), 4);
    assert!(headers.contains_key("Content-Type"));
    assert!(headers.contains_key("X-Custom-One"));
    assert!(headers.contains_key("X-Custom-Two"));
    assert!(headers.contains_key("Authorization"));
    assert!(!headers.contains_key("X-Nonexistent"));
    assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
    assert_eq!(headers.get("X-Custom-One").unwrap(), "value1");
    assert_eq!(headers.get("Authorization").unwrap(), "Bearer xyz");
}

#[test]
fn test_request_all_methods_workflow() {
    let base_uri = "https://api.example.com/items";

    let get_req = Request::builder()
        .method(Method::GET)
        .uri(base_uri)
        .body(())
        .unwrap();
    let post_req = Request::post(base_uri).body("new item").unwrap();
    let put_req = Request::put("https://api.example.com/items/1").body("updated").unwrap();
    let patch_req = Request::patch("https://api.example.com/items/1").body("partial").unwrap();
    let delete_req = Request::delete("https://api.example.com/items/1").body(()).unwrap();
    let head_req = Request::head(base_uri).body(()).unwrap();
    let options_req = Request::options(base_uri).body(()).unwrap();
    let trace_req = Request::trace(base_uri).body(()).unwrap();

    assert_eq!(get_req.method(), &Method::GET);
    assert_eq!(post_req.method(), &Method::POST);
    assert_eq!(put_req.method(), &Method::PUT);
    assert_eq!(patch_req.method(), &Method::PATCH);
    assert_eq!(delete_req.method(), &Method::DELETE);
    assert_eq!(head_req.method(), &Method::HEAD);
    assert_eq!(options_req.method(), &Method::OPTIONS);
    assert_eq!(trace_req.method(), &Method::TRACE);
}

#[test]
fn test_request_mutation_workflow() {
    let mut req = Request::get("https://example.com/start")
        .header("Accept", "text/html")
        .body("initial body")
        .unwrap();


    assert_eq!(req.method(), &Method::GET);
    assert_eq!(req.uri().path(), "/start");
    assert_eq!(req.version(), Version::HTTP_11);
    assert_eq!(req.headers().get("Accept").unwrap(), "text/html");


    *req.method_mut() = Method::POST;
    assert_eq!(req.method(), &Method::POST);


    *req.uri_mut() = Uri::from_static("https://example.com/end?done=true");
    assert_eq!(req.uri().path(), "/end");
    assert_eq!(req.uri().query(), Some("done=true"));


    *req.version_mut() = Version::HTTP_2;
    assert_eq!(req.version(), Version::HTTP_2);


    assert_eq!(req.headers().get("Accept").unwrap(), "text/html");
}