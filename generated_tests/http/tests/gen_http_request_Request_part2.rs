
use http::{Request, HeaderMap, StatusCode, Method, Version};
use http::header::{HeaderValue, HeaderName, Entry};
use http::uri::Uri;

#[test]
fn test_headers_mut_insert_and_modify() {
    let mut request = Request::builder()
        .uri("https://example.com/path")
        .method("POST")
        .header("Content-Type", "application/json")
        .body("initial body")
        .unwrap();


    assert_eq!(
        request.headers().get("content-type").unwrap(),
        &HeaderValue::from_static("application/json")
    );
    assert_eq!(request.headers().len(), 1);


    let headers = request.headers_mut();
    headers.insert(
        HeaderName::from_bytes(b"x-request-id").unwrap(),
        HeaderValue::from_static("abc-123"),
    );
    headers.insert(
        HeaderName::from_bytes(b"authorization").unwrap(),
        HeaderValue::from_static("Bearer token123"),
    );


    assert_eq!(request.headers().len(), 3);
    assert_eq!(
        request.headers().get("x-request-id").unwrap(),
        &HeaderValue::from_static("abc-123")
    );
    assert_eq!(
        request.headers().get("authorization").unwrap(),
        &HeaderValue::from_static("Bearer token123")
    );
    assert_eq!(
        request.headers().get("content-type").unwrap(),
        &HeaderValue::from_static("application/json")
    );


    let headers = request.headers_mut();
    headers.remove("content-type");
    assert_eq!(request.headers().len(), 2);
    assert!(request.headers().get("content-type").is_none());
}

#[test]
fn test_headers_mut_append_multiple_values() {
    let mut request = Request::builder()
        .uri("https://example.com/")
        .body(())
        .unwrap();

    assert_eq!(request.headers().len(), 0);

    let headers = request.headers_mut();
    let name = HeaderName::from_bytes(b"set-cookie").unwrap();
    headers.append(name.clone(), HeaderValue::from_static("a=1"));
    headers.append(name.clone(), HeaderValue::from_static("b=2"));
    headers.append(name.clone(), HeaderValue::from_static("c=3"));


    assert_eq!(request.headers().len(), 3);

    let values: Vec<&HeaderValue> = request.headers().get_all("set-cookie").iter().collect();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0], &HeaderValue::from_static("a=1"));
    assert_eq!(values[1], &HeaderValue::from_static("b=2"));
    assert_eq!(values[2], &HeaderValue::from_static("c=3"));
}

#[test]
fn test_extensions_empty_and_read() {
    let request = Request::builder()
        .uri("https://example.com/resource")
        .body("hello")
        .unwrap();

    let ext = request.extensions();

    assert!(ext.get::<u32>().is_none());
    assert!(ext.get::<String>().is_none());
    assert!(ext.get::<Vec<u8>>().is_none());
    assert!(ext.get::<bool>().is_none());


    assert_eq!(request.uri().path(), "/resource");
    assert_eq!(request.method(), Method::GET);
    assert_eq!(*request.body(), "hello");
    assert_eq!(request.uri().host().unwrap(), "example.com");
}

#[test]
fn test_extensions_mut_insert_and_retrieve() {
    #[derive(Debug, Clone, PartialEq)]
    struct RequestId(u64);

    #[derive(Debug, Clone, PartialEq)]
    struct Timestamp(i64);

    let mut request = Request::builder()
        .uri("https://api.example.com/v1/users")
        .method("PUT")
        .body(Vec::<u8>::new())
        .unwrap();


    assert!(request.extensions().get::<RequestId>().is_none());
    assert!(request.extensions().get::<Timestamp>().is_none());


    request.extensions_mut().insert(RequestId(42));
    request.extensions_mut().insert(Timestamp(1700000000));


    assert_eq!(request.extensions().get::<RequestId>(), Some(&RequestId(42)));
    assert_eq!(request.extensions().get::<Timestamp>(), Some(&Timestamp(1700000000)));


    request.extensions_mut().insert(RequestId(99));
    assert_eq!(request.extensions().get::<RequestId>(), Some(&RequestId(99)));
    assert_ne!(request.extensions().get::<RequestId>(), Some(&RequestId(42)));


    let removed = request.extensions_mut().remove::<Timestamp>();
    assert_eq!(removed, Some(Timestamp(1700000000)));
    assert!(request.extensions().get::<Timestamp>().is_none());
}

#[test]
fn test_body_mut_modify_string_body() {
    let mut request = Request::builder()
        .uri("https://example.com/submit")
        .method("POST")
        .header("Content-Type", "text/plain")
        .body(String::from("original content"))
        .unwrap();


    assert_eq!(request.body(), "original content");
    assert_eq!(request.body().len(), 16);


    let body = request.body_mut();
    body.push_str(" with additions");


    assert_eq!(request.body(), "original content with additions");
    assert_eq!(request.body().len(), 31);


    request.body_mut().clear();
    assert_eq!(request.body(), "");
    assert_eq!(request.body().len(), 0);

    request.body_mut().push_str("replaced");
    assert_eq!(request.body(), "replaced");
}

#[test]
fn test_body_mut_modify_vec_body() {
    let mut request = Request::builder()
        .uri("https://example.com/upload")
        .method("POST")
        .body(vec![1u8, 2, 3, 4])
        .unwrap();


    assert_eq!(request.body().len(), 4);
    assert_eq!(request.body()[0], 1);


    request.body_mut().extend_from_slice(&[5, 6, 7, 8]);
    assert_eq!(request.body().len(), 8);
    assert_eq!(request.body()[4], 5);
    assert_eq!(request.body()[7], 8);


    request.body_mut()[0] = 100;
    assert_eq!(request.body()[0], 100);
    assert_ne!(request.body()[0], 1);
}

#[test]
fn test_into_body_consumes_request() {
    let request = Request::builder()
        .uri("https://example.com/data")
        .method("POST")
        .header("X-Custom", "value")
        .body(vec![10u8, 20, 30, 40, 50])
        .unwrap();


    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/data");

    let body = request.into_body();
    assert_eq!(body.len(), 5);
    assert_eq!(body[0], 10);
    assert_eq!(body[1], 20);
    assert_eq!(body[2], 30);
    assert_eq!(body[3], 40);
    assert_eq!(body[4], 50);
    assert_eq!(body, vec![10u8, 20, 30, 40, 50]);
}

#[test]
fn test_into_body_with_complex_type() {
    #[derive(Debug, PartialEq)]
    struct JsonPayload {
        name: String,
        age: u32,
        tags: Vec<String>,
    }

    let payload = JsonPayload {
        name: String::from("Alice"),
        age: 30,
        tags: vec![String::from("admin"), String::from("user")],
    };

    let request = Request::builder()
        .uri("https://api.example.com/users")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(payload)
        .unwrap();

    assert_eq!(request.method(), Method::POST);

    let body = request.into_body();
    assert_eq!(body.name, "Alice");
    assert_eq!(body.age, 30);
    assert_eq!(body.tags.len(), 2);
    assert_eq!(body.tags[0], "admin");
    assert_eq!(body.tags[1], "user");
    assert_ne!(body.name, "Bob");
    assert_ne!(body.age, 0);
}

#[test]
fn test_map_transforms_body_type() {
    let request = Request::builder()
        .uri("https://example.com/transform")
        .method("PUT")
        .header("Accept", "application/json")
        .body("42")
        .unwrap();


    assert_eq!(request.method(), Method::PUT);
    assert_eq!(*request.body(), "42");


    let mapped: Request<u64> = request.map(|body| body.parse::<u64>().unwrap());


    assert_eq!(*mapped.body(), 42u64);

    assert_eq!(mapped.method(), Method::PUT);
    assert_eq!(mapped.uri().path(), "/transform");
    assert_eq!(mapped.uri().host().unwrap(), "example.com");
    assert_eq!(
        mapped.headers().get("accept").unwrap(),
        &HeaderValue::from_static("application/json")
    );
    assert_eq!(mapped.headers().len(), 1);
}

#[test]
fn test_map_string_to_bytes() {
    let mut request = Request::builder()
        .uri("https://example.com/encode")
        .method("POST")
        .header("Content-Type", "text/plain")
        .body(String::from("hello world"))
        .unwrap();


    request.extensions_mut().insert(123u32);

    assert_eq!(request.body().len(), 11);


    let mapped: Request<Vec<u8>> = request.map(|s| s.into_bytes());

    assert_eq!(mapped.body().len(), 11);
    assert_eq!(mapped.body(), &b"hello world"[..]);
    assert_eq!(mapped.body()[0], b'h');
    assert_eq!(mapped.body()[5], b' ');
    assert_eq!(mapped.body()[6], b'w');

    assert_eq!(mapped.extensions().get::<u32>(), Some(&123u32));

    assert_eq!(
        mapped.headers().get("content-type").unwrap(),
        &HeaderValue::from_static("text/plain")
    );
}

#[test]
fn test_map_with_closure_capturing_state() {
    let prefix = b"PREFIX:".to_vec();

    let request = Request::builder()
        .uri("https://example.com/prefix")
        .method("POST")
        .body(vec![1u8, 2, 3])
        .unwrap();

    assert_eq!(request.body().len(), 3);

    let mapped: Request<Vec<u8>> = request.map(|mut body| {
        let mut result = prefix.clone();
        result.append(&mut body);
        result
    });

    assert_eq!(mapped.body().len(), 10);
    assert_eq!(&mapped.body()[..7], b"PREFIX:");
    assert_eq!(mapped.body()[7], 1);
    assert_eq!(mapped.body()[8], 2);
    assert_eq!(mapped.body()[9], 3);
    assert_eq!(mapped.method(), Method::POST);
    assert_eq!(mapped.uri().path(), "/prefix");
    assert_eq!(mapped.uri().host().unwrap(), "example.com");
}

#[test]
fn test_combined_workflow_headers_extensions_body_map() {
    #[derive(Debug, Clone, PartialEq)]
    struct TraceId(String);


    let mut request = Request::builder()
        .uri("https://api.example.com/v2/items?page=1")
        .method("POST")
        .header("Authorization", "Bearer secret")
        .body(String::from("{\"item\": \"widget\"}"))
        .unwrap();


    request.headers_mut().insert(
        HeaderName::from_bytes(b"x-trace-id").unwrap(),
        HeaderValue::from_static("trace-001"),
    );


    request.extensions_mut().insert(TraceId(String::from("trace-001")));


    assert_eq!(request.headers().len(), 2);
    assert_eq!(
        request.headers().get("x-trace-id").unwrap(),
        &HeaderValue::from_static("trace-001")
    );
    assert_eq!(
        request.extensions().get::<TraceId>(),
        Some(&TraceId(String::from("trace-001")))
    );
    assert_eq!(request.body(), "{\"item\": \"widget\"}");


    *request.body_mut() = String::from("{\"item\": \"gadget\", \"qty\": 5}");
    assert_eq!(request.body(), "{\"item\": \"gadget\", \"qty\": 5}");


    let final_request: Request<Vec<u8>> = request.map(|s| s.into_bytes());
    assert_eq!(final_request.body().len(), 28);
    assert_eq!(final_request.method(), Method::POST);
    assert_eq!(final_request.uri().query().unwrap(), "page=1");
    assert_eq!(
        final_request.extensions().get::<TraceId>(),
        Some(&TraceId(String::from("trace-001")))
    );
}

#[test]
fn test_headers_mut_entry_api() {
    let mut request = Request::builder()
        .uri("https://example.com/")
        .body(())
        .unwrap();


    let headers = request.headers_mut();
    let name = HeaderName::from_bytes(b"x-counter").unwrap();


    let entry = headers.entry(name.clone());
    let val = entry.or_insert(HeaderValue::from_static("0"));
    assert_eq!(val, &HeaderValue::from_static("0"));


    assert_eq!(headers.len(), 1);
    assert_eq!(headers.get("x-counter").unwrap(), &HeaderValue::from_static("0"));


    let entry2 = headers.entry(name);
    let val2 = entry2.or_insert(HeaderValue::from_static("999"));
    assert_eq!(val2, &HeaderValue::from_static("0"));
    assert_ne!(val2, &HeaderValue::from_static("999"));
    assert_eq!(headers.len(), 1);
}