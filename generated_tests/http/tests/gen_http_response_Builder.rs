
use http::header::{HeaderMap, HeaderValue, HeaderName};
use http::response::{Builder, Response};
use http::status::StatusCode;
use http::version::Version;
use http::Extensions;

#[test]
fn test_response_builder_status_various_codes() {
    let response = Response::builder()
        .status(200)
        .body(())
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.status().as_u16(), 200);

    let response = Response::builder()
        .status(404)
        .body(())
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.status().as_u16(), 404);

    let response = Response::builder()
        .status(500)
        .body(())
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.status().as_u16(), 500);

    let response = Response::builder()
        .status(StatusCode::CREATED)
        .body(())
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.status().as_u16(), 201);
}

#[test]
fn test_response_builder_version_settings() {
    let response = Response::builder()
        .version(Version::HTTP_10)
        .status(200)
        .body(())
        .unwrap();
    assert_eq!(response.version(), Version::HTTP_10);
    assert_eq!(response.status(), StatusCode::OK);

    let response = Response::builder()
        .version(Version::HTTP_11)
        .status(301)
        .body("moved")
        .unwrap();
    assert_eq!(response.version(), Version::HTTP_11);
    assert_eq!(response.status().as_u16(), 301);
    assert_eq!(*response.body(), "moved");

    let response = Response::builder()
        .version(Version::HTTP_2)
        .status(204)
        .body(Vec::<u8>::new())
        .unwrap();
    assert_eq!(response.version(), Version::HTTP_2);
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.body().is_empty());
}

#[test]
fn test_response_builder_header_single_and_multiple() {
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header("X-Request-Id", "abc-123")
        .header("Cache-Control", "no-cache")
        .body(())
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        &HeaderValue::from_static("application/json")
    );
    assert_eq!(
        response.headers().get("x-request-id").unwrap(),
        &HeaderValue::from_static("abc-123")
    );
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        &HeaderValue::from_static("no-cache")
    );
    assert_eq!(response.headers().len(), 3);


    let response = Response::builder()
        .status(200)
        .header("Set-Cookie", "a=1")
        .header("Set-Cookie", "b=2")
        .body(())
        .unwrap();
    let values: Vec<&HeaderValue> = response.headers().get_all("set-cookie").iter().collect();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], &HeaderValue::from_static("a=1"));
    assert_eq!(values[1], &HeaderValue::from_static("b=2"));
}

#[test]
fn test_response_builder_headers_ref() {
    let builder = Response::builder()
        .status(200)
        .header("X-Foo", "bar")
        .header("X-Baz", "qux");

    let headers_ref = builder.headers_ref();
    assert!(headers_ref.is_some());
    let headers = headers_ref.unwrap();
    assert_eq!(headers.len(), 2);
    assert_eq!(headers.get("x-foo").unwrap(), &HeaderValue::from_static("bar"));
    assert_eq!(headers.get("x-baz").unwrap(), &HeaderValue::from_static("qux"));
    assert!(headers.get("nonexistent").is_none());


    let fresh_builder = Response::builder();
    let fresh_headers = fresh_builder.headers_ref().unwrap();
    assert_eq!(fresh_headers.len(), 0);
    assert!(fresh_headers.is_empty());
}

#[test]
fn test_response_builder_headers_mut() {
    let mut builder = Response::builder()
        .status(200)
        .header("X-Original", "value1");

    {
        let headers_mut = builder.headers_mut();
        assert!(headers_mut.is_some());
        let headers = headers_mut.unwrap();
        assert_eq!(headers.len(), 1);
        headers.insert(
            HeaderName::from_bytes(b"x-added").unwrap(),
            HeaderValue::from_static("value2"),
        );
        headers.insert(
            HeaderName::from_bytes(b"x-another").unwrap(),
            HeaderValue::from_static("value3"),
        );
    }

    let response = builder.body(()).unwrap();
    assert_eq!(response.headers().len(), 3);
    assert_eq!(
        response.headers().get("x-original").unwrap(),
        &HeaderValue::from_static("value1")
    );
    assert_eq!(
        response.headers().get("x-added").unwrap(),
        &HeaderValue::from_static("value2")
    );
    assert_eq!(
        response.headers().get("x-another").unwrap(),
        &HeaderValue::from_static("value3")
    );
}

#[test]
fn test_response_builder_extension_and_extensions_ref() {
    #[derive(Debug, Clone, PartialEq)]
    struct RequestId(u64);

    #[derive(Debug, Clone, PartialEq)]
    struct Timestamp(u32);

    let builder = Response::builder()
        .status(200)
        .extension(RequestId(42))
        .extension(Timestamp(1000));

    let ext_ref = builder.extensions_ref();
    assert!(ext_ref.is_some());
    let extensions = ext_ref.unwrap();
    assert_eq!(extensions.get::<RequestId>(), Some(&RequestId(42)));
    assert_eq!(extensions.get::<Timestamp>(), Some(&Timestamp(1000)));
    assert!(extensions.get::<String>().is_none());

    let response = builder.body(()).unwrap();
    assert_eq!(response.extensions().get::<RequestId>(), Some(&RequestId(42)));
    assert_eq!(response.extensions().get::<Timestamp>(), Some(&Timestamp(1000)));
}

#[test]
fn test_response_builder_extensions_mut() {
    #[derive(Debug, Clone, PartialEq)]
    struct TraceId(String);

    #[derive(Debug, Clone, PartialEq)]
    struct Priority(u8);

    let mut builder = Response::builder()
        .status(202)
        .extension(TraceId("initial".to_string()));

    {
        let ext_mut = builder.extensions_mut();
        assert!(ext_mut.is_some());
        let extensions = ext_mut.unwrap();
        assert_eq!(
            extensions.get::<TraceId>(),
            Some(&TraceId("initial".to_string()))
        );

        extensions.insert(TraceId("overwritten".to_string()));
        extensions.insert(Priority(5));
    }

    let response = builder.body("accepted").unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(
        response.extensions().get::<TraceId>(),
        Some(&TraceId("overwritten".to_string()))
    );
    assert_eq!(response.extensions().get::<Priority>(), Some(&Priority(5)));
    assert_eq!(*response.body(), "accepted");
}

#[test]
fn test_response_builder_full_workflow() {
    #[derive(Debug, Clone, PartialEq)]
    struct CorrelationId(String);

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .version(Version::HTTP_2)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("X-Frame-Options", "DENY")
        .header("Strict-Transport-Security", "max-age=31536000")
        .extension(CorrelationId("req-abc-def".to_string()));


    let headers = builder.headers_ref().unwrap();
    assert_eq!(headers.len(), 3);
    assert!(headers.contains_key("content-type"));

    let ext = builder.extensions_ref().unwrap();
    assert_eq!(
        ext.get::<CorrelationId>(),
        Some(&CorrelationId("req-abc-def".to_string()))
    );


    {
        let hm = builder.headers_mut().unwrap();
        hm.insert(
            HeaderName::from_bytes(b"x-powered-by").unwrap(),
            HeaderValue::from_static("rust"),
        );
    }

    let response = builder.body("<html></html>").unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.version(), Version::HTTP_2);
    assert_eq!(response.headers().len(), 4);
    assert_eq!(
        response.headers().get("x-powered-by").unwrap(),
        &HeaderValue::from_static("rust")
    );
    assert_eq!(*response.body(), "<html></html>");
}

#[test]
fn test_response_builder_chaining_status_override() {

    let response = Response::builder()
        .status(200)
        .status(404)
        .status(503)
        .body(())
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.status().as_u16(), 503);


    let response = Response::builder()
        .version(Version::HTTP_10)
        .version(Version::HTTP_11)
        .version(Version::HTTP_2)
        .body(())
        .unwrap();
    assert_eq!(response.version(), Version::HTTP_2);
    assert_ne!(response.version(), Version::HTTP_11);
    assert_ne!(response.version(), Version::HTTP_10);


    let response = Response::builder().body(()).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.status().as_u16(), 200);
}

#[test]
fn test_response_builder_new_explicit() {
    let builder = Builder::new();
    let headers = builder.headers_ref().unwrap();
    assert!(headers.is_empty());
    assert_eq!(headers.len(), 0);

    let response = Builder::new()
        .status(418)
        .header("X-Teapot", "yes")
        .body("I'm a teapot")
        .unwrap();
    assert_eq!(response.status().as_u16(), 418);
    assert_eq!(
        response.headers().get("x-teapot").unwrap(),
        &HeaderValue::from_static("yes")
    );
    assert_eq!(response.headers().len(), 1);
    assert_eq!(*response.body(), "I'm a teapot");
}