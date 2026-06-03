use std::mem::MaybeUninit;

use httparse::{Header, ParserConfig, Request, Response, Status, EMPTY_HEADER};

#[test]
fn test_parser_config_multiple_spaces_in_request_line_delimiters_default() {
    let config = ParserConfig::default();
    let default_value = config.multiple_spaces_in_request_line_delimiters_are_allowed();
    assert_eq!(default_value, false);

    let mut config2 = ParserConfig::default();
    config2.allow_multiple_spaces_in_request_line_delimiters(true);
    let enabled_value = config2.multiple_spaces_in_request_line_delimiters_are_allowed();
    assert_eq!(enabled_value, true);

    config2.allow_multiple_spaces_in_request_line_delimiters(false);
    let disabled_value = config2.multiple_spaces_in_request_line_delimiters_are_allowed();
    assert_eq!(disabled_value, false);


    let mut config3 = ParserConfig::default();
    config3.allow_multiple_spaces_in_request_line_delimiters(true);
    let buf = b"GET  /path  HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);
    let result = config3.parse_request(&mut req, buf);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(req.method.unwrap(), "GET");
    assert_eq!(req.path.unwrap(), "/path");
    assert_eq!(req.version.unwrap(), 1);
}

#[test]
fn test_parser_config_multiple_spaces_in_response_status_delimiters_default() {
    let config = ParserConfig::default();
    let default_value = config.multiple_spaces_in_response_status_delimiters_are_allowed();
    assert_eq!(default_value, false);

    let mut config2 = ParserConfig::default();
    config2.allow_multiple_spaces_in_response_status_delimiters(true);
    let enabled_value = config2.multiple_spaces_in_response_status_delimiters_are_allowed();
    assert_eq!(enabled_value, true);

    config2.allow_multiple_spaces_in_response_status_delimiters(false);
    let disabled_value = config2.multiple_spaces_in_response_status_delimiters_are_allowed();
    assert_eq!(disabled_value, false);


    let mut config3 = ParserConfig::default();
    config3.allow_multiple_spaces_in_response_status_delimiters(true);
    let buf = b"HTTP/1.1  200  OK\r\nContent-Length: 0\r\n\r\n";
    let mut headers = [EMPTY_HEADER; 16];
    let mut resp = Response::new(&mut headers);
    let result = config3.parse_response(&mut resp, buf);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(resp.code.unwrap(), 200);
    assert_eq!(resp.version.unwrap(), 1);
}

#[test]
fn test_parser_config_obsolete_multiline_headers_in_responses_default() {
    let config = ParserConfig::default();
    let default_value = config.obsolete_multiline_headers_in_responses_are_allowed();
    assert_eq!(default_value, false);

    let mut config2 = ParserConfig::default();
    config2.allow_obsolete_multiline_headers_in_responses(true);
    let enabled_value = config2.obsolete_multiline_headers_in_responses_are_allowed();
    assert_eq!(enabled_value, true);

    config2.allow_obsolete_multiline_headers_in_responses(false);
    let disabled_value = config2.obsolete_multiline_headers_in_responses_are_allowed();
    assert_eq!(disabled_value, false);


    let mut config3 = ParserConfig::default();
    config3.allow_obsolete_multiline_headers_in_responses(true);
    let buf = b"HTTP/1.1 200 OK\r\nX-Custom: first\r\n second\r\nContent-Length: 0\r\n\r\n";
    let mut headers = [EMPTY_HEADER; 16];
    let mut resp = Response::new(&mut headers);
    let result = config3.parse_response(&mut resp, buf);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(resp.code.unwrap(), 200);
}

#[test]
fn test_parser_config_space_before_first_header_name_default() {
    let config = ParserConfig::default();
    let default_value = config.space_before_first_header_name_are_allowed();
    assert_eq!(default_value, false);

    let mut config2 = ParserConfig::default();
    config2.allow_space_before_first_header_name(true);
    let enabled_value = config2.space_before_first_header_name_are_allowed();
    assert_eq!(enabled_value, true);

    config2.allow_space_before_first_header_name(false);
    let disabled_value = config2.space_before_first_header_name_are_allowed();
    assert_eq!(disabled_value, false);


    let mut config3 = ParserConfig::default();
    config3.allow_space_before_first_header_name(true);
    let buf = b"HTTP/1.1 200 OK\r\n Host: example.com\r\nContent-Length: 0\r\n\r\n";
    let mut headers = [EMPTY_HEADER; 16];
    let mut resp = Response::new(&mut headers);
    let result = config3.parse_response(&mut resp, buf);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(resp.code.unwrap(), 200);
    assert_eq!(resp.version.unwrap(), 1);
}

#[test]
fn test_parse_request_with_uninit_headers_basic() {
    let config = ParserConfig::default();
    let buf = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nAccept: text/html\r\nConnection: keep-alive\r\n\r\n";

    let mut headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut req = Request::new(&mut []);
    let result = config.parse_request_with_uninit_headers(&mut req, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(req.method.unwrap(), "GET");
    assert_eq!(req.path.unwrap(), "/index.html");
    assert_eq!(req.version.unwrap(), 1);
    assert_eq!(req.headers.len(), 3);
    assert_eq!(req.headers[0].name, "Host");
    assert_eq!(req.headers[0].value, b"example.com");
    assert_eq!(req.headers[1].name, "Accept");
    assert_eq!(req.headers[1].value, b"text/html");
    assert_eq!(req.headers[2].name, "Connection");
    assert_eq!(req.headers[2].value, b"keep-alive");
}

#[test]
fn test_parse_request_with_uninit_headers_partial() {
    let config = ParserConfig::default();
    let buf = b"POST /api/data HTTP/1.1\r\nContent-Type: application/json\r\n";

    let mut headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut req = Request::new(&mut []);
    let result = config.parse_request_with_uninit_headers(&mut req, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert_eq!(status.is_complete(), false);


    let buf_complete = b"POST /api/data HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: 13\r\n\r\n";
    let mut headers2: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut req2 = Request::new(&mut []);
    let result2 = config.parse_request_with_uninit_headers(&mut req2, buf_complete, &mut headers2);
    assert!(result2.is_ok());
    let status2 = result2.unwrap();
    assert!(status2.is_complete());
    assert_eq!(req2.method.unwrap(), "POST");
    assert_eq!(req2.path.unwrap(), "/api/data");
    assert_eq!(req2.headers.len(), 2);
}

#[test]
fn test_parse_response_with_uninit_headers_basic() {
    let config = ParserConfig::default();
    let buf = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 1234\r\nServer: test\r\n\r\n";

    let mut headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut resp = Response::new(&mut []);
    let result = config.parse_response_with_uninit_headers(&mut resp, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(resp.version.unwrap(), 1);
    assert_eq!(resp.code.unwrap(), 200);
    assert_eq!(resp.reason.unwrap(), "OK");
    assert_eq!(resp.headers.len(), 3);
    assert_eq!(resp.headers[0].name, "Content-Type");
    assert_eq!(resp.headers[0].value, b"text/html");
    assert_eq!(resp.headers[1].name, "Content-Length");
    assert_eq!(resp.headers[1].value, b"1234");
    assert_eq!(resp.headers[2].name, "Server");
    assert_eq!(resp.headers[2].value, b"test");
}

#[test]
fn test_parse_response_with_uninit_headers_partial() {
    let config = ParserConfig::default();
    let buf = b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n";

    let mut headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut resp = Response::new(&mut []);
    let result = config.parse_response_with_uninit_headers(&mut resp, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert_eq!(status.is_complete(), false);


    let buf_complete = b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\n";
    let mut headers2: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut resp2 = Response::new(&mut []);
    let result2 = config.parse_response_with_uninit_headers(&mut resp2, buf_complete, &mut headers2);
    assert!(result2.is_ok());
    let status2 = result2.unwrap();
    assert!(status2.is_complete());
    assert_eq!(resp2.code.unwrap(), 404);
    assert_eq!(resp2.reason.unwrap(), "Not Found");
    assert_eq!(resp2.headers.len(), 2);
}

#[test]
fn test_parse_request_with_uninit_headers_many_headers() {
    let config = ParserConfig::default();
    let buf = b"GET / HTTP/1.1\r\nH1: v1\r\nH2: v2\r\nH3: v3\r\nH4: v4\r\nH5: v5\r\nH6: v6\r\nH7: v7\r\nH8: v8\r\n\r\n";

    let mut headers: [MaybeUninit<Header<'_>>; 32] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut req = Request::new(&mut []);
    let result = config.parse_request_with_uninit_headers(&mut req, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(req.headers.len(), 8);
    assert_eq!(req.headers[0].name, "H1");
    assert_eq!(req.headers[0].value, b"v1");
    assert_eq!(req.headers[7].name, "H8");
    assert_eq!(req.headers[7].value, b"v8");
    assert_eq!(req.method.unwrap(), "GET");
    assert_eq!(req.path.unwrap(), "/");
}

#[test]
fn test_parse_response_with_uninit_headers_with_config_options() {
    let mut config = ParserConfig::default();
    config.allow_multiple_spaces_in_response_status_delimiters(true);
    assert_eq!(config.multiple_spaces_in_response_status_delimiters_are_allowed(), true);

    let buf = b"HTTP/1.1  200  OK\r\nServer: nginx\r\nX-Powered-By: Rust\r\n\r\n";
    let mut headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut resp = Response::new(&mut []);
    let result = config.parse_response_with_uninit_headers(&mut resp, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(resp.code.unwrap(), 200);
    assert_eq!(resp.reason.unwrap(), "OK");
    assert_eq!(resp.headers.len(), 2);
    assert_eq!(resp.headers[0].name, "Server");
    assert_eq!(resp.headers[1].name, "X-Powered-By");
    assert_eq!(resp.headers[1].value, b"Rust");
}

#[test]
fn test_parse_request_with_uninit_headers_with_multiple_spaces_config() {
    let mut config = ParserConfig::default();
    config.allow_multiple_spaces_in_request_line_delimiters(true);
    assert_eq!(config.multiple_spaces_in_request_line_delimiters_are_allowed(), true);

    let buf = b"DELETE  /resource/42  HTTP/1.1\r\nAuthorization: Bearer token123\r\n\r\n";
    let mut headers: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut req = Request::new(&mut []);
    let result = config.parse_request_with_uninit_headers(&mut req, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(req.method.unwrap(), "DELETE");
    assert_eq!(req.path.unwrap(), "/resource/42");
    assert_eq!(req.version.unwrap(), 1);
    assert_eq!(req.headers.len(), 1);
    assert_eq!(req.headers[0].name, "Authorization");
    assert_eq!(req.headers[0].value, b"Bearer token123");
}

#[test]
fn test_config_chaining_and_getter_consistency() {
    let mut config = ParserConfig::default();


    assert_eq!(config.multiple_spaces_in_request_line_delimiters_are_allowed(), false);
    assert_eq!(config.multiple_spaces_in_response_status_delimiters_are_allowed(), false);
    assert_eq!(config.obsolete_multiline_headers_in_responses_are_allowed(), false);
    assert_eq!(config.space_before_first_header_name_are_allowed(), false);


    config.allow_multiple_spaces_in_request_line_delimiters(true);
    config.allow_multiple_spaces_in_response_status_delimiters(true);
    config.allow_obsolete_multiline_headers_in_responses(true);
    config.allow_space_before_first_header_name(true);


    assert_eq!(config.multiple_spaces_in_request_line_delimiters_are_allowed(), true);
    assert_eq!(config.multiple_spaces_in_response_status_delimiters_are_allowed(), true);
    assert_eq!(config.obsolete_multiline_headers_in_responses_are_allowed(), true);
    assert_eq!(config.space_before_first_header_name_are_allowed(), true);


    config.allow_multiple_spaces_in_request_line_delimiters(false);
    config.allow_multiple_spaces_in_response_status_delimiters(false);
    config.allow_obsolete_multiline_headers_in_responses(false);
    config.allow_space_before_first_header_name(false);


    assert_eq!(config.multiple_spaces_in_request_line_delimiters_are_allowed(), false);
    assert_eq!(config.multiple_spaces_in_response_status_delimiters_are_allowed(), false);
    assert_eq!(config.obsolete_multiline_headers_in_responses_are_allowed(), false);
    assert_eq!(config.space_before_first_header_name_are_allowed(), false);
}

#[test]
fn test_parse_request_uninit_headers_too_few_headers() {
    let config = ParserConfig::default();
    let buf = b"GET / HTTP/1.1\r\nA: 1\r\nB: 2\r\nC: 3\r\n\r\n";


    let mut headers: [MaybeUninit<Header<'_>>; 1] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut req = Request::new(&mut []);
    let result = config.parse_request_with_uninit_headers(&mut req, buf, &mut headers);

    assert!(result.is_err());


    let mut headers2: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut req2 = Request::new(&mut []);
    let result2 = config.parse_request_with_uninit_headers(&mut req2, buf, &mut headers2);
    assert!(result2.is_ok());
    assert!(result2.unwrap().is_complete());
    assert_eq!(req2.headers.len(), 3);
    assert_eq!(req2.method.unwrap(), "GET");
}

#[test]
fn test_parse_response_uninit_headers_too_few_headers() {
    let config = ParserConfig::default();
    let buf = b"HTTP/1.1 200 OK\r\nA: 1\r\nB: 2\r\nC: 3\r\n\r\n";


    let mut headers: [MaybeUninit<Header<'_>>; 1] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut resp = Response::new(&mut []);
    let result = config.parse_response_with_uninit_headers(&mut resp, buf, &mut headers);
    assert!(result.is_err());


    let mut headers2: [MaybeUninit<Header<'_>>; 16] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut resp2 = Response::new(&mut []);
    let result2 = config.parse_response_with_uninit_headers(&mut resp2, buf, &mut headers2);
    assert!(result2.is_ok());
    assert!(result2.unwrap().is_complete());
    assert_eq!(resp2.headers.len(), 3);
    assert_eq!(resp2.code.unwrap(), 200);
    assert_eq!(resp2.reason.unwrap(), "OK");
    assert_eq!(resp2.version.unwrap(), 1);
}

#[test]
fn test_parse_request_uninit_headers_zero_headers() {
    let config = ParserConfig::default();
    let buf = b"OPTIONS * HTTP/1.1\r\n\r\n";

    let mut headers: [MaybeUninit<Header<'_>>; 4] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut req = Request::new(&mut []);
    let result = config.parse_request_with_uninit_headers(&mut req, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(req.method.unwrap(), "OPTIONS");
    assert_eq!(req.path.unwrap(), "*");
    assert_eq!(req.version.unwrap(), 1);
    assert_eq!(req.headers.len(), 0);


    if let Status::Complete(n) = status {
        assert_eq!(n, buf.len());
    }
}

#[test]
fn test_parse_response_uninit_headers_zero_headers() {
    let config = ParserConfig::default();
    let buf = b"HTTP/1.0 204 No Content\r\n\r\n";

    let mut headers: [MaybeUninit<Header<'_>>; 4] = unsafe {
        MaybeUninit::uninit().assume_init()
    };
    let mut resp = Response::new(&mut []);
    let result = config.parse_response_with_uninit_headers(&mut resp, buf, &mut headers);
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.is_complete());
    assert_eq!(resp.version.unwrap(), 0);
    assert_eq!(resp.code.unwrap(), 204);
    assert_eq!(resp.reason.unwrap(), "No Content");
    assert_eq!(resp.headers.len(), 0);

    if let Status::Complete(n) = status {
        assert_eq!(n, buf.len());
    }
}

#[test]
fn test_obsolete_multiline_headers_rejected_by_default() {
    let config = ParserConfig::default();
    assert_eq!(config.obsolete_multiline_headers_in_responses_are_allowed(), false);

    let buf = b"HTTP/1.1 200 OK\r\nX-Folded: line1\r\n line2\r\nContent-Length: 0\r\n\r\n";
    let mut headers = [EMPTY_HEADER; 16];
    let mut resp = Response::new(&mut headers);
    let result = config.parse_response(&mut resp, buf);

    assert!(result.is_err());


    let mut config2 = ParserConfig::default();
    config2.allow_obsolete_multiline_headers_in_responses(true);
    assert_eq!(config2.obsolete_multiline_headers_in_responses_are_allowed(), true);

    let mut headers2 = [EMPTY_HEADER; 16];
    let mut resp2 = Response::new(&mut headers2);
    let result2 = config2.parse_response(&mut resp2, buf);
    assert!(result2.is_ok());
    assert!(result2.unwrap().is_complete());
    assert_eq!(resp2.code.unwrap(), 200);
}

#[test]
fn test_multiple_spaces_in_request_rejected_by_default() {
    let config = ParserConfig::default();
    assert_eq!(config.multiple_spaces_in_request_line_delimiters_are_allowed(), false);

    let buf = b"GET  /path  HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);
    let result = config.parse_request(&mut req, buf);

    assert!(result.is_err());


    let mut config2 = ParserConfig::default();
    config2.allow_multiple_spaces_in_request_line_delimiters(true);
    let mut headers2 = [EMPTY_HEADER; 16];
    let mut req2 = Request::new(&mut headers2);
    let result2 = config2.parse_request(&mut req2, buf);
    assert!(result2.is_ok());
    assert!(result2.unwrap().is_complete());
    assert_eq!(req2.method.unwrap(), "GET");
    assert_eq!(req2.path.unwrap(), "/path");
}