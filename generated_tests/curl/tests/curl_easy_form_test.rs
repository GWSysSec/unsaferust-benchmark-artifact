use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use curl::easy::{Easy, Form, List};

fn unique_temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    path.push(format!(
        "curl_integration_form_{}_{}_{}",
        std::process::id(),
        nanos,
        name
    ));
    path
}

#[test]
fn multipart_file_content_filename_and_content_header_can_be_configured() {
    curl::init();

    let upload_path = unique_temp_path("payload.txt");
    let payload = b"first line\nsecond line\n";
    fs::write(&upload_path, payload).expect("write upload payload");

    assert!(upload_path.exists(), "test payload file should exist");
    assert_eq!(
        fs::metadata(&upload_path).expect("metadata").len(),
        payload.len() as u64
    );

    let mut per_part_headers = List::new();
    per_part_headers
        .append("X-Part-Source: integration-test")
        .expect("append first content header");
    per_part_headers
        .append("Content-Transfer-Encoding: binary")
        .expect("append second content header");

    let mut form = Form::new();
    let add_result = form
        .part("document")
        .file_content(&upload_path)
        .filename("client-visible-name.txt")
        .content_header(per_part_headers)
        .add();

    assert!(
        add_result.is_ok(),
        "form part using file_content, filename, and content_header should be accepted: {:?}",
        add_result.err()
    );

    let mut easy = Easy::new();
    easy.url("http://example.invalid/upload")
        .expect("set upload url");
    easy.timeout(Duration::from_secs(1)).expect("set timeout");
    easy.post(true).expect("enable post");
    easy.httppost(form).expect("attach multipart form");

    let effective = easy.effective_url().expect("read effective url");
    assert_eq!(effective, Some("http://example.invalid/upload"));

    let encoded = easy.url_encode(b"client-visible-name.txt");
    assert!(
        encoded.contains("client-visible-name.txt"),
        "safe filename characters should remain readable after URL encoding"
    );

    let decoded = easy.url_decode(&encoded);
    assert_eq!(decoded, b"client-visible-name.txt");

    fs::remove_file(&upload_path).expect("remove upload payload");
}

#[test]
fn multiple_form_parts_can_mix_custom_headers_and_edge_case_filename() {
    curl::init();

    let empty_path = unique_temp_path("empty.bin");
    fs::write(&empty_path, []).expect("write empty upload payload");

    assert!(empty_path.exists(), "empty payload file should exist");
    assert_eq!(
        fs::metadata(&empty_path).expect("metadata").len(),
        0,
        "empty payload should really be empty"
    );

    let mut first_headers = List::new();
    first_headers
        .append("X-Empty-Part: yes")
        .expect("append custom header for empty file");

    let mut second_headers = List::new();
    second_headers
        .append("X-Normal-Field: yes")
        .expect("append custom header for text field");

    let mut form = Form::new();

    let empty_file_part = form
        .part("empty-file")
        .file_content(&empty_path)
        .filename("")
        .content_header(first_headers)
        .add();

    assert!(
        empty_file_part.is_ok(),
        "empty file content with an explicitly empty submitted filename should be accepted: {:?}",
        empty_file_part.err()
    );

    let text_part = form
        .part("description")
        .contents(b"multipart form built entirely through public API")
        .filename("description.txt")
        .content_header(second_headers)
        .add();

    assert!(
        text_part.is_ok(),
        "text part with filename and content_header should be accepted: {:?}",
        text_part.err()
    );

    let mut easy = Easy::new();
    easy.url("http://example.invalid/forms")
        .expect("set form target url");
    easy.useragent("curl-rust-integration-test/1.0")
        .expect("set user agent");
    easy.post(true).expect("enable post");
    easy.httppost(form).expect("attach multipart form");

    assert_eq!(
        easy.effective_url().expect("read effective url"),
        Some("http://example.invalid/forms")
    );
    assert_eq!(
        easy.response_code().expect("response code before perform"),
        0,
        "no transfer has run, so libcurl should report response code 0"
    );
    assert_eq!(
        easy.redirect_count().expect("redirect count before perform"),
        0,
        "no transfer has run, so there should be no redirects"
    );

    fs::remove_file(&empty_path).expect("remove empty upload payload");
}