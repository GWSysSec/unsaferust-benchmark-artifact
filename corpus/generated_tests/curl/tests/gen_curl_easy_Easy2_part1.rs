#[test]
fn test_curl_version_num() {
    let v = curl::Version::num();
    assert!(!v.is_empty());
    assert!(v.contains('.'));
}