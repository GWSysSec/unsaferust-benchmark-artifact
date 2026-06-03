use headers::{Connection, Header, HeaderMapExt};
use http::header::{HeaderMap, HeaderValue, CONNECTION};

#[test]
fn test_connection_close_creation_and_encoding() {
    let conn = Connection::close();


    let mut headers = HeaderMap::new();
    headers.typed_insert(conn.clone());

    let raw_value = headers.get(CONNECTION).expect("Connection header must be present");
    assert_eq!(raw_value, "close");


    let decoded: Connection = headers.typed_get().expect("Should decode Connection header");

    let mut headers2 = HeaderMap::new();
    headers2.typed_insert(decoded);
    let raw_value2 = headers2.get(CONNECTION).expect("Connection header must be present after round-trip");
    assert_eq!(raw_value2, "close");


    assert_ne!(raw_value, "keep-alive");
    assert_ne!(raw_value, "upgrade");


    assert_eq!(CONNECTION.as_str(), "connection");


    let cloned = Connection::close();
    let mut headers3 = HeaderMap::new();
    headers3.typed_insert(cloned);
    assert_eq!(headers3.get(CONNECTION).unwrap(), "close");


    assert_eq!(headers3.len(), 1);
}

#[test]
fn test_connection_keep_alive_creation_and_encoding() {
    let conn = Connection::keep_alive();

    let mut headers = HeaderMap::new();
    headers.typed_insert(conn);

    let raw_value = headers.get(CONNECTION).expect("Connection header must be present");
    assert_eq!(raw_value, "keep-alive");


    let decoded: Connection = headers.typed_get().expect("Should decode Connection keep-alive");
    let mut headers2 = HeaderMap::new();
    headers2.typed_insert(decoded);
    let raw_value2 = headers2.get(CONNECTION).expect("Must be present after round-trip");
    assert_eq!(raw_value2, "keep-alive");


    assert_ne!(raw_value, "close");
    assert_ne!(raw_value, "upgrade");


    assert_eq!(headers.len(), 1);
    assert_eq!(headers2.len(), 1);


    assert!(headers.contains_key(CONNECTION));
}

#[test]
fn test_connection_upgrade_creation_and_encoding() {
    let conn = Connection::upgrade();

    let mut headers = HeaderMap::new();
    headers.typed_insert(conn);

    let raw_value = headers.get(CONNECTION).expect("Connection header must be present");
    assert_eq!(raw_value, "upgrade");


    let decoded: Connection = headers.typed_get().expect("Should decode Connection upgrade");
    let mut headers2 = HeaderMap::new();
    headers2.typed_insert(decoded);
    let raw_value2 = headers2.get(CONNECTION).expect("Must be present after round-trip");
    assert_eq!(raw_value2, "upgrade");


    assert_ne!(raw_value, "close");
    assert_ne!(raw_value, "keep-alive");


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


    assert_ne!(v_close, v_keep);
    assert_ne!(v_close, v_upgrade);
    assert_ne!(v_keep, v_upgrade);


    assert_eq!(v_close, "close");
    assert_eq!(v_keep, "keep-alive");
    assert_eq!(v_upgrade, "upgrade");


    assert_eq!(h_close.len(), 1);
    assert_eq!(h_keep.len(), 1);
    assert_eq!(h_upgrade.len(), 1);
}

#[test]
fn test_connection_decode_from_raw_header_values() {

    let mut headers = HeaderMap::new();
    headers.insert(CONNECTION, HeaderValue::from_static("close"));
    let decoded: Connection = headers.typed_get().expect("Should decode 'close'");
    let mut re_encoded = HeaderMap::new();
    re_encoded.typed_insert(decoded);
    assert_eq!(re_encoded.get(CONNECTION).unwrap(), "close");


    let mut headers2 = HeaderMap::new();
    headers2.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    let decoded2: Connection = headers2.typed_get().expect("Should decode 'keep-alive'");
    let mut re_encoded2 = HeaderMap::new();
    re_encoded2.typed_insert(decoded2);
    assert_eq!(re_encoded2.get(CONNECTION).unwrap(), "keep-alive");


    let mut headers3 = HeaderMap::new();
    headers3.insert(CONNECTION, HeaderValue::from_static("upgrade"));
    let decoded3: Connection = headers3.typed_get().expect("Should decode 'upgrade'");
    let mut re_encoded3 = HeaderMap::new();
    re_encoded3.typed_insert(decoded3);
    assert_eq!(re_encoded3.get(CONNECTION).unwrap(), "upgrade");


    assert_eq!(re_encoded.len(), 1);
    assert_eq!(re_encoded2.len(), 1);
    assert_eq!(re_encoded3.len(), 1);
}

#[test]
fn test_connection_overwrite_in_header_map() {
    let mut headers = HeaderMap::new();


    headers.typed_insert(Connection::close());
    assert_eq!(headers.get(CONNECTION).unwrap(), "close");


    headers.typed_insert(Connection::keep_alive());
    assert_eq!(headers.get(CONNECTION).unwrap(), "keep-alive");
    assert_eq!(headers.len(), 1);


    headers.typed_insert(Connection::upgrade());
    assert_eq!(headers.get(CONNECTION).unwrap(), "upgrade");
    assert_eq!(headers.len(), 1);


    headers.typed_insert(Connection::close());
    assert_eq!(headers.get(CONNECTION).unwrap(), "close");
    assert_eq!(headers.len(), 1);


    let final_decoded: Connection = headers.typed_get().expect("Should decode final value");
    let mut final_map = HeaderMap::new();
    final_map.typed_insert(final_decoded);
    assert_eq!(final_map.get(CONNECTION).unwrap(), "close");
}

#[test]
fn test_connection_header_name_static() {

    let name = Connection::name();
    assert_eq!(name.as_str(), "connection");


    assert_eq!(name, CONNECTION);


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


    assert_eq!(h1.len(), 1);
    assert_eq!(h2.len(), 1);
    assert_eq!(h3.len(), 1);
}