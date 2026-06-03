use curl::easy::{Easy, List};

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
        }
    };
}

fn list_entries_as_bytes(list: &List) -> Vec<Vec<u8>> {
    list.iter()
        .map(|entry| {
            let bytes: &[u8] = entry.as_ref();
            bytes.to_vec()
        })
        .collect()
}

#[test]
fn list_iter_reports_empty_and_preserves_appended_header_order() {
    curl::init();

    let empty = List::new();
    assert_eq!(empty.iter().count(), 0, "new lists should iterate as empty");
    assert!(
        empty.iter().next().is_none(),
        "empty list iterator should have no first element"
    );

    let mut headers = List::new();
    t!(headers.append("Accept: text/plain"));
    t!(headers.append("X-Workflow-Step: list-iter"));
    t!(headers.append("X-Empty-Value:"));

    let collected = list_entries_as_bytes(&headers);
    assert_eq!(collected.len(), 3);
    assert_eq!(collected[0], b"Accept: text/plain");
    assert_eq!(collected[1], b"X-Workflow-Step: list-iter");
    assert_eq!(collected[2], b"X-Empty-Value:");

    let second_pass = list_entries_as_bytes(&headers);
    assert_eq!(
        second_pass, collected,
        "iterating a List should not consume or mutate it"
    );

    let mut iter = headers.iter();
    let first = iter.next().expect("first appended header should exist");
    let first_bytes: &[u8] = first.as_ref();
    assert_eq!(first_bytes, b"Accept: text/plain");

    assert!(
        iter.next().is_some(),
        "iterator should continue after the first header"
    );
    assert!(
        iter.next().is_some(),
        "iterator should include the third appended header"
    );
    assert!(
        iter.next().is_none(),
        "iterator should end after all appended headers"
    );
}

#[test]
fn list_iter_can_validate_configuration_before_moving_list_into_easy_handle() {
    curl::init();

    let mut headers = List::new();
    t!(headers.append("User-Agent: curl-rust-list-iter-test/1.0"));
    t!(headers.append("Accept: application/json"));
    t!(headers.append("X-Repeated: first"));
    t!(headers.append("X-Repeated: second"));

    let entries = list_entries_as_bytes(&headers);
    assert_eq!(entries.len(), 4);
    assert!(
        entries
            .iter()
            .any(|entry| entry.as_slice() == b"Accept: application/json"),
        "expected Accept header to be present before installing the list"
    );
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.starts_with(b"X-Repeated:"))
            .count(),
        2,
        "duplicate header names should remain as distinct list nodes"
    );
    assert_eq!(
        entries.first().map(Vec::as_slice),
        Some(&b"User-Agent: curl-rust-list-iter-test/1.0"[..])
    );
    assert_eq!(
        entries.last().map(Vec::as_slice),
        Some(&b"X-Repeated: second"[..])
    );

    let mut easy = Easy::new();
    t!(easy.url("http://example.invalid/list-iter"));
    t!(easy.http_headers(headers));
    t!(easy.useragent("curl-rust-integration-test"));

    assert_eq!(
        t!(easy.effective_url()),
        Some("http://example.invalid/list-iter"),
        "setting headers must not disturb the configured URL"
    );

    let encoded = easy.url_encode(b"header value with spaces");
    assert_eq!(encoded, "header%20value%20with%20spaces");

    let decoded = easy.url_decode(&encoded);
    assert_eq!(decoded, b"header value with spaces");
}