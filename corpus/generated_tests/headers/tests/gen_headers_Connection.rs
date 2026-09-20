use headers::{Connection, Header, HeaderMapExt};
use http::header::{HeaderMap, HeaderValue, CONNECTION};

#[test]
fn test_connection_close_creation_and_encoding() {
    let conn = Connection::close();

    // Encode the header into a HeaderMap to inspect its value
    let mut headers = HeaderMap::new();
    headers.typed_insert(conn.clone());

    let raw_value = headers.get(CONNECTION).expect("Connection header must be present");
    assert_eq!(raw_value, "close");

    // Decode it back
    let decoded: Connection = headers.typed_get().expect("Should decode Connection header");
    // Re-encode to verify round-trip
    let mut headers2 = HeaderMap::new();
    headers2.typed_insert(decoded);
    let raw_value2 = headers2.get(CONNECTION).expect("Connection header must be present after round-trip");
    assert_eq!(raw_value2, "close");

    // Verify it's not keep-alive or upgrade
    assert_ne!(raw_value, "keep-alive");
    assert_ne!(raw_value, "upgrade");

    // Verify the header name used is correct
    assert_eq!(CONNECTION.as_str(), "connection");

    // Verify we can clone it
    let cloned = Connection::close();
    let mut headers3 = HeaderMap::new();
    headers3.typed_insert(cloned);
    assert_eq!(headers3.get(CONNECTION).unwrap(), "close");

    // Verify header count is exactly 1
    assert_eq!(headers3.len(), 1);
}

#[test]
fn test_connection_keep_alive_creation_and_encoding() {
    let conn = Connection::keep_alive();

    let mut headers = HeaderMap::new();
    headers.typed_insert(conn);

    let raw_value = headers.get(CONNECTION).expect("Connection header must be present");
    assert_eq!(raw_value, "keep-alive");

    // Decode it back from the header map
    let decoded: Connection = headers.typed_get().expect("Should decode Connection keep-alive");
    let mut headers2 = HeaderMap::new();
    headers2.typed_insert(decoded);
    let raw_value2 = headers2.get(CONNECTION).expect("Must be present after round-trip");
    assert_eq!(raw_value2, "keep-alive");

    // Verify it's not close or upgrade
    assert_ne!(raw_value, "close");
    assert_ne!(raw_value, "upgrade");

    // Verify header map has exactly one entry
    assert_eq!(headers.len(), 1);
    assert_eq!(headers2.len(), 1);

    // Verify the name is connection
    assert!(headers.contains_key(CONNECTION));
}

#[test]
fn test_connection_upgrade_creation_and_encoding() {
    let conn = Connection::upgrade();

    let mut headers = HeaderMap::new();
    headers.typed_insert(conn);

    let raw_value = headers.get(CONNECTION).expect("Connection header must be present");
    assert_eq!(raw_value, "upgrade");

    // Decode round-trip
    let decoded: Connection = headers.typed_get().expect("Should decode Connection upgrade");
    let mut headers2 = HeaderMap::new();
    headers2.typed_insert(decoded);
    let raw_value2 = headers2.get(CONNECTION).expect("Must be present after round-trip");
    assert_eq!(raw_value2, "upgrade");

    // Verify it's not close or keep-alive
    assert_ne!(raw_value, "close");
    assert_ne!(raw_value, "keep-alive");

    // Verify header map structure
    assert_eq!(headers.len(), 1);
    assert_eq!(headers2.len(), 1);
    assert!(headers.contains_key(CONNECTION));
}

#[test]
fn test_connection_variants_are_distinct() {
    let close = Connection::close();
    let keep_alive = Connection::keep_alive();
    let upgrade = Connection::upgrade();

    let mut h_close = HeaderMap::new();
    h_close.typed_insert(close);

    let mut h_keep = HeaderMap::new();
    h_keep.typed_insert(keep_alive);

    let mut h_upgrade = HeaderMap::new();
    h_upgrade.typed_insert(upgrade);

    let v_close = h_close.get(CONNECTION).unwrap();
    let v_keep = h_keep.get(CONNECTION).unwrap();
    let v_upgrade = h_upgrade.get(CONNECTION).unwrap();

    // All three are distinct from each other
    assert_ne!(v_close, v_keep);
    assert_ne!(v_close, v_upgrade);
    assert_ne!(v_keep, v_upgrade);

    // Verify exact values
    assert_eq!(v_close, "close");
    assert_eq!(v_keep, "keep-alive");
    assert_eq!(v_upgrade, "upgrade");

    // Verify all maps have exactly one entry
    assert_eq!(h_close.len(), 1);
    assert_eq!(h_keep.len(), 1);
    assert_eq!(h_upgrade.len(), 1);
}

#[test]
fn test_connection_decode_from_raw_header_values() {
    // Test decoding close from raw header value
    let mut headers = HeaderMap::new();
    headers.insert(CONNECTION, HeaderValue::from_static("close"));
    let decoded: Connection = headers.typed_get().expect("Should decode 'close'");
    let mut re_encoded = HeaderMap::new();
    re_encoded.typed_insert(decoded);
    assert_eq!(re_encoded.get(CONNECTION).unwrap(), "close");

    // Test decoding keep-alive from raw header value
    let mut headers2 = HeaderMap::new();
    headers2.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    let decoded2: Connection = headers2.typed_get().expect("Should decode 'keep-alive'");
    let mut re_encoded2 = HeaderMap::new();
    re_encoded2.typed_insert(decoded2);
    assert_eq!(re_encoded2.get(CONNECTION).unwrap(), "keep-alive");

    // Test decoding upgrade from raw header value
    let mut headers3 = HeaderMap::new();
    headers3.insert(CONNECTION, HeaderValue::from_static("upgrade"));
    let decoded3: Connection = headers3.typed_get().expect("Should decode 'upgrade'");
    let mut re_encoded3 = HeaderMap::new();
    re_encoded3.typed_insert(decoded3);
    assert_eq!(re_encoded3.get(CONNECTION).unwrap(), "upgrade");

    // Verify all re-encoded maps have one entry
    assert_eq!(re_encoded.len(), 1);
    assert_eq!(re_encoded2.len(), 1);
    assert_eq!(re_encoded3.len(), 1);
}

#[test]
fn test_connection_overwrite_in_header_map() {
    let mut headers = HeaderMap::new();

    // Insert close first
    headers.typed_insert(Connection::close());
    assert_eq!(headers.get(CONNECTION).unwrap(), "close");

    // Overwrite with keep-alive
    headers.typed_insert(Connection::keep_alive());
    assert_eq!(headers.get(CONNECTION).unwrap(), "keep-alive");
    assert_eq!(headers.len(), 1); // Should still be one entry, not two

    // Overwrite with upgrade
    headers.typed_insert(Connection::upgrade());
    assert_eq!(headers.get(CONNECTION).unwrap(), "upgrade");
    assert_eq!(headers.len(), 1);

    // Overwrite back to close
    headers.typed_insert(Connection::close());
    assert_eq!(headers.get(CONNECTION).unwrap(), "close");
    assert_eq!(headers.len(), 1);

    // Final verification of the decoded typed header
    let final_decoded: Connection = headers.typed_get().expect("Should decode final value");
    let mut final_map = HeaderMap::new();
    final_map.typed_insert(final_decoded);
    assert_eq!(final_map.get(CONNECTION).unwrap(), "close");
}

#[test]
fn test_connection_header_name_static() {
    // Verify the Header trait gives us the correct header name
    let name = Connection::name();
    assert_eq!(name.as_str(), "connection");

    // Verify it matches the http crate's CONNECTION constant
    assert_eq!(name, CONNECTION);

    // Create all variants and verify they all use the same header name
    let mut h1 = HeaderMap::new();
    h1.typed_insert(Connection::close());
    assert!(h1.contains_key(CONNECTION));
    assert!(h1.contains_key("connection"));

    let mut h2 = HeaderMap::new();
    h2.typed_insert(Connection::keep_alive());
    assert!(h2.contains_key(CONNECTION));

    let mut h3 = HeaderMap::new();
    h3.typed_insert(Connection::upgrade());
    assert!(h3.contains_key(CONNECTION));

    // Verify none of them accidentally set other headers
    assert_eq!(h1.len(), 1);
    assert_eq!(h2.len(), 1);
    assert_eq!(h3.len(), 1);
}