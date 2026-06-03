
use httparse::{Request, Response, Header, Status, parse_chunk_size, parse_headers, ParserConfig, EMPTY_HEADER};

#[test]
fn test_status_is_complete_with_full_request() {
    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);
    let buf = b"GET /path HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0\r\n\r\n";
    let result = req.parse(buf).unwrap();

    assert!(result.is_complete());
    assert!(!result.is_partial());
    assert_eq!(req.method.unwrap(), "GET");
    assert_eq!(req.path.unwrap(), "/path");
    assert_eq!(req.version.unwrap(), 1);
    assert_eq!(req.headers.len(), 2);
    assert_eq!(req.headers[0].name, "Host");
    assert_eq!(req.headers[0].value, b"example.com");
    assert_eq!(req.headers[1].name, "Content-Length");
    assert_eq!(req.headers[1].value, b"0");
}

#[test]
fn test_status_is_partial_with_incomplete_request() {
    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);
    let buf = b"GET /path HTTP/1.1\r\nHost: example.com\r\n";
    let result = req.parse(buf).unwrap();

    assert!(result.is_partial());
    assert!(!result.is_complete());


    let mut headers2 = [EMPTY_HEADER; 16];
    let mut req2 = Request::new(&mut headers2);
    let buf_complete = b"GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result2 = req2.parse(buf_complete).unwrap();

    assert!(result2.is_complete());
    assert!(!result2.is_partial());
    assert_eq!(req2.method.unwrap(), "GET");
    assert_eq!(req2.path.unwrap(), "/path");
    assert_eq!(req2.version.unwrap(), 1);
    assert_eq!(req2.headers[0].name, "Host");
}

#[test]
fn test_status_unwrap_on_complete_request() {
    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);
    let buf = b"POST /submit HTTP/1.1\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello";
    let result = req.parse(buf).unwrap();

    assert!(result.is_complete());
    let bytes_parsed = result.unwrap();
    assert_eq!(bytes_parsed, buf.len() - 5);
    assert_eq!(req.method.unwrap(), "POST");
    assert_eq!(req.path.unwrap(), "/submit");
    assert_eq!(req.version.unwrap(), 1);
    assert_eq!(req.headers.len(), 2);
    assert_eq!(req.headers[0].name, "Content-Type");
    assert_eq!(req.headers[0].value, b"text/plain");
}

#[test]
fn test_status_is_complete_response() {
    let mut headers = [EMPTY_HEADER; 16];
    let mut resp = Response::new(&mut headers);
    let buf = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nServer: test\r\n\r\n";
    let result = resp.parse(buf).unwrap();

    assert!(result.is_complete());
    assert!(!result.is_partial());
    let bytes_parsed = result.unwrap();
    assert_eq!(bytes_parsed, buf.len());
    assert_eq!(resp.version.unwrap(), 1);
    assert_eq!(resp.code.unwrap(), 200);
    assert_eq!(resp.reason.unwrap(), "OK");
    assert_eq!(resp.headers.len(), 2);
    assert_eq!(resp.headers[1].name, "Server");
}

#[test]
fn test_status_is_partial_response() {
    let mut headers = [EMPTY_HEADER; 16];
    let mut resp = Response::new(&mut headers);
    let buf = b"HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\n";
    let result = resp.parse(buf).unwrap();

    assert!(result.is_partial());
    assert!(!result.is_complete());


    let mut headers2 = [EMPTY_HEADER; 16];
    let mut resp2 = Response::new(&mut headers2);
    let buf2 = b"HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\n\r\n";
    let result2 = resp2.parse(buf2).unwrap();

    assert!(result2.is_complete());
    assert!(!result2.is_partial());
    assert_eq!(resp2.code.unwrap(), 404);
    assert_eq!(resp2.reason.unwrap(), "Not Found");
}

#[test]
fn test_status_unwrap_on_parse_chunk_size_complete() {
    let buf = b"a\r\n";
    let result = parse_chunk_size(buf).unwrap();

    assert!(result.is_complete());
    assert!(!result.is_partial());
    let (bytes_consumed, chunk_size) = result.unwrap();
    assert_eq!(bytes_consumed, 3);
    assert_eq!(chunk_size, 10);


    let buf2 = b"FF\r\n";
    let result2 = parse_chunk_size(buf2).unwrap();
    assert!(result2.is_complete());
    let (bytes2, size2) = result2.unwrap();
    assert_eq!(bytes2, 4);
    assert_eq!(size2, 255);
}

#[test]
fn test_status_is_partial_on_incomplete_chunk_size() {
    let buf = b"a";
    let result = parse_chunk_size(buf).unwrap();

    assert!(result.is_partial());
    assert!(!result.is_complete());

    let buf2 = b"FF";
    let result2 = parse_chunk_size(buf2).unwrap();
    assert!(result2.is_partial());
    assert!(!result2.is_complete());


    let buf3 = b"0\r\n";
    let result3 = parse_chunk_size(buf3).unwrap();
    assert!(result3.is_complete());
    assert!(!result3.is_partial());
    let (bytes3, size3) = result3.unwrap();
    assert_eq!(bytes3, 3);
    assert_eq!(size3, 0);
}

#[test]
#[should_panic]
fn test_status_unwrap_panics_on_partial() {
    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);
    let buf = b"GET /path HTTP/1.1\r\nHost: example.com\r\n";
    let result = req.parse(buf).unwrap();

    assert!(result.is_partial());

    let _ = result.unwrap();
}

#[test]
fn test_status_methods_with_parse_headers() {
    let buf = b"Host: example.com\r\nAccept: */*\r\n\r\ntrailing";
    let mut headers = [EMPTY_HEADER; 4];
    let result = parse_headers(buf, &mut headers).unwrap();

    assert!(result.is_complete());
    assert!(!result.is_partial());
    let (bytes_parsed, parsed_headers) = result.unwrap();
    assert_eq!(parsed_headers.len(), 2);
    assert_eq!(parsed_headers[0].name, "Host");
    assert_eq!(parsed_headers[0].value, b"example.com");
    assert_eq!(parsed_headers[1].name, "Accept");
    assert_eq!(parsed_headers[1].value, b"*/*");

    assert_eq!(bytes_parsed, buf.len() - b"trailing".len());
}

#[test]
fn test_status_partial_with_parse_headers_incomplete() {
    let buf = b"Host: example.com\r\nAccept: */*\r\n";
    let mut headers = [EMPTY_HEADER; 4];
    let result = parse_headers(buf, &mut headers).unwrap();

    assert!(result.is_partial());
    assert!(!result.is_complete());


    let buf2 = b"Host: example.com\r\n\r\n";
    let mut headers2 = [EMPTY_HEADER; 4];
    let result2 = parse_headers(buf2, &mut headers2).unwrap();
    assert!(result2.is_complete());
    assert!(!result2.is_partial());
    let (_, hdrs) = result2.unwrap();
    assert_eq!(hdrs.len(), 1);
    assert_eq!(hdrs[0].name, "Host");
}

#[test]
fn test_status_unwrap_large_chunk_sizes() {

    let buf_1000 = b"3E8\r\n";
    let result = parse_chunk_size(buf_1000).unwrap();
    assert!(result.is_complete());
    let (bytes, size) = result.unwrap();
    assert_eq!(size, 1000);
    assert_eq!(bytes, 5);

    let buf_max = b"FFFFFFFF\r\n";
    let result2 = parse_chunk_size(buf_max).unwrap();
    assert!(result2.is_complete());
    assert!(!result2.is_partial());
    let (bytes2, size2) = result2.unwrap();
    assert_eq!(size2, 0xFFFFFFFF);
    assert_eq!(bytes2, 10);


    let buf_lower = b"ff\r\n";
    let result3 = parse_chunk_size(buf_lower).unwrap();
    assert!(result3.is_complete());
    let (_, size3) = result3.unwrap();
    assert_eq!(size3, 255);
}

#[test]
fn test_status_complete_and_partial_multi_step_workflow() {

    let partial1 = b"GET / HT";
    let partial2 = b"GET / HTTP/1.1\r\n";
    let complete = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";

    let mut headers1 = [EMPTY_HEADER; 16];
    let mut req1 = Request::new(&mut headers1);
    let r1 = req1.parse(partial1).unwrap();
    assert!(r1.is_partial());
    assert!(!r1.is_complete());

    let mut headers2 = [EMPTY_HEADER; 16];
    let mut req2 = Request::new(&mut headers2);
    let r2 = req2.parse(partial2).unwrap();
    assert!(r2.is_partial());
    assert!(!r2.is_complete());

    let mut headers3 = [EMPTY_HEADER; 16];
    let mut req3 = Request::new(&mut headers3);
    let r3 = req3.parse(complete).unwrap();
    assert!(r3.is_complete());
    assert!(!r3.is_partial());
    let consumed = r3.unwrap();
    assert_eq!(consumed, complete.len());
}

#[test]
fn test_status_unwrap_with_chunk_extensions() {

    let buf = b"a;ext=val\r\n";
    let result = parse_chunk_size(buf).unwrap();
    assert!(result.is_complete());
    assert!(!result.is_partial());
    let (bytes, size) = result.unwrap();
    assert_eq!(size, 10);
    assert_eq!(bytes, 11);


    let buf2 = b"1F;name=\"value\"\r\n";
    let result2 = parse_chunk_size(buf2).unwrap();
    assert!(result2.is_complete());
    let (bytes2, size2) = result2.unwrap();
    assert_eq!(size2, 31);
    assert_eq!(bytes2, buf2.len());
}