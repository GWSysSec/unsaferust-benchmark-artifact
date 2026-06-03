#[cfg(all(feature = "wasm_js", target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen_test::wasm_bindgen_test as test;

use getrandom::{fill, Error};

fn classify_error(error: Error) -> String {
    match Error::raw_os_error(error) {
        Some(code) => format!("os:{code}"),
        None => "non-os".to_owned(),
    }
}

#[test]
fn custom_error_codes_are_not_reported_as_raw_os_errors() {
    const ZERO_CODE: Error = Error::new_custom(0);
    const SMALL_CODE: Error = Error::new_custom(7);
    const MAX_CODE: Error = Error::new_custom(u16::MAX);

    let zero_raw: Option<i32> = Error::raw_os_error(ZERO_CODE);
    let small_raw: Option<i32> = Error::raw_os_error(SMALL_CODE);
    let max_raw: Option<i32> = Error::raw_os_error(MAX_CODE);

    assert_eq!(zero_raw, None);
    assert_eq!(small_raw, None);
    assert_eq!(max_raw, None);

    assert_eq!(classify_error(ZERO_CODE), "non-os");
    assert_eq!(classify_error(SMALL_CODE), "non-os");
    assert_eq!(classify_error(MAX_CODE), "non-os");
}

#[test]
fn custom_errors_remain_distinguishable_when_formatted() {
    let first = Error::new_custom(1);
    let second = Error::new_custom(2);
    let first_again = Error::new_custom(1);

    let first_debug = format!("{first:?}");
    let second_debug = format!("{second:?}");
    let first_again_debug = format!("{first_again:?}");

    assert!(!first_debug.is_empty());
    assert!(!second_debug.is_empty());
    assert_eq!(first_debug, first_again_debug);
    assert_ne!(first_debug, second_debug);

    let display = format!("{}", Error::new_custom(42));
    assert!(!display.trim().is_empty());
    assert_eq!(Error::raw_os_error(Error::new_custom(42)), None);
}

#[test]
fn custom_error_handling_can_be_chained_with_random_fill_workflow() {
    let mut session_key = [0xA5_u8; 32];
    let before_fill = session_key;

    fill(&mut session_key).expect("system random source should fill a session key");

    assert_eq!(session_key.len(), 32);
    assert_ne!(session_key, before_fill);

    let validation_error = if session_key.iter().any(|&byte| byte != 0) {
        None
    } else {
        Some(Error::new_custom(100))
    };

    assert!(validation_error.is_none());

    let synthetic_error = Error::new_custom(100);
    assert_eq!(Error::raw_os_error(synthetic_error), None);
    assert_eq!(classify_error(synthetic_error), "non-os");
}