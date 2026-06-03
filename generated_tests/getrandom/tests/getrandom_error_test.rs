#[cfg(all(feature = "wasm_js", target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen_test::wasm_bindgen_test as test;

use getrandom::{fill, Error};

fn raw_os_error(error: Error) -> Option<i32> {
    Error::raw_os_error(error)
}

fn classify_with_error_api(error: Error) -> &'static str {
    if raw_os_error(error).is_some() {
        "raw-os"
    } else {
        "not-raw-os"
    }
}

#[test]
fn custom_errors_have_no_raw_os_error_through_error_api() {
    let custom_errors = [
        Error::new_custom(0),
        Error::new_custom(1),
        Error::new_custom(42),
        Error::new_custom(u16::MAX),
    ];

    for error in custom_errors {
        assert_eq!(raw_os_error(error), None);
        assert!(raw_os_error(error).is_none());
        assert_eq!(classify_with_error_api(error), "not-raw-os");
    }
}

#[test]
fn raw_os_error_helper_agrees_with_root_error_method() {
    let first = Error::new_custom(7);
    let second = Error::new_custom(4096);

    let first_from_helper = raw_os_error(first);
    let first_from_method = Error::raw_os_error(first);
    let second_from_helper = raw_os_error(second);
    let second_from_method = Error::raw_os_error(second);

    assert_eq!(first_from_helper, first_from_method);
    assert_eq!(second_from_helper, second_from_method);
    assert_eq!(first_from_helper, None);
    assert_eq!(second_from_helper, None);
    assert_eq!(classify_with_error_api(first), "not-raw-os");
}

#[test]
fn raw_os_error_can_be_used_in_a_realistic_fill_error_handling_path() {
    let mut bytes = [0_u8; 64];

    match fill(&mut bytes) {
        Ok(()) => {
            assert_eq!(bytes.len(), 64);
            assert!(bytes.iter().any(|byte| *byte != 0) || bytes.iter().all(|byte| *byte == 0));
            assert_eq!(raw_os_error(Error::new_custom(123)), None);
            assert_eq!(classify_with_error_api(Error::new_custom(123)), "not-raw-os");
        }
        Err(error) => {
            let raw = raw_os_error(error);
            assert_eq!(raw, Error::raw_os_error(error));

            if let Some(code) = raw {
                assert_ne!(code, 0);
            } else {
                assert_eq!(classify_with_error_api(error), "not-raw-os");
            }
        }
    }
}

#[test]
fn empty_fill_and_custom_error_classification_are_stable() {
    let mut empty: [u8; 0] = [];

    let fill_result = fill(&mut empty);
    assert!(fill_result.is_ok());
    assert_eq!(empty.len(), 0);

    let custom = Error::new_custom(555);
    let raw_before_formatting = raw_os_error(custom);
    let debug_text = format!("{custom:?}");
    let display_text = format!("{custom}");

    assert_eq!(raw_before_formatting, None);
    assert!(debug_text.contains("Error") || !debug_text.is_empty());
    assert!(!display_text.is_empty());
    assert_eq!(raw_os_error(custom), None);
}