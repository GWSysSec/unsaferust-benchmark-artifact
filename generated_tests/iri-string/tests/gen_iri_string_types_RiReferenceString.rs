use iri_string::types::{RiReferenceString, RiString, RiRelativeString};
use iri_string::spec::{IriSpec, UriSpec};

#[test]
fn test_into_iri_with_absolute_iri_reference() {

    let input = "http://example.com/path?query=value#fragment";
    let ref_string: RiReferenceString<IriSpec> =
        unsafe { RiReferenceString::new_unchecked(input.to_string()) };


    assert_eq!(ref_string.as_str(), input);
    assert_eq!(ref_string.len(), input.len());

    let result = ref_string.into_iri();
    assert!(result.is_ok());

    let iri_string: RiString<IriSpec> = result.unwrap();
    assert_eq!(iri_string.as_str(), input);
    assert_eq!(iri_string.len(), input.len());
    assert!(!iri_string.as_str().is_empty());
    assert!(iri_string.as_str().starts_with("http://"));
    assert!(iri_string.as_str().contains("example.com"));
    assert!(iri_string.as_str().ends_with("#fragment"));
}

#[test]
fn test_into_iri_with_relative_reference_fails() {

    let input = "/relative/path?query=1";
    let ref_string: RiReferenceString<IriSpec> =
        unsafe { RiReferenceString::new_unchecked(input.to_string()) };

    assert_eq!(ref_string.as_str(), input);

    let result = ref_string.into_iri();
    assert!(result.is_err());

    let relative: RiRelativeString<IriSpec> = result.unwrap_err();
    assert_eq!(relative.as_str(), input);
    assert_eq!(relative.len(), input.len());
    assert!(relative.as_str().starts_with("/relative"));
    assert!(relative.as_str().contains("query=1"));
    assert!(!relative.as_str().contains("://"));
    assert_ne!(relative.as_str(), "");
}

#[test]
fn test_into_relative_iri_with_relative_reference() {

    let input = "../other/path";
    let ref_string: RiReferenceString<UriSpec> =
        unsafe { RiReferenceString::new_unchecked(input.to_string()) };

    assert_eq!(ref_string.as_str(), input);
    assert_eq!(ref_string.len(), input.len());

    let result = ref_string.into_relative_iri();
    assert!(result.is_ok());

    let relative: RiRelativeString<UriSpec> = result.unwrap();
    assert_eq!(relative.as_str(), input);
    assert_eq!(relative.len(), input.len());
    assert!(relative.as_str().starts_with(".."));
    assert!(relative.as_str().contains("other/path"));
    assert!(!relative.as_str().contains("://"));
    assert_ne!(relative.as_str(), "/absolute");
}

#[test]
fn test_into_relative_iri_with_absolute_iri_fails() {

    let input = "https://user:pass@host.example:8080/p?q=v#f";
    let ref_string: RiReferenceString<UriSpec> =
        unsafe { RiReferenceString::new_unchecked(input.to_string()) };

    assert_eq!(ref_string.as_str(), input);

    let result = ref_string.into_relative_iri();
    assert!(result.is_err());

    let iri: RiString<UriSpec> = result.unwrap_err();
    assert_eq!(iri.as_str(), input);
    assert_eq!(iri.len(), input.len());
    assert!(iri.as_str().starts_with("https://"));
    assert!(iri.as_str().contains("host.example"));
    assert!(iri.as_str().contains(":8080"));
    assert!(iri.as_str().ends_with("#f"));
}

#[test]
fn test_remove_password_inline_with_password() {

    let input = "http://user:secret@example.com/path";
    let mut ref_string: RiReferenceString<IriSpec> =
        unsafe { RiReferenceString::new_unchecked(input.to_string()) };


    assert_eq!(ref_string.as_str(), input);
    assert!(ref_string.as_str().contains(":secret@"));
    assert!(ref_string.as_str().contains("user:"));
    assert_eq!(ref_string.len(), input.len());

    ref_string.remove_password_inline();


    assert!(!ref_string.as_str().contains("secret"));
    assert!(!ref_string.as_str().contains(":secret@"));
    assert!(ref_string.as_str().contains("example.com"));
    assert!(ref_string.as_str().contains("/path"));
    assert!(ref_string.as_str().starts_with("http://"));

    assert!(ref_string.as_str().contains("user"));
    assert_ne!(ref_string.as_str(), input);
    assert!(ref_string.len() < input.len());
}

#[test]
fn test_remove_password_inline_without_password() {
    let input = "http://user@example.com/path";
    let mut ref_string: RiReferenceString<IriSpec> =
        unsafe { RiReferenceString::new_unchecked(input.to_string()) };

    assert_eq!(ref_string.as_str(), input);
    assert!(ref_string.as_str().contains("user@"));
    assert!(!ref_string.as_str().contains("user:"));
    let original_len = ref_string.len();

    ref_string.remove_nonempty_password_inline();


    assert_eq!(ref_string.as_str(), input);
    assert_eq!(ref_string.len(), original_len);
    assert!(ref_string.as_str().contains("user@"));
    assert!(ref_string.as_str().contains("example.com"));
    assert!(ref_string.as_str().starts_with("http://"));
    assert!(ref_string.as_str().contains("/path"));
    assert!(!ref_string.as_str().is_empty());
    assert_ne!(ref_string.len(), 0);
}

#[test]
fn test_remove_nonempty_password_inline_with_nonempty_password() {

    let input = "http://admin:p4ssw0rd@host.example:443/secure";
    let mut ref_string: RiReferenceString<UriSpec> =
        unsafe { RiReferenceString::new_unchecked(input.to_string()) };


    assert_eq!(ref_string.as_str(), input);
    assert!(ref_string.as_str().contains(":p4ssw0rd@"));
    assert!(ref_string.as_str().contains("admin"));
    assert!(ref_string.as_str().contains(":443"));
    let original_len = ref_string.len();

    ref_string.remove_nonempty_password_inline();


    assert!(!ref_string.as_str().contains("p4ssw0rd"));
    assert!(!ref_string.as_str().contains(":p4ssw0rd@"));
    assert!(ref_string.as_str().contains("admin"));
    assert!(ref_string.as_str().contains("host.example"));
    assert!(ref_string.as_str().contains(":443"));
    assert!(ref_string.as_str().contains("/secure"));
    assert!(ref_string.as_str().starts_with("http://"));
    assert_ne!(ref_string.as_str(), input);
    assert!(ref_string.len() < original_len);
}