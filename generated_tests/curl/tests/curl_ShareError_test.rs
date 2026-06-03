use std::mem;

unsafe fn share_error_from_code(code: i32) -> curl::ShareError {
    unsafe { mem::transmute::<i32, curl::ShareError>(code) }
}

fn share_error_code_as_i32(error: &curl::ShareError) -> i32 {
    unsafe { mem::transmute_copy(&curl::ShareError::code(error)) }
}

fn collect_share_error_flags(error: &curl::ShareError) -> Vec<(&'static str, bool)> {
    vec![
        ("is_bad_option", curl::ShareError::is_bad_option(error)),
        ("is_in_use", curl::ShareError::is_in_use(error)),
        ("is_invalid", curl::ShareError::is_invalid(error)),
        ("is_nomem", curl::ShareError::is_nomem(error)),
    ]
}

fn assert_only_share_error_flag(error: &curl::ShareError, expected_name: &'static str) {
    let enabled: Vec<&'static str> = collect_share_error_flags(error)
        .into_iter()
        .filter_map(|(name, enabled)| if enabled { Some(name) } else { None })
        .collect();

    assert_eq!(enabled, vec![expected_name]);
}

fn assert_no_share_error_flags(error: &curl::ShareError) {
    for (name, enabled) in collect_share_error_flags(error) {
        assert!(
            !enabled,
            "unexpected ShareError flag {name} was enabled for code {}",
            share_error_code_as_i32(error)
        );
    }
}

#[test]
fn known_share_error_codes_report_exclusive_flags_codes_and_descriptions() {
    curl::init();

    let cases = [
        (1, "is_bad_option", "option"),
        (2, "is_in_use", "use"),
        (3, "is_invalid", "invalid"),
        (4, "is_nomem", "memory"),
    ];

    for (raw_code, expected_flag, description_hint) in cases {
        let error = unsafe { share_error_from_code(raw_code) };

        assert_eq!(share_error_code_as_i32(&error), raw_code);
        assert_only_share_error_flag(&error, expected_flag);

        let description = curl::ShareError::description(&error);
        assert!(
            !description.trim().is_empty(),
            "description for ShareError code {raw_code} should not be empty"
        );
        assert!(
            description
                .to_ascii_lowercase()
                .contains(&description_hint.to_ascii_lowercase()),
            "description {description:?} for ShareError code {raw_code} should mention {description_hint:?}"
        );
    }
}

#[test]
fn ok_and_unknown_share_error_codes_have_no_specific_error_flags() {
    curl::init();

    let ok = unsafe { share_error_from_code(0) };
    assert_eq!(share_error_code_as_i32(&ok), 0);
    assert_no_share_error_flags(&ok);
    assert!(
        !curl::ShareError::description(&ok).trim().is_empty(),
        "CURLSHE_OK should still have a human-readable description"
    );

    let unknown = unsafe { share_error_from_code(999) };
    assert_eq!(share_error_code_as_i32(&unknown), 999);
    assert_no_share_error_flags(&unknown);

    let unknown_description = curl::ShareError::description(&unknown);
    assert!(
        !unknown_description.trim().is_empty(),
        "unknown ShareError codes should still produce a fallback description"
    );
    assert!(
        unknown_description.to_ascii_lowercase().contains("unknown")
            || unknown_description.to_ascii_lowercase().contains("error"),
        "fallback description should be meaningful, got {unknown_description:?}"
    );
}

#[test]
fn share_error_predicates_are_stable_across_repeated_metadata_queries() {
    curl::init();

    let error = unsafe { share_error_from_code(2) };

    let first_code = share_error_code_as_i32(&error);
    let first_description = curl::ShareError::description(&error).to_owned();
    let first_flags = collect_share_error_flags(&error);

    let second_description = curl::ShareError::description(&error).to_owned();
    let second_flags = collect_share_error_flags(&error);
    let second_code = share_error_code_as_i32(&error);

    assert_eq!(first_code, 2);
    assert_eq!(first_code, second_code);
    assert_eq!(first_description, second_description);
    assert_eq!(first_flags, second_flags);
    assert!(curl::ShareError::is_in_use(&error));
    assert!(!curl::ShareError::is_bad_option(&error));
    assert!(!curl::ShareError::is_invalid(&error));
    assert!(!curl::ShareError::is_nomem(&error));
}