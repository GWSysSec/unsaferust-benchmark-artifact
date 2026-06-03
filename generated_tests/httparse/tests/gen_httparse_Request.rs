
use std::mem::MaybeUninit;

use httparse::{Header, Request, Status, EMPTY_HEADER};

#[test]
fn test_parse_with_uninit_headers_complete_request() {
    let req_bytes = b"GET /hello HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0\r\n\r\n";

    let mut req = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result = req.parse_with_uninit_headers(req_bytes, &mut uninit_headers);
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(status.is_complete());

    let bytes_parsed = match status {
        Status::Complete(n) => n,
        _ => panic!("Expected complete"),
    };

    assert_eq!(bytes_parsed, req_bytes.len());
    assert_eq!(req.method, Some("GET"));
    assert_eq!(req.path, Some("/hello"));
    assert_eq!(req.version, Some(1));
    assert_eq!(req.headers.len(), 2);
    assert_eq!(req.headers[0].name, "Host");
    assert_eq!(req.headers[0].value, b"example.com");
    assert_eq!(req.headers[1].name, "Content-Length");
    assert_eq!(req.headers[1].value, b"0");
}

#[test]
fn test_parse_with_uninit_headers_partial_request() {
    let partial = b"GET /path HTTP/1.1\r\nHost: example.com\r\n";

    let mut req = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result = req.parse_with_uninit_headers(partial, &mut uninit_headers);
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(!status.is_complete());



    assert_eq!(status.is_complete(), false);


    let complete = b"GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n";

    let mut req2 = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers2: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result2 = req2.parse_with_uninit_headers(complete, &mut uninit_headers2);
    assert!(result2.is_ok());
    let status2 = result2.unwrap();
    assert!(status2.is_complete());
    assert_eq!(req2.method, Some("GET"));
    assert_eq!(req2.path, Some("/path"));
}

#[test]
fn test_parse_with_uninit_headers_many_headers() {
    let req_bytes = b"POST /submit HTTP/1.1\r\n\
        Host: example.com\r\n\
        Content-Type: application/json\r\n\
        Content-Length: 42\r\n\
        Accept: */*\r\n\
        Authorization: Bearer token123\r\n\
        X-Request-Id: abc-def-ghi\r\n\
        Cache-Control: no-cache\r\n\
        User-Agent: test/1.0\r\n\
        \r\n";

    let mut req = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers: [MaybeUninit<Header<'_>>; 32] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result = req.parse_with_uninit_headers(req_bytes, &mut uninit_headers);
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(status.is_complete());

    assert_eq!(req.method, Some("POST"));
    assert_eq!(req.path, Some("/submit"));
    assert_eq!(req.version, Some(1));
    assert_eq!(req.headers.len(), 8);
    assert_eq!(req.headers[0].name, "Host");
    assert_eq!(req.headers[1].name, "Content-Type");
    assert_eq!(req.headers[1].value, b"application/json");
    assert_eq!(req.headers[2].name, "Content-Length");
    assert_eq!(req.headers[2].value, b"42");
    assert_eq!(req.headers[3].name, "Accept");
    assert_eq!(req.headers[4].name, "Authorization");
    assert_eq!(req.headers[4].value, b"Bearer token123");
    assert_eq!(req.headers[5].name, "X-Request-Id");
    assert_eq!(req.headers[6].name, "Cache-Control");
    assert_eq!(req.headers[7].name, "User-Agent");
    assert_eq!(req.headers[7].value, b"test/1.0");
}

#[test]
fn test_parse_with_uninit_headers_too_few_slots() {
    let req_bytes = b"GET / HTTP/1.1\r\nHost: a\r\nFoo: b\r\nBar: c\r\n\r\n";

    let mut req = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };


    let mut uninit_headers: [MaybeUninit<Header<'_>>; 1] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result = req.parse_with_uninit_headers(req_bytes, &mut uninit_headers);

    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_clone = err.clone();

    assert_eq!(format!("{}", err), format!("{}", err_clone));
}

#[test]
fn test_parse_with_uninit_headers_no_headers() {
    let req_bytes = b"DELETE /resource HTTP/1.0\r\n\r\n";

    let mut req = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers: [MaybeUninit<Header<'_>>; 8] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result = req.parse_with_uninit_headers(req_bytes, &mut uninit_headers);
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(status.is_complete());

    let bytes_parsed = match status {
        Status::Complete(n) => n,
        _ => panic!("Expected complete"),
    };

    assert_eq!(bytes_parsed, req_bytes.len());
    assert_eq!(req.method, Some("DELETE"));
    assert_eq!(req.path, Some("/resource"));
    assert_eq!(req.version, Some(0));
    assert_eq!(req.headers.len(), 0);
}

#[test]
fn test_parse_with_uninit_headers_invalid_request() {
    let bad_bytes = b"NOT A VALID REQUEST\r\n\r\n";

    let mut req = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers: [MaybeUninit<Header<'_>>; 8] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result = req.parse_with_uninit_headers(bad_bytes, &mut uninit_headers);

    assert!(result.is_err());


    let empty = b"";
    let mut req2 = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers2: [MaybeUninit<Header<'_>>; 8] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result2 = req2.parse_with_uninit_headers(empty, &mut uninit_headers2);
    assert!(result2.is_ok());
    let status2 = result2.unwrap();
    assert!(!status2.is_complete());


    assert_eq!(req2.method, None);
    assert_eq!(req2.path, None);
    assert_eq!(req2.version, None);
}

#[test]
fn test_parse_with_uninit_headers_comparison_with_regular_parse() {
    let req_bytes = b"PUT /data HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\n\r\n";


    let mut headers_regular = [EMPTY_HEADER; 16];
    let mut req_regular = Request::new(&mut headers_regular);
    let result_regular = req_regular.parse(req_bytes);
    assert!(result_regular.is_ok());
    let status_regular = result_regular.unwrap();
    assert!(status_regular.is_complete());


    let mut req_uninit = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result_uninit = req_uninit.parse_with_uninit_headers(req_bytes, &mut uninit_headers);
    assert!(result_uninit.is_ok());
    let status_uninit = result_uninit.unwrap();
    assert!(status_uninit.is_complete());


    assert_eq!(req_regular.method, req_uninit.method);
    assert_eq!(req_regular.path, req_uninit.path);
    assert_eq!(req_regular.version, req_uninit.version);
    assert_eq!(req_regular.headers.len(), req_uninit.headers.len());
    assert_eq!(req_regular.headers[0].name, req_uninit.headers[0].name);
    assert_eq!(req_regular.headers[0].value, req_uninit.headers[0].value);
    assert_eq!(req_regular.headers[1].name, req_uninit.headers[1].name);
    assert_eq!(req_regular.headers[1].value, req_uninit.headers[1].value);


    let n_regular = match status_regular {
        Status::Complete(n) => n,
        _ => panic!("expected complete"),
    };
    let n_uninit = match status_uninit {
        Status::Complete(n) => n,
        _ => panic!("expected complete"),
    };
    assert_eq!(n_regular, n_uninit);
}

#[test]
fn test_parse_with_uninit_headers_large_header_values() {
    let long_value = "x".repeat(4096);
    let req_str = format!(
        "GET /big HTTP/1.1\r\nHost: example.com\r\nX-Big: {}\r\n\r\n",
        long_value
    );
    let req_bytes = req_str.as_bytes();

    let mut req = Request {
        method: None,
        path: None,
        version: None,
        headers: &mut [],
    };

    let mut uninit_headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };

    let result = req.parse_with_uninit_headers(req_bytes, &mut uninit_headers);
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(status.is_complete());

    assert_eq!(req.method, Some("GET"));
    assert_eq!(req.path, Some("/big"));
    assert_eq!(req.version, Some(1));
    assert_eq!(req.headers.len(), 2);
    assert_eq!(req.headers[1].name, "X-Big");
    assert_eq!(req.headers[1].value.len(), 4096);
    assert_eq!(req.headers[1].value[0], b'x');
    assert_eq!(req.headers[1].value[4095], b'x');
}