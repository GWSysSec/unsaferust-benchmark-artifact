use curl::easy::List;

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
        }
    };
}

fn collect_list_entries(list: &List) -> Vec<Vec<u8>> {
    list.iter()
        .map(|entry| {
            let bytes: &[u8] = entry.as_ref();
            bytes.to_vec()
        })
        .collect()
}

#[test]
fn iter_over_new_and_populated_lists_has_expected_semantics() {
    curl::init();

    let empty = List::new();
    assert_eq!(empty.iter().count(), 0);
    assert!(empty.iter().next().is_none());

    let mut headers = List::new();
    t!(headers.append("Accept: text/plain"));
    t!(headers.append("X-Test-Name: list-iter"));
    t!(headers.append("X-Empty-Value:"));

    let collected = collect_list_entries(&headers);
    assert_eq!(collected.len(), 3);
    assert_eq!(collected[0], b"Accept: text/plain");
    assert_eq!(collected[1], b"X-Test-Name: list-iter");
    assert_eq!(collected[2], b"X-Empty-Value:");

    let second_pass = collect_list_entries(&headers);
    assert_eq!(second_pass, collected);
}

#[test]
fn appended_list_preserves_iteration_order() {
    curl::init();

    let mut headers = List::new();
    t!(headers.append("X-First: one"));
    t!(headers.append("X-Second: two"));
    t!(headers.append("X-Third: three"));

    let expected = vec![
        b"X-First: one".to_vec(),
        b"X-Second: two".to_vec(),
        b"X-Third: three".to_vec(),
    ];

    let collected = collect_list_entries(&headers);
    assert_eq!(collected.len(), 3);
    assert_eq!(collected, expected);
    assert_eq!(headers.iter().count(), 3);

    let entries_as_strings: Vec<String> = headers
        .iter()
        .map(|entry| String::from_utf8(entry.as_ref().to_vec()).expect("header is valid UTF-8"))
        .collect();

    assert!(entries_as_strings.contains(&"X-First: one".to_string()));
    assert!(entries_as_strings.contains(&"X-Second: two".to_string()));
    assert!(entries_as_strings.contains(&"X-Third: three".to_string()));
}