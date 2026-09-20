
use http::{Response, StatusCode, Version};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use http::Extensions;

#[test]
fn test_response_status_and_status_mut() {
    let response = Response::builder()
        .status(StatusCode::OK)
        .body("hello")
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.status().as_u16(), 200);

    let mut response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("not found")
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.status().as_u16(), 404);

    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.status().as_u16(), 500);

    *response.status_mut() = StatusCode::CREATED;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.status().as_u16(), 201);
}

#[test]
fn test_response_version_and_version_mut() {
    let response = Response::builder()
        .version(Version::HTTP_11)
        .body(())
        .unwrap();

    assert_eq!(response.version(), Version::HTTP_11);

    let mut response = Response::builder()
        .version(Version::HTTP_10)
        .body("body")
        .unwrap();

    assert_eq!(response.version(), Version::HTTP_10);
    assert_ne!(response.version(), Version::HTTP_11);

    *response.version_mut() = Version::HTTP_2;
    assert_eq!(response.version(), Version::HTTP_2);
    assert_ne!(response.version(), Version::HTTP_10);

    *response.version_mut() = Version::HTTP_11;
    assert_eq!(response.version(), Version::HTTP_11);

    *response.version_mut() = Version::HTTP_09;
    assert_eq!(response.version(), Version::HTTP_09);
}

#[test]
fn test_response_headers_and_headers_mut() {
    let response = Response::builder()
        .header("Content-Type", "text/html")
        .header("X-Custom", "value1")
        .body(())
        .unwrap();

    let headers = response.headers();
    assert_eq!(headers.len(), 2);
    assert_eq!(
        headers.get("content-type").unwrap(),
        &HeaderValue::from_static("text/html")
    );
    assert_eq!(
        headers.get("x-custom").unwrap(),
        &HeaderValue::from_static("value1")
    );

    let mut response = Response::builder()
        .header("Server", "test-server")
        .body(())
        .unwrap();

    assert_eq!(response.headers().len(), 1);

    response.headers_mut().insert(
        HeaderName::from_bytes(b"x-added").unwrap(),
        HeaderValue::from_static("added-value"),
    );
    assert_eq!(response.headers().len(), 2);
    assert_eq!(
        response.headers().get("x-added").unwrap(),
        &HeaderValue::from_static("added-value")
    );

    response.headers_mut().remove("server");
    assert_eq!(response.headers().len(), 1);
    assert!(response.headers().get("server").is_none());
}

#[test]
fn test_response_extensions_and_extensions_mut() {
    #[derive(Debug, Clone, PartialEq)]
    struct RequestId(u64);

    #[derive(Debug, Clone, PartialEq)]
    struct Timestamp(u32);

    let mut response = Response::builder().body(()).unwrap();

    assert!(response.extensions().get::<RequestId>().is_none());
    assert!(response.extensions().get::<Timestamp>().is_none());

    response.extensions_mut().insert(RequestId(42));
    response.extensions_mut().insert(Timestamp(1000));

    assert_eq!(response.extensions().get::<RequestId>(), Some(&RequestId(42)));
    assert_eq!(response.extensions().get::<Timestamp>(), Some(&Timestamp(1000)));

    response.extensions_mut().insert(RequestId(99));
    assert_eq!(response.extensions().get::<RequestId>(), Some(&RequestId(99)));
    assert_ne!(response.extensions().get::<RequestId>(), Some(&RequestId(42)));
}

#[test]
fn test_response_body_mut_and_into_body() {
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .body(String::from("initial"))
        .unwrap();

    assert_eq!(response.body_mut(), &mut String::from("initial"));
    assert_eq!(response.body_mut().as_str(), "initial");

    response.body_mut().push_str(" appended");
    assert_eq!(response.body_mut().as_str(), "initial appended");
    assert_eq!(response.body_mut().len(), 16);

    response.body_mut().clear();
    assert_eq!(response.body_mut().as_str(), "");
    assert!(response.body_mut().is_empty());

    *response.body_mut() = String::from("replaced");
    let body = response.into_body();
    assert_eq!(body, "replaced");
    assert_eq!(body.len(), 8);
}

#[test]
fn test_response_into_body_with_vec() {
    let data: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(data)
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    assert_eq!(body.len(), 8);
    assert_eq!(body[0], 1);
    assert_eq!(body[7], 8);
    assert_eq!(body.iter().sum::<u8>(), 36);
    assert_ne!(body, vec![0u8; 8]);
    assert_eq!(body, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn test_response_map() {
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body("hello world")
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let mapped = response.map(|body: &str| body.len());

    assert_eq!(mapped.status(), StatusCode::OK);
    assert_eq!(mapped.status().as_u16(), 200);
    assert_eq!(*mapped.body(), 11);
    assert_eq!(
        mapped.headers().get("content-type").unwrap(),
        &HeaderValue::from_static("application/json")
    );

    let mapped2 = mapped.map(|len| format!("length={}", len));
    assert_eq!(mapped2.status(), StatusCode::OK);
    assert_eq!(mapped2.body(), "length=11");
    assert_eq!(mapped2.headers().len(), 1);
    assert_eq!(mapped2.body().len(), 9);
}

#[test]
fn test_response_map_preserves_all_metadata() {
    #[derive(Debug, Clone, PartialEq)]
    struct TraceId(String);

    let mut response = Response::builder()
        .status(StatusCode::ACCEPTED)
        .version(Version::HTTP_2)
        .header("X-Request-Id", "abc123")
        .header("Server", "test")
        .body(vec![10u8, 20, 30])
        .unwrap();

    response.extensions_mut().insert(TraceId("trace-001".to_string()));

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.version(), Version::HTTP_2);
    assert_eq!(response.headers().len(), 2);

    let mapped = response.map(|body| {
        body.iter().map(|b| *b as u32).sum::<u32>()
    });

    assert_eq!(mapped.status(), StatusCode::ACCEPTED);
    assert_eq!(mapped.status().as_u16(), 202);
    assert_eq!(mapped.version(), Version::HTTP_2);
    assert_eq!(mapped.headers().len(), 2);
    assert_eq!(
        mapped.headers().get("x-request-id").unwrap(),
        &HeaderValue::from_static("abc123")
    );
    assert_eq!(*mapped.body(), 60u32);
    assert_eq!(
        mapped.extensions().get::<TraceId>(),
        Some(&TraceId("trace-001".to_string()))
    );
}

#[test]
fn test_response_full_workflow_mutation() {
    let mut response = Response::builder()
        .status(StatusCode::from_u16(100).unwrap())
        .version(Version::HTTP_11)
        .body(String::new())
        .unwrap();

    assert_eq!(response.status().as_u16(), 100);
    assert_eq!(response.version(), Version::HTTP_11);
    assert!(response.headers().is_empty());
    assert!(response.body_mut().is_empty());

    *response.status_mut() = StatusCode::OK;
    *response.version_mut() = Version::HTTP_2;
    response.headers_mut().insert(
        HeaderName::from_bytes(b"content-length").unwrap(),
        HeaderValue::from_static("13"),
    );
    *response.body_mut() = String::from("Hello, World!");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.version(), Version::HTTP_2);
    assert_eq!(response.headers().len(), 1);
    assert_eq!(
        response.headers().get("content-length").unwrap(),
        &HeaderValue::from_static("13")
    );

    let body = response.into_body();
    assert_eq!(body, "Hello, World!");
    assert_eq!(body.len(), 13);
}

#[test]
fn test_response_default_values() {
    let response = Response::builder().body(()).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.version(), Version::HTTP_11);
    assert!(response.headers().is_empty());
    assert_eq!(response.headers().len(), 0);
    assert!(response.extensions().get::<u32>().is_none());
    assert!(response.extensions().get::<String>().is_none());
    assert_eq!(*response.body(), ());
    assert_eq!(response.into_body(), ());
}