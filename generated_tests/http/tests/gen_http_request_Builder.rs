use http::header::{HeaderMap, HeaderValue};
use http::method::Method;
use http::request::Request;
use http::status::StatusCode;
use http::uri::Uri;
use http::version::Version;
use http::Extensions;

#[test]
fn test_request_builder_method_and_method_ref() {
    let builder = Request::builder().method("POST");
    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::POST);

    let builder = Request::builder().method("GET");
    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::GET);

    let builder = Request::builder().method("PUT");
    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::PUT);

    let builder = Request::builder().method("DELETE");
    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::DELETE);

    let builder = Request::builder().method("PATCH");
    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::PATCH);

    let builder = Request::builder().method("HEAD");
    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::HEAD);

    let builder = Request::builder().method("OPTIONS");
    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::OPTIONS);

    let builder = Request::builder().method("CONNECT");
    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::CONNECT);
}

#[test]
fn test_request_builder_uri_and_uri_ref() {
    let builder = Request::builder().uri("https://example.com/path?query=1");
    let uri_ref = builder.uri_ref().unwrap();
    assert_eq!(uri_ref.scheme_str(), Some("https"));
    assert_eq!(uri_ref.host(), Some("example.com"));
    assert_eq!(uri_ref.path(), "/path");
    assert_eq!(uri_ref.query(), Some("query=1"));

    let builder = Request::builder().uri("/relative/path");
    let uri_ref = builder.uri_ref().unwrap();
    assert_eq!(uri_ref.path(), "/relative/path");
    assert_eq!(uri_ref.scheme_str(), None);
    assert_eq!(uri_ref.host(), None);

    let builder = Request::builder().uri("http://localhost:8080/api");
    let uri_ref = builder.uri_ref().unwrap();
    assert_eq!(uri_ref.host(), Some("localhost"));
    assert_eq!(uri_ref.port_u16(), Some(8080));
}

#[test]
fn test_request_builder_version_and_version_ref() {
    let builder = Request::builder().version(Version::HTTP_11);
    let version_ref = builder.version_ref().unwrap();
    assert_eq!(*version_ref, Version::HTTP_11);

    let builder = Request::builder().version(Version::HTTP_10);
    let version_ref = builder.version_ref().unwrap();
    assert_eq!(*version_ref, Version::HTTP_10);

    let builder = Request::builder().version(Version::HTTP_2);
    let version_ref = builder.version_ref().unwrap();
    assert_eq!(*version_ref, Version::HTTP_2);


    let builder = Request::builder();
    let version_ref = builder.version_ref().unwrap();
    assert_eq!(*version_ref, Version::HTTP_11);


    let builder = Request::builder().version(Version::HTTP_10).version(Version::HTTP_2);
    let version_ref = builder.version_ref().unwrap();
    assert_eq!(*version_ref, Version::HTTP_2);

    let builder = Request::builder().version(Version::HTTP_11);
    assert_ne!(*builder.version_ref().unwrap(), Version::HTTP_2);
    assert_ne!(*builder.version_ref().unwrap(), Version::HTTP_10);

    let req = builder.body(()).unwrap();
    assert_eq!(req.version(), Version::HTTP_11);
}

#[test]
fn test_request_builder_header_and_headers_ref() {
    let builder = Request::builder()
        .header("Content-Type", "application/json")
        .header("X-Custom-Header", "custom-value")
        .header("Accept", "text/html");

    let headers = builder.headers_ref().unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers.get("content-type").unwrap(), "application/json");
    assert_eq!(headers.get("x-custom-header").unwrap(), "custom-value");
    assert_eq!(headers.get("accept").unwrap(), "text/html");
    assert!(headers.contains_key("content-type"));
    assert!(headers.contains_key("x-custom-header"));
    assert!(headers.contains_key("accept"));
    assert!(!headers.contains_key("authorization"));
    assert!(!headers.is_empty());
}

#[test]
fn test_request_builder_headers_mut() {
    let mut builder = Request::builder()
        .header("X-Initial", "initial-value");

    {
        let headers = builder.headers_mut().unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("x-initial").unwrap(), "initial-value");

        headers.insert(
            http::header::HeaderName::from_bytes(b"x-added").unwrap(),
            HeaderValue::from_static("added-value"),
        );
        headers.insert(
            http::header::HeaderName::from_bytes(b"x-another").unwrap(),
            HeaderValue::from_static("another-value"),
        );
    }

    let headers = builder.headers_ref().unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers.get("x-initial").unwrap(), "initial-value");
    assert_eq!(headers.get("x-added").unwrap(), "added-value");
    assert_eq!(headers.get("x-another").unwrap(), "another-value");

    let req = builder.body(()).unwrap();
    assert_eq!(req.headers().len(), 3);
}

#[test]
fn test_request_builder_extension_and_extensions_ref() {
    #[derive(Debug, Clone, PartialEq)]
    struct RequestId(u64);

    #[derive(Debug, Clone, PartialEq)]
    struct TraceContext(String);

    let builder = Request::builder()
        .extension(RequestId(42))
        .extension(TraceContext("trace-abc-123".to_string()));

    let extensions = builder.extensions_ref().unwrap();
    let req_id = extensions.get::<RequestId>().unwrap();
    assert_eq!(req_id.0, 42);

    let trace = extensions.get::<TraceContext>().unwrap();
    assert_eq!(trace.0, "trace-abc-123");

    assert!(extensions.get::<u32>().is_none());
    assert!(extensions.get::<String>().is_none());

    let req = builder.body(()).unwrap();
    let ext = req.extensions();
    assert_eq!(ext.get::<RequestId>().unwrap().0, 42);
    assert_eq!(ext.get::<TraceContext>().unwrap().0, "trace-abc-123");
}

#[test]
fn test_request_builder_extensions_mut() {
    #[derive(Debug, Clone, PartialEq)]
    struct Priority(u8);

    #[derive(Debug, Clone, PartialEq)]
    struct Tag(String);

    let mut builder = Request::builder()
        .extension(Priority(1));

    {
        let extensions = builder.extensions_mut().unwrap();
        let priority = extensions.get::<Priority>().unwrap();
        assert_eq!(priority.0, 1);

        extensions.insert(Priority(5));
        extensions.insert(Tag("important".to_string()));
    }

    let extensions = builder.extensions_ref().unwrap();
    let priority = extensions.get::<Priority>().unwrap();
    assert_eq!(priority.0, 5);
    let tag = extensions.get::<Tag>().unwrap();
    assert_eq!(tag.0, "important");

    let req = builder.body(()).unwrap();
    assert_eq!(req.extensions().get::<Priority>().unwrap().0, 5);
    assert_eq!(req.extensions().get::<Tag>().unwrap().0, "important");
}

#[test]
fn test_request_builder_full_workflow() {
    #[derive(Debug, Clone, PartialEq)]
    struct Timeout(u64);

    let request = Request::builder()
        .method("POST")
        .uri("https://api.example.com/v1/users")
        .version(Version::HTTP_2)
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer token123")
        .header("X-Request-Id", "req-001")
        .extension(Timeout(30))
        .body("{ \"name\": \"test\" }")
        .unwrap();

    assert_eq!(request.method(), &Method::POST);
    assert_eq!(request.uri().host(), Some("api.example.com"));
    assert_eq!(request.uri().path(), "/v1/users");
    assert_eq!(request.uri().scheme_str(), Some("https"));
    assert_eq!(request.version(), Version::HTTP_2);
    assert_eq!(request.headers().get("content-type").unwrap(), "application/json");
    assert_eq!(request.headers().get("authorization").unwrap(), "Bearer token123");
    assert_eq!(request.headers().get("x-request-id").unwrap(), "req-001");
    assert_eq!(request.headers().len(), 3);
    assert_eq!(request.extensions().get::<Timeout>().unwrap().0, 30);
    assert_eq!(*request.body(), "{ \"name\": \"test\" }");
}

#[test]
fn test_request_builder_method_overwrite() {
    let builder = Request::builder()
        .method("GET")
        .method("POST")
        .method("PUT");

    let method_ref = builder.method_ref().unwrap();
    assert_eq!(method_ref, &Method::PUT);
    assert_ne!(method_ref, &Method::GET);
    assert_ne!(method_ref, &Method::POST);

    let req = builder.uri("/test").body(()).unwrap();
    assert_eq!(req.method(), &Method::PUT);


    let default_builder = Request::builder();
    assert_eq!(default_builder.method_ref().unwrap(), &Method::GET);

    let default_req = default_builder.uri("/").body(()).unwrap();
    assert_eq!(default_req.method(), &Method::GET);
    assert_ne!(default_req.method(), &Method::POST);
}

#[test]
fn test_request_builder_uri_overwrite() {
    let builder = Request::builder()
        .uri("http://first.com/a")
        .uri("http://second.com/b")
        .uri("http://third.com/c");

    let uri_ref = builder.uri_ref().unwrap();
    assert_eq!(uri_ref.host(), Some("third.com"));
    assert_eq!(uri_ref.path(), "/c");
    assert_ne!(uri_ref.host(), Some("first.com"));
    assert_ne!(uri_ref.host(), Some("second.com"));

    let req = builder.method("GET").body(()).unwrap();
    assert_eq!(req.uri().host(), Some("third.com"));
    assert_eq!(req.uri().path(), "/c");
    assert_eq!(req.uri().scheme_str(), Some("http"));
    assert_eq!(req.method(), &Method::GET);
}

#[test]
fn test_request_builder_multiple_same_header() {
    let builder = Request::builder()
        .header("Set-Cookie", "a=1")
        .header("Set-Cookie", "b=2")
        .header("Set-Cookie", "c=3");

    let headers = builder.headers_ref().unwrap();
    let values: Vec<&HeaderValue> = headers.get_all("set-cookie").iter().collect();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0], "a=1");
    assert_eq!(values[1], "b=2");
    assert_eq!(values[2], "c=3");

    assert_eq!(headers.keys().len(), 1);

    let req = builder.body(()).unwrap();
    let req_values: Vec<&HeaderValue> = req.headers().get_all("set-cookie").iter().collect();
    assert_eq!(req_values.len(), 3);
}

#[test]
fn test_request_builder_default_refs() {
    let builder = Request::builder();


    assert_eq!(builder.method_ref().unwrap(), &Method::GET);


    let uri_ref = builder.uri_ref().unwrap();
    assert_eq!(uri_ref.path(), "/");


    assert_eq!(*builder.version_ref().unwrap(), Version::HTTP_11);


    let headers = builder.headers_ref().unwrap();
    assert!(headers.is_empty());
    assert_eq!(headers.len(), 0);


    let extensions = builder.extensions_ref().unwrap();
    assert!(extensions.get::<u32>().is_none());
    assert!(extensions.get::<String>().is_none());
}