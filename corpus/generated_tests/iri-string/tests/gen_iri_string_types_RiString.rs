use iri_string::types::{RiStr, RiString, RiAbsoluteStr, RiAbsoluteString, RiFragmentStr};
use iri_string::spec::IriSpec;
use iri_string::spec::UriSpec;

#[test]
fn test_ri_string_into_absolute_and_fragment_with_fragment() {
    let iri_str = "http://example.com/path?query=1#fragment-part";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let ri_string: RiString<IriSpec> = ri_str.to_owned();

    let (absolute, fragment_opt) = ri_string.into_absolute_and_fragment();

    // The absolute part should not contain the fragment
    let absolute_str = absolute.as_str();
    assert_eq!(absolute_str, "http://example.com/path?query=1");
    assert!(!absolute_str.contains('#'));

    // The fragment should be present
    let fragment = fragment_opt.expect("should have a fragment");
    assert_eq!(fragment.as_str(), "fragment-part");

    // Validate the absolute IRI is valid
    let reparse = RiAbsoluteStr::<IriSpec>::new(absolute_str);
    assert!(reparse.is_ok());

    // Validate the fragment content
    let frag_str = RiFragmentStr::<IriSpec>::new(fragment.as_str());
    assert!(frag_str.is_ok());

    // Check that the original can be reconstructed
    let reconstructed = format!("{}#{}", absolute_str, fragment.as_str());
    assert_eq!(reconstructed, "http://example.com/path?query=1#fragment-part");

    // Verify absolute part preserves scheme
    assert!(absolute_str.starts_with("http://"));

    // Verify absolute part preserves query
    assert!(absolute_str.contains("?query=1"));
}

#[test]
fn test_ri_string_into_absolute_and_fragment_without_fragment() {
    let iri_str = "http://example.com/no-fragment";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let ri_string: RiString<IriSpec> = ri_str.to_owned();

    let (absolute, fragment_opt) = ri_string.into_absolute_and_fragment();

    assert_eq!(absolute.as_str(), "http://example.com/no-fragment");
    assert!(fragment_opt.is_none());

    // Validate the absolute IRI
    let reparse = RiAbsoluteStr::<IriSpec>::new(absolute.as_str());
    assert!(reparse.is_ok());

    // Ensure no hash in the result
    assert!(!absolute.as_str().contains('#'));

    // Verify path is preserved
    assert!(absolute.as_str().contains("/no-fragment"));

    // Verify scheme is preserved
    assert!(absolute.as_str().starts_with("http://"));

    // Verify host is preserved
    assert!(absolute.as_str().contains("example.com"));

    // Test with empty path
    let iri_str2 = "http://example.com";
    let ri_str2 = RiStr::<IriSpec>::new(iri_str2).expect("valid IRI");
    let ri_string2: RiString<IriSpec> = ri_str2.to_owned();
    let (absolute2, frag2) = ri_string2.into_absolute_and_fragment();
    assert_eq!(absolute2.as_str(), "http://example.com");
    assert!(frag2.is_none());
}

#[test]
fn test_ri_string_into_absolute_and_fragment_empty_fragment() {
    // An IRI with an empty fragment (just the '#')
    let iri_str = "http://example.com/path#";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let ri_string: RiString<IriSpec> = ri_str.to_owned();

    let (absolute, fragment_opt) = ri_string.into_absolute_and_fragment();

    assert_eq!(absolute.as_str(), "http://example.com/path");
    // Empty fragment should still be Some
    let fragment = fragment_opt.expect("empty fragment should still be Some");
    assert_eq!(fragment.as_str(), "");

    // Validate absolute part
    let reparse = RiAbsoluteStr::<IriSpec>::new(absolute.as_str());
    assert!(reparse.is_ok());

    // Verify no hash in absolute
    assert!(!absolute.as_str().contains('#'));

    // Verify path preserved
    assert!(absolute.as_str().ends_with("/path"));

    // Verify fragment is truly empty
    assert!(fragment.as_str().is_empty());

    // Verify scheme
    assert!(absolute.as_str().starts_with("http"));

    // Verify host
    assert!(absolute.as_str().contains("example.com"));
}

#[test]
fn test_ri_string_into_absolute() {
    let iri_str = "http://example.com/resource?key=value#sec";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let ri_string: RiString<IriSpec> = ri_str.to_owned();

    let absolute: RiAbsoluteString<IriSpec> = ri_string.into_absolute();

    // Fragment should be stripped
    assert_eq!(absolute.as_str(), "http://example.com/resource?key=value");
    assert!(!absolute.as_str().contains('#'));
    assert!(!absolute.as_str().contains("sec"));

    // Validate it's a proper absolute IRI
    let reparse = RiAbsoluteStr::<IriSpec>::new(absolute.as_str());
    assert!(reparse.is_ok());

    // Verify query preserved
    assert!(absolute.as_str().contains("?key=value"));

    // Verify path preserved
    assert!(absolute.as_str().contains("/resource"));

    // Verify scheme preserved
    assert!(absolute.as_str().starts_with("http://"));

    // Verify host preserved
    assert!(absolute.as_str().contains("example.com"));
}

#[test]
fn test_ri_string_into_absolute_no_fragment() {
    let iri_str = "https://example.org/already-absolute";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let ri_string: RiString<IriSpec> = ri_str.to_owned();

    let absolute: RiAbsoluteString<IriSpec> = ri_string.into_absolute();

    assert_eq!(absolute.as_str(), "https://example.org/already-absolute");
    assert!(!absolute.as_str().contains('#'));

    let reparse = RiAbsoluteStr::<IriSpec>::new(absolute.as_str());
    assert!(reparse.is_ok());

    assert!(absolute.as_str().starts_with("https://"));
    assert!(absolute.as_str().contains("example.org"));
    assert!(absolute.as_str().contains("/already-absolute"));

    // Test with URI spec as well
    let uri_str = "http://example.com/uri-test";
    let ri_str_uri = RiStr::<UriSpec>::new(uri_str).expect("valid URI");
    let ri_string_uri: RiString<UriSpec> = ri_str_uri.to_owned();
    let absolute_uri = ri_string_uri.into_absolute();
    assert_eq!(absolute_uri.as_str(), "http://example.com/uri-test");
    assert!(!absolute_uri.as_str().contains('#'));
}

#[test]
fn test_ri_string_remove_password_inline() {
    let iri_str = "http://user:password@example.com/path";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let mut ri_string: RiString<IriSpec> = ri_str.to_owned();

    // Pre-state: password is present
    assert!(ri_string.as_str().contains(":password@"));
    assert!(ri_string.as_str().contains("user"));
    assert_eq!(ri_string.as_str(), "http://user:password@example.com/path");

    ri_string.remove_password_inline();

    // Post-state: password is removed
    let result = ri_string.as_str();
    assert!(!result.contains("password"));
    assert!(!result.contains(":password"));

    // The user should still be present
    assert!(result.contains("user"));

    // The host should still be present
    assert!(result.contains("example.com"));

    // The path should still be present
    assert!(result.contains("/path"));

    // The scheme should still be present
    assert!(result.starts_with("http://"));

    // It should still be a valid IRI
    let reparse = RiStr::<IriSpec>::new(result);
    assert!(reparse.is_ok());
}

#[test]
fn test_ri_string_remove_password_inline_empty_password() {
    // Empty password (user:@host)
    let iri_str = "http://user:@example.com/path";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let mut ri_string: RiString<IriSpec> = ri_str.to_owned();

    // Pre-state
    assert!(ri_string.as_str().contains("user:@"));

    ri_string.remove_password_inline();

    let result = ri_string.as_str();
    // Should still be valid
    let reparse = RiStr::<IriSpec>::new(result);
    assert!(reparse.is_ok());

    // Host preserved
    assert!(result.contains("example.com"));

    // Path preserved
    assert!(result.contains("/path"));

    // Scheme preserved
    assert!(result.starts_with("http://"));

    // User should still be present
    assert!(result.contains("user"));

    // Should not have the colon-at pattern for empty password
    // (the empty password colon should be removed)
    assert!(!result.contains(":@"));

    // Verify overall structure
    assert!(result.contains("@example.com") || result.contains("example.com"));
}

#[test]
fn test_ri_string_remove_password_inline_no_userinfo() {
    let iri_str = "http://example.com/path";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let mut ri_string: RiString<IriSpec> = ri_str.to_owned();

    // Pre-state: no userinfo at all
    let pre = ri_string.as_str().to_owned();
    assert!(!pre.contains('@'));

    ri_string.remove_password_inline();

    // Post-state: should be unchanged
    let result = ri_string.as_str();
    assert_eq!(result, pre);
    assert_eq!(result, "http://example.com/path");
    assert!(!result.contains('@'));

    let reparse = RiStr::<IriSpec>::new(result);
    assert!(reparse.is_ok());

    assert!(result.starts_with("http://"));
    assert!(result.contains("example.com"));
}

#[test]
fn test_ri_string_remove_nonempty_password_inline() {
    let iri_str = "http://user:secret@example.com/resource";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let mut ri_string: RiString<IriSpec> = ri_str.to_owned();

    // Pre-state
    assert!(ri_string.as_str().contains(":secret@"));
    assert_eq!(ri_string.as_str(), "http://user:secret@example.com/resource");

    ri_string.remove_nonempty_password_inline();

    let result = ri_string.as_str();
    // Password should be removed
    assert!(!result.contains("secret"));

    // User should remain
    assert!(result.contains("user"));

    // Host should remain
    assert!(result.contains("example.com"));

    // Path should remain
    assert!(result.contains("/resource"));

    // Scheme should remain
    assert!(result.starts_with("http://"));

    // Should still be valid
    let reparse = RiStr::<IriSpec>::new(result);
    assert!(reparse.is_ok());
}

#[test]
fn test_ri_string_remove_nonempty_password_inline_empty_password_preserved() {
    // With empty password, remove_nonempty_password_inline should NOT remove it
    let iri_str = "http://user:@example.com/path";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let mut ri_string: RiString<IriSpec> = ri_str.to_owned();

    // Pre-state: empty password present
    let pre = ri_string.as_str().to_owned();
    assert!(pre.contains("user:@"));

    ri_string.remove_nonempty_password_inline();

    // Post-state: empty password should be preserved (it's not "nonempty")
    let result = ri_string.as_str();
    assert_eq!(result, pre);
    assert!(result.contains("user:@"));

    // Still valid
    let reparse = RiStr::<IriSpec>::new(result);
    assert!(reparse.is_ok());

    assert!(result.starts_with("http://"));
    assert!(result.contains("example.com"));
    assert!(result.contains("/path"));
}

#[test]
fn test_ri_string_remove_nonempty_password_inline_no_userinfo() {
    let iri_str = "https://example.com/secure";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let mut ri_string: RiString<IriSpec> = ri_str.to_owned();

    let pre = ri_string.as_str().to_owned();
    assert!(!pre.contains('@'));

    ri_string.remove_nonempty_password_inline();

    let result = ri_string.as_str();
    assert_eq!(result, pre);
    assert_eq!(result, "https://example.com/secure");
    assert!(!result.contains('@'));

    let reparse = RiStr::<IriSpec>::new(result);
    assert!(reparse.is_ok());

    assert!(result.starts_with("https://"));
    assert!(result.contains("example.com"));
}

#[test]
fn test_ri_string_into_absolute_and_fragment_uri_spec() {
    let uri_str = "http://example.com/page?q=test#top";
    let ri_str = RiStr::<UriSpec>::new(uri_str).expect("valid URI");
    let ri_string: RiString<UriSpec> = ri_str.to_owned();

    let (absolute, fragment_opt) = ri_string.into_absolute_and_fragment();

    assert_eq!(absolute.as_str(), "http://example.com/page?q=test");
    let fragment = fragment_opt.expect("fragment should be present");
    assert_eq!(fragment.as_str(), "top");

    assert!(!absolute.as_str().contains('#'));
    assert!(absolute.as_str().starts_with("http://"));
    assert!(absolute.as_str().contains("example.com"));
    assert!(absolute.as_str().contains("/page"));
    assert!(absolute.as_str().contains("?q=test"));

    let reparse = RiAbsoluteStr::<UriSpec>::new(absolute.as_str());
    assert!(reparse.is_ok());
}

#[test]
fn test_ri_string_combined_workflow() {
    // Complex workflow: create IRI with password and fragment, remove password, then split
    let iri_str = "http://admin:p4ssw0rd@example.com/api/v1?token=abc#response";
    let ri_str = RiStr::<IriSpec>::new(iri_str).expect("valid IRI");
    let mut ri_string: RiString<IriSpec> = ri_str.to_owned();

    // Step 1: Verify initial state
    assert!(ri_string.as_str().contains("p4ssw0rd"));
    assert!(ri_string.as_str().contains("#response"));
    assert!(ri_string.as_str().contains("admin"));

    // Step 2: Remove password
    ri_string.remove_password_inline();
    let after_remove = ri_string.as_str().to_owned();
    assert!(!after_remove.contains("p4ssw0rd"));
    assert!(after_remove.contains("admin"));
    assert!(after_remove.contains("#response"));
    assert!(after_remove.contains("example.com"));

    // Step 3: Split into absolute and fragment
    let (absolute, fragment_opt) = ri_string.into_absolute_and_fragment();
    let fragment = fragment_opt.expect("fragment should exist");

    assert_eq!(fragment.as_str(), "response");
    assert!(!absolute.as_str().contains('#'));
    assert!(!absolute.as_str().contains("p4ssw0rd"));
    assert!(absolute.as_str().contains("admin"));
    assert!(absolute.as_str().contains("/api/v1"));
    assert!(absolute.as_str().contains("?token=abc"));
}

#[test]
fn test_unsafe_new_unchecked_ri_string() {
    // Exercise the unsafe API
    let valid_iri = String::from("http://example.com/safe-path?key=val#frag");

    let ri_string: RiString<IriSpec> = unsafe {
        RiString::new_unchecked(valid_iri)
    };

    assert_eq!(ri_string.as_str(), "http://example.com/safe-path?key=val#frag");

    // Verify it works correctly with into_absolute
    let cloned = ri_string.as_str().to_owned();
    let ri_str = RiStr::<IriSpec>::new(&cloned).expect("should be valid");
    let ri_string2: RiString<IriSpec> = ri_str.to_owned();
    let absolute = ri_string2.into_absolute();
    assert_eq!(absolute.as_str(), "http://example.com/safe-path?key=val");
    assert!(!absolute.as_str().contains('#'));

    // Verify into_absolute_and_fragment on the original
    let (abs, frag) = ri_string.into_absolute_and_fragment();
    assert_eq!(abs.as_str(), "http://example.com/safe-path?key=val");
    let f = frag.expect("fragment present");
    assert_eq!(f.as_str(), "frag");
}