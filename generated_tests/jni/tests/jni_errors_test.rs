use jni::errors::jni_error_code_to_result;
use jni::sys::jint;

#[test]
fn jni_ok_code_is_success_and_allows_follow_up_workflow() {
    let ok_code: jint = 0;

    let first_check = jni::errors::jni_error_code_to_result(ok_code);
    assert!(first_check.is_ok(), "JNI_OK must translate to Ok(())");

    let mut completed_steps = Vec::new();

    if jni_error_code_to_result(ok_code).is_ok() {
        completed_steps.push("validated status");
    }

    if jni_error_code_to_result(ok_code).is_ok() {
        completed_steps.push("continued workflow");
    }

    assert_eq!(completed_steps.len(), 2);
    assert_eq!(completed_steps[0], "validated status");
    assert_eq!(completed_steps[1], "continued workflow");
}

#[test]
fn standard_jni_error_codes_are_rejected() {
    let standard_error_codes: &[(jint, &str)] = &[
        (-1, "JNI_ERR"),
        (-2, "JNI_EDETACHED"),
        (-3, "JNI_EVERSION"),
        (-4, "JNI_ENOMEM"),
        (-5, "JNI_EEXIST"),
        (-6, "JNI_EINVAL"),
    ];

    let mut rejected_names = Vec::new();

    for &(code, name) in standard_error_codes {
        let result = jni::errors::jni_error_code_to_result(code);
        assert!(
            result.is_err(),
            "{name} ({code}) must translate to an error result"
        );

        let debug_message = format!("{:?}", result.unwrap_err());
        assert!(
            !debug_message.is_empty(),
            "{name} ({code}) should produce a debuggable error"
        );

        rejected_names.push(name);
    }

    assert_eq!(rejected_names.len(), standard_error_codes.len());
    assert!(rejected_names.contains(&"JNI_ERR"));
    assert!(rejected_names.contains(&"JNI_EINVAL"));
}

#[test]
fn mixed_status_codes_can_be_partitioned_into_successes_and_failures() {
    let observed_codes: [jint; 7] = [0, -1, -2, 0, -4, -6, 0];

    let mut successes = Vec::new();
    let mut failures = Vec::new();

    for code in observed_codes {
        match jni::errors::jni_error_code_to_result(code) {
            Ok(()) => successes.push(code),
            Err(err) => failures.push((code, format!("{err:?}"))),
        }
    }

    assert_eq!(successes, vec![0, 0, 0]);
    assert_eq!(failures.len(), 4);
    assert_eq!(failures.iter().map(|(code, _)| *code).collect::<Vec<_>>(), vec![-1, -2, -4, -6]);
    assert!(failures.iter().all(|(_, message)| !message.is_empty()));
}

#[test]
fn unknown_non_zero_codes_are_treated_as_errors() {
    let unusual_codes: [jint; 4] = [1, 42, -7, jint::MIN];

    for code in unusual_codes {
        let result = jni::errors::jni_error_code_to_result(code);
        assert!(
            result.is_err(),
            "non-zero JNI status code {code} should not be accepted as success"
        );
    }

    assert!(jni::errors::jni_error_code_to_result(0).is_ok());
}