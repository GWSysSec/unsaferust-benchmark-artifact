use headers::{ContentType, Header};

#[test]
fn test_content_type_text() {
    let ct = ContentType::text();
    let name = ContentType::name();
    assert_eq!(name.as_str(), "content-type");


    let mut encoded = vec![];
    ct.encode(&mut encoded);
    assert_eq!(encoded.len(), 1);
    let value_str = encoded[0].to_str().unwrap();
    assert_eq!(value_str, "text/plain");


    let decoded = ContentType::decode(&mut encoded.iter()).unwrap();
    let mut re_encoded = vec![];
    decoded.encode(&mut re_encoded);
    assert_eq!(re_encoded.len(), 1);
    assert_eq!(re_encoded[0].to_str().unwrap(), "text/plain");


    let json_ct = ContentType::json();
    let mut json_encoded = vec![];
    json_ct.encode(&mut json_encoded);
    assert_ne!(json_encoded[0].to_str().unwrap(), value_str);


    let html_ct = ContentType::html();
    let mut html_encoded = vec![];
    html_ct.encode(&mut html_encoded);
    assert_ne!(html_encoded[0].to_str().unwrap(), value_str);
}

#[test]
fn test_content_type_text_utf8() {
    let ct = ContentType::text_utf8();
    let mut encoded = vec![];
    ct.encode(&mut encoded);
    assert_eq!(encoded.len(), 1);
    let value_str = encoded[0].to_str().unwrap();
    assert_eq!(value_str, "text/plain; charset=utf-8");


    let plain = ContentType::text();
    let mut plain_encoded = vec![];
    plain.encode(&mut plain_encoded);
    assert_ne!(plain_encoded[0].to_str().unwrap(), value_str);


    let decoded = ContentType::decode(&mut encoded.iter()).unwrap();
    let mut re_encoded = vec![];
    decoded.encode(&mut re_encoded);
    assert_eq!(re_encoded[0].to_str().unwrap(), "text/plain; charset=utf-8");


    assert_eq!(ContentType::name().as_str(), "content-type");


    assert!(value_str.contains("text"));
    assert!(value_str.contains("utf-8"));
    assert!(value_str.contains("plain"));
}

#[test]
fn test_content_type_html() {
    let ct = ContentType::html();
    let mut encoded = vec![];
    ct.encode(&mut encoded);
    assert_eq!(encoded.len(), 1);
    let value_str = encoded[0].to_str().unwrap();
    assert_eq!(value_str, "text/html");


    let decoded = ContentType::decode(&mut encoded.iter()).unwrap();
    let mut re_encoded = vec![];
    decoded.encode(&mut re_encoded);
    assert_eq!(re_encoded[0].to_str().unwrap(), "text/html");


    let text_ct = ContentType::text();
    let mut text_encoded = vec![];
    text_ct.encode(&mut text_encoded);
    assert_ne!(text_encoded[0].to_str().unwrap(), value_str);


    let xml_ct = ContentType::xml();
    let mut xml_encoded = vec![];
    xml_ct.encode(&mut xml_encoded);
    assert_ne!(xml_encoded[0].to_str().unwrap(), value_str);

    assert!(value_str.starts_with("text/"));
    assert!(value_str.contains("html"));
}

#[test]
fn test_content_type_xml() {
    let ct = ContentType::xml();
    let mut encoded = vec![];
    ct.encode(&mut encoded);
    assert_eq!(encoded.len(), 1);
    let value_str = encoded[0].to_str().unwrap();
    assert_eq!(value_str, "text/xml");


    let decoded = ContentType::decode(&mut encoded.iter()).unwrap();
    let mut re_encoded = vec![];
    decoded.encode(&mut re_encoded);
    assert_eq!(re_encoded[0].to_str().unwrap(), "text/xml");


    let html_ct = ContentType::html();
    let mut html_encoded = vec![];
    html_ct.encode(&mut html_encoded);
    assert_ne!(html_encoded[0].to_str().unwrap(), value_str);


    let json_ct = ContentType::json();
    let mut json_encoded = vec![];
    json_ct.encode(&mut json_encoded);
    assert_ne!(json_encoded[0].to_str().unwrap(), value_str);

    assert!(value_str.starts_with("text/"));
    assert!(value_str.ends_with("xml"));
}

#[test]
fn test_content_type_form_url_encoded() {
    let ct = ContentType::form_url_encoded();
    let mut encoded = vec![];
    ct.encode(&mut encoded);
    assert_eq!(encoded.len(), 1);
    let value_str = encoded[0].to_str().unwrap();
    assert_eq!(value_str, "application/x-www-form-urlencoded");


    let decoded = ContentType::decode(&mut encoded.iter()).unwrap();
    let mut re_encoded = vec![];
    decoded.encode(&mut re_encoded);
    assert_eq!(re_encoded[0].to_str().unwrap(), "application/x-www-form-urlencoded");


    let json_ct = ContentType::json();
    let mut json_encoded = vec![];
    json_ct.encode(&mut json_encoded);
    assert_ne!(json_encoded[0].to_str().unwrap(), value_str);

    assert!(value_str.starts_with("application/"));
    assert!(value_str.contains("form"));
    assert!(value_str.contains("urlencoded"));
}

#[test]
fn test_content_type_jpeg() {
    let ct = ContentType::jpeg();
    let mut encoded = vec![];
    ct.encode(&mut encoded);
    assert_eq!(encoded.len(), 1);
    let value_str = encoded[0].to_str().unwrap();
    assert_eq!(value_str, "image/jpeg");


    let decoded = ContentType::decode(&mut encoded.iter()).unwrap();
    let mut re_encoded = vec![];
    decoded.encode(&mut re_encoded);
    assert_eq!(re_encoded[0].to_str().unwrap(), "image/jpeg");


    let png_ct = ContentType::png();
    let mut png_encoded = vec![];
    png_ct.encode(&mut png_encoded);
    assert_ne!(png_encoded[0].to_str().unwrap(), value_str);

    assert!(value_str.starts_with("image/"));
    assert!(value_str.contains("jpeg"));
    assert_ne!(value_str, "image/png");
}

#[test]
fn test_content_type_png() {
    let ct = ContentType::png();
    let mut encoded = vec![];
    ct.encode(&mut encoded);
    assert_eq!(encoded.len(), 1);
    let value_str = encoded[0].to_str().unwrap();
    assert_eq!(value_str, "image/png");


    let decoded = ContentType::decode(&mut encoded.iter()).unwrap();
    let mut re_encoded = vec![];
    decoded.encode(&mut re_encoded);
    assert_eq!(re_encoded[0].to_str().unwrap(), "image/png");


    let jpeg_ct = ContentType::jpeg();
    let mut jpeg_encoded = vec![];
    jpeg_ct.encode(&mut jpeg_encoded);
    assert_ne!(jpeg_encoded[0].to_str().unwrap(), value_str);

    assert!(value_str.starts_with("image/"));
    assert!(value_str.contains("png"));
    assert_ne!(value_str, "image/jpeg");
}

#[test]
fn test_content_type_octet_stream() {
    let ct = ContentType::octet_stream();
    let mut encoded = vec![];
    ct.encode(&mut encoded);
    assert_eq!(encoded.len(), 1);
    let value_str = encoded[0].to_str().unwrap();
    assert_eq!(value_str, "application/octet-stream");


    let decoded = ContentType::decode(&mut encoded.iter()).unwrap();
    let mut re_encoded = vec![];
    decoded.encode(&mut re_encoded);
    assert_eq!(re_encoded[0].to_str().unwrap(), "application/octet-stream");


    let json_ct = ContentType::json();
    let mut json_encoded = vec![];
    json_ct.encode(&mut json_encoded);
    assert_ne!(json_encoded[0].to_str().unwrap(), value_str);

    assert!(value_str.starts_with("application/"));
    assert!(value_str.contains("octet-stream"));
    assert_ne!(value_str, "application/json");
}

#[test]
fn test_content_type_all_variants_unique() {
    let variants: Vec<ContentType> = vec![
        ContentType::text(),
        ContentType::text_utf8(),
        ContentType::html(),
        ContentType::xml(),
        ContentType::form_url_encoded(),
        ContentType::jpeg(),
        ContentType::png(),
        ContentType::octet_stream(),
        ContentType::json(),
    ];

    let mut values: Vec<String> = Vec::new();
    for ct in &variants {
        let mut encoded = vec![];
        ct.encode(&mut encoded);
        values.push(encoded[0].to_str().unwrap().to_string());
    }


    assert_eq!(values.len(), 9);
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            assert_ne!(values[i], values[j], "Duplicate content types at index {} and {}", i, j);
        }
    }


    assert_eq!(values[0], "text/plain");
    assert_eq!(values[1], "text/plain; charset=utf-8");
    assert_eq!(values[2], "text/html");
    assert_eq!(values[3], "text/xml");
    assert_eq!(values[4], "application/x-www-form-urlencoded");
    assert_eq!(values[5], "image/jpeg");
    assert_eq!(values[6], "image/png");
    assert_eq!(values[7], "application/octet-stream");
    assert_eq!(values[8], "application/json");
}

#[test]
fn test_content_type_decode_from_raw_header_values() {

    let test_cases = vec![
        ("text/plain", "text/plain"),
        ("text/html", "text/html"),
        ("text/xml", "text/xml"),
        ("image/jpeg", "image/jpeg"),
        ("image/png", "image/png"),
        ("application/octet-stream", "application/octet-stream"),
        ("application/x-www-form-urlencoded", "application/x-www-form-urlencoded"),
        ("application/json", "application/json"),
    ];

    for (input, expected) in &test_cases {
        let header_value = http::header::HeaderValue::from_static(input);
        let values = vec![header_value];
        let decoded = ContentType::decode(&mut values.iter()).unwrap();
        let mut re_encoded = vec![];
        decoded.encode(&mut re_encoded);
        assert_eq!(re_encoded[0].to_str().unwrap(), *expected);
    }


    let bad_value = http::header::HeaderValue::from_static("not a valid mime");
    let bad_values = vec![bad_value];
    let result = ContentType::decode(&mut bad_values.iter());
    assert!(result.is_err());
}