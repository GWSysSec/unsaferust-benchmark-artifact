use iri_string::types::RiReferenceStr;
use iri_string::types::RiStr;
use iri_string::types::RiRelativeStr;
use iri_string::spec::IriSpec;
use iri_string::spec::UriSpec;
use std::fmt::Write as _;

#[test]
fn test_ri_reference_str_to_iri_with_absolute_iri() {

    let input = "http://example.com/path?query=value#fragment";
    let ri_ref = RiReferenceStr::<IriSpec>::new(input).expect("valid IRI reference");

    let result = ri_ref.to_iri();
    assert!(result.is_ok(), "absolute IRI should return Ok from to_iri");

    let iri = result.unwrap();
    assert_eq!(iri.as_str(), input);


    let rel_result = ri_ref.to_relative_iri();
    assert!(rel_result.is_err(), "absolute IRI should return Err from to_relative_iri");

    let iri_from_err = rel_result.unwrap_err();
    assert_eq!(iri_from_err.as_str(), input);


    let iri_str: &RiStr<IriSpec> = RiStr::new(input).expect("valid IRI");
    assert_eq!(iri.as_str(), iri_str.as_str());


    let input2 = "https://user:pass@host.example.org:8080/a/b/c?x=1&y=2#frag";
    let ri_ref2 = RiReferenceStr::<IriSpec>::new(input2).expect("valid IRI reference");
    let result2 = ri_ref2.to_iri();
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap().as_str(), input2);
}

#[test]
fn test_ri_reference_str_to_iri_with_relative_reference() {

    let input = "../relative/path?query#frag";
    let ri_ref = RiReferenceStr::<IriSpec>::new(input).expect("valid IRI reference");

    let result = ri_ref.to_iri();
    assert!(result.is_err(), "relative reference should return Err from to_iri");

    let relative_str = result.unwrap_err();
    assert_eq!(relative_str.as_str(), input);


    let rel_result = ri_ref.to_relative_iri();
    assert!(rel_result.is_ok(), "relative reference should return Ok from to_relative_iri");

    let rel_iri = rel_result.unwrap();
    assert_eq!(rel_iri.as_str(), input);


    let direct_rel: &RiRelativeStr<IriSpec> = RiRelativeStr::new(input).expect("valid relative ref");
    assert_eq!(rel_iri.as_str(), direct_rel.as_str());


    let input2 = "/absolute/path";
    let ri_ref2 = RiReferenceStr::<IriSpec>::new(input2).expect("valid IRI reference");
    let result2 = ri_ref2.to_iri();
    assert!(result2.is_err(), "path-only reference is relative");
    assert_eq!(result2.unwrap_err().as_str(), input2);
}

#[test]
fn test_ri_reference_str_to_iri_uri_spec() {

    let input = "http://example.com/resource";
    let ri_ref = RiReferenceStr::<UriSpec>::new(input).expect("valid URI reference");

    let iri_result = ri_ref.to_iri();
    assert!(iri_result.is_ok());
    assert_eq!(iri_result.unwrap().as_str(), input);

    let rel_result = ri_ref.to_relative_iri();
    assert!(rel_result.is_err());
    assert_eq!(rel_result.unwrap_err().as_str(), input);


    let rel_input = "relative/path";
    let ri_ref_rel = RiReferenceStr::<UriSpec>::new(rel_input).expect("valid URI reference");

    let iri_result2 = ri_ref_rel.to_iri();
    assert!(iri_result2.is_err());
    assert_eq!(iri_result2.unwrap_err().as_str(), rel_input);

    let rel_result2 = ri_ref_rel.to_relative_iri();
    assert!(rel_result2.is_ok());
    assert_eq!(rel_result2.unwrap().as_str(), rel_input);
}

#[test]
fn test_ri_reference_str_to_relative_iri_various_forms() {

    let empty = "";
    let ri_ref = RiReferenceStr::<IriSpec>::new(empty).expect("empty is valid IRI reference");
    let rel_result = ri_ref.to_relative_iri();
    assert!(rel_result.is_ok());
    assert_eq!(rel_result.unwrap().as_str(), "");


    let query_only = "?query=value";
    let ri_ref2 = RiReferenceStr::<IriSpec>::new(query_only).expect("valid IRI reference");
    let rel_result2 = ri_ref2.to_relative_iri();
    assert!(rel_result2.is_ok());
    assert_eq!(rel_result2.unwrap().as_str(), query_only);


    let frag_only = "#fragment";
    let ri_ref3 = RiReferenceStr::<IriSpec>::new(frag_only).expect("valid IRI reference");
    let rel_result3 = ri_ref3.to_relative_iri();
    assert!(rel_result3.is_ok());
    assert_eq!(rel_result3.unwrap().as_str(), frag_only);


    let auth_rel = "//authority/path";
    let ri_ref4 = RiReferenceStr::<IriSpec>::new(auth_rel).expect("valid IRI reference");
    let rel_result4 = ri_ref4.to_relative_iri();
    assert!(rel_result4.is_ok());
    assert_eq!(rel_result4.unwrap().as_str(), auth_rel);


    assert!(ri_ref.to_iri().is_err());
    assert!(ri_ref2.to_iri().is_err());
    assert!(ri_ref3.to_iri().is_err());
}

#[test]
fn test_mask_password_with_userinfo() {

    let input = "http://user:secret@example.com/path";
    let ri_ref = RiReferenceStr::<IriSpec>::new(input).expect("valid IRI reference");

    let masked = ri_ref.mask_password();
    let masked_str = format!("{}", masked);


    assert!(!masked_str.contains("secret"), "password should be masked");
    assert!(masked_str.contains("user"), "username should be preserved");
    assert!(masked_str.contains("example.com"), "host should be preserved");
    assert!(masked_str.contains("/path"), "path should be preserved");
    assert!(masked_str.contains("http://"), "scheme should be preserved");


    assert!(masked_str.starts_with("http://"));


    assert!(masked_str.contains("@"));


    assert_ne!(masked_str, input);
}

#[test]
fn test_mask_password_without_password() {

    let input = "http://justuser@example.com/path";
    let ri_ref = RiReferenceStr::<IriSpec>::new(input).expect("valid IRI reference");

    let masked = ri_ref.mask_password();
    let masked_str = format!("{}", masked);


    assert!(masked_str.contains("justuser"), "username should be preserved");
    assert!(masked_str.contains("example.com"), "host should be preserved");
    assert!(masked_str.contains("/path"), "path should be preserved");
    assert_eq!(masked_str, input, "no password means no masking needed");


    let no_user = "http://example.com/path?q=1#f";
    let ri_ref2 = RiReferenceStr::<IriSpec>::new(no_user).expect("valid IRI reference");
    let masked2 = ri_ref2.mask_password();
    let masked_str2 = format!("{}", masked2);
    assert_eq!(masked_str2, no_user, "no userinfo means no change");


    let relative = "/just/a/path";
    let ri_ref3 = RiReferenceStr::<IriSpec>::new(relative).expect("valid IRI reference");
    let masked3 = ri_ref3.mask_password();
    let masked_str3 = format!("{}", masked3);
    assert_eq!(masked_str3, relative, "relative ref without authority unchanged");
}

#[test]
fn test_mask_password_replace_password() {
    use iri_string::mask_password::PasswordMasked;

    let input = "http://admin:p4ssw0rd@db.example.com:5432/mydb";
    let ri_ref = RiReferenceStr::<IriSpec>::new(input).expect("valid IRI reference");

    let masked = ri_ref.mask_password();


    let replaced = masked.replace_password("***REDACTED***");
    let replaced_str = format!("{}", replaced);

    assert!(replaced_str.contains("admin"), "username preserved");
    assert!(replaced_str.contains("***REDACTED***"), "custom replacement used");
    assert!(!replaced_str.contains("p4ssw0rd"), "original password removed");
    assert!(replaced_str.contains("db.example.com"), "host preserved");
    assert!(replaced_str.contains(":5432"), "port preserved");
    assert!(replaced_str.contains("/mydb"), "path preserved");
    assert!(replaced_str.starts_with("http://"));
    assert!(replaced_str.contains("@"));
}

#[test]
fn test_mask_password_uri_spec() {

    let input = "https://deploy:token123@registry.example.com/v2/image/manifests/latest";
    let ri_ref = RiReferenceStr::<UriSpec>::new(input).expect("valid URI reference");

    let masked = ri_ref.mask_password();
    let masked_str = format!("{}", masked);

    assert!(!masked_str.contains("token123"), "password should be masked");
    assert!(masked_str.contains("deploy"), "username should be preserved");
    assert!(masked_str.contains("registry.example.com"), "host preserved");
    assert!(masked_str.contains("/v2/image/manifests/latest"), "path preserved");
    assert!(masked_str.starts_with("https://"));
    assert!(masked_str.contains("@"));
    assert_ne!(masked_str, input);


    let replaced = masked.replace_password("[hidden]");
    let replaced_str = format!("{}", replaced);
    assert!(replaced_str.contains("[hidden]"));
}

#[test]
fn test_to_iri_and_to_relative_iri_are_complementary() {

    let test_cases: Vec<(&str, bool)> = vec![
        ("http://example.com", true),
        ("ftp://files.example.com/f", true),
        ("urn:isbn:0451450523", true),
        ("../relative", false),
        ("//authority/path", false),
        ("?query", false),
        ("#frag", false),
        ("", false),
    ];

    for (input, is_absolute) in &test_cases {
        let ri_ref = RiReferenceStr::<IriSpec>::new(input)
            .unwrap_or_else(|_| panic!("should be valid IRI reference: {:?}", input));

        let iri_result = ri_ref.to_iri();
        let rel_result = ri_ref.to_relative_iri();

        if *is_absolute {
            assert!(iri_result.is_ok(), "expected Ok for to_iri on {:?}", input);
            assert!(rel_result.is_err(), "expected Err for to_relative_iri on {:?}", input);
        } else {
            assert!(iri_result.is_err(), "expected Err for to_iri on {:?}", input);
            assert!(rel_result.is_ok(), "expected Ok for to_relative_iri on {:?}", input);
        }
    }
}