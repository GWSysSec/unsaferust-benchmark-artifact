use headers::{ContentDisposition, Header};
use http::HeaderValue;

#[test]
fn test_content_disposition_inline_creation_and_checks() {
    let cd = ContentDisposition::inline();


    assert_eq!(cd.is_inline(), true);


    assert_eq!(cd.is_attachment(), false);


    assert_eq!(cd.is_form_data(), false);


    let mut values = Vec::new();
    cd.encode(&mut values);
    assert!(!values.is_empty());


    let encoded_str = values[0].to_str().unwrap();
    assert!(encoded_str.contains("inline"));


    let decoded = ContentDisposition::decode(&mut values.iter()).unwrap();
    assert_eq!(decoded.is_inline(), true);
    assert_eq!(decoded.is_attachment(), false);
    assert_eq!(decoded.is_form_data(), false);
}

#[test]
fn test_content_disposition_attachment_from_header_value() {
    let value = HeaderValue::from_static("attachment");
    let mut iter = std::iter::once(&value);
    let cd = ContentDisposition::decode(&mut iter).unwrap();

    assert_eq!(cd.is_attachment(), true);
    assert_eq!(cd.is_inline(), false);
    assert_eq!(cd.is_form_data(), false);


    let mut values = Vec::new();
    cd.encode(&mut values);
    assert!(!values.is_empty());

    let encoded_str = values[0].to_str().unwrap();
    assert!(encoded_str.contains("attachment"));

    let decoded = ContentDisposition::decode(&mut values.iter()).unwrap();
    assert_eq!(decoded.is_attachment(), true);
    assert_eq!(decoded.is_inline(), false);
}

#[test]
fn test_content_disposition_form_data_from_header_value() {
    let value = HeaderValue::from_static("form-data; name=\"field1\"");
    let mut iter = std::iter::once(&value);
    let cd = ContentDisposition::decode(&mut iter).unwrap();

    assert_eq!(cd.is_form_data(), true);
    assert_eq!(cd.is_inline(), false);
    assert_eq!(cd.is_attachment(), false);


    let mut values = Vec::new();
    cd.encode(&mut values);
    assert!(!values.is_empty());

    let encoded_str = values[0].to_str().unwrap();
    assert!(encoded_str.contains("form-data"));

    let decoded = ContentDisposition::decode(&mut values.iter()).unwrap();
    assert_eq!(decoded.is_form_data(), true);
    assert_eq!(decoded.is_inline(), false);
}

#[test]
fn test_content_disposition_attachment_with_filename() {
    let value = HeaderValue::from_static("attachment; filename=\"report.pdf\"");
    let mut iter = std::iter::once(&value);
    let cd = ContentDisposition::decode(&mut iter).unwrap();

    assert_eq!(cd.is_attachment(), true);
    assert_eq!(cd.is_inline(), false);
    assert_eq!(cd.is_form_data(), false);


    let mut values = Vec::new();
    cd.encode(&mut values);
    assert!(!values.is_empty());

    let encoded_str = values[0].to_str().unwrap();
    assert!(encoded_str.contains("attachment"));
    assert!(encoded_str.contains("filename"));
    assert!(encoded_str.contains("report.pdf"));
}

#[test]
fn test_content_disposition_inline_is_distinct_from_others() {
    let inline_cd = ContentDisposition::inline();

    let attachment_value = HeaderValue::from_static("attachment");
    let mut att_iter = std::iter::once(&attachment_value);
    let attachment_cd = ContentDisposition::decode(&mut att_iter).unwrap();

    let form_value = HeaderValue::from_static("form-data; name=\"file\"");
    let mut form_iter = std::iter::once(&form_value);
    let form_cd = ContentDisposition::decode(&mut form_iter).unwrap();


    assert_eq!(inline_cd.is_inline(), true);
    assert_eq!(inline_cd.is_attachment(), false);
    assert_eq!(inline_cd.is_form_data(), false);


    assert_eq!(attachment_cd.is_inline(), false);
    assert_eq!(attachment_cd.is_attachment(), true);
    assert_eq!(attachment_cd.is_form_data(), false);


    assert_eq!(form_cd.is_inline(), false);
    assert_eq!(form_cd.is_attachment(), false);
    assert_eq!(form_cd.is_form_data(), true);
}

#[test]
fn test_content_disposition_header_name() {
    let name = ContentDisposition::name();
    assert_eq!(name.as_str(), "content-disposition");

    let cd = ContentDisposition::inline();
    let mut values = Vec::new();
    cd.encode(&mut values);
    assert_eq!(values.len(), 1);


    let decoded = ContentDisposition::decode(&mut values.iter()).unwrap();
    assert_eq!(decoded.is_inline(), true);


    assert_eq!(ContentDisposition::name().as_str(), "content-disposition");
    assert_ne!(ContentDisposition::name().as_str(), "content-type");
}

#[test]
fn test_content_disposition_invalid_decode() {

    let values: Vec<HeaderValue> = Vec::new();
    let result = ContentDisposition::decode(&mut values.iter());
    assert!(result.is_err());


    let cd = ContentDisposition::inline();
    assert_eq!(cd.is_inline(), true);
    assert_eq!(cd.is_attachment(), false);
    assert_eq!(cd.is_form_data(), false);


    let bad_value = HeaderValue::from_static("inline");
    let mut good_iter = std::iter::once(&bad_value);
    let good_result = ContentDisposition::decode(&mut good_iter);

    assert!(good_result.is_ok());
    let parsed = good_result.unwrap();
    assert_eq!(parsed.is_inline(), true);
}

#[test]
fn test_content_disposition_multiple_inline_instances() {
    let cd1 = ContentDisposition::inline();
    let cd2 = ContentDisposition::inline();

    assert_eq!(cd1.is_inline(), true);
    assert_eq!(cd2.is_inline(), true);
    assert_eq!(cd1.is_attachment(), false);
    assert_eq!(cd2.is_attachment(), false);
    assert_eq!(cd1.is_form_data(), false);
    assert_eq!(cd2.is_form_data(), false);

    let mut values1 = Vec::new();
    let mut values2 = Vec::new();
    cd1.encode(&mut values1);
    cd2.encode(&mut values2);

    assert_eq!(values1.len(), values2.len());
    assert_eq!(values1[0].to_str().unwrap(), values2[0].to_str().unwrap());
}