use jni::sys::jint;

#[test]
fn successful_status_codes_can_drive_a_multi_step_state_machine() {
    let status_codes: [jint; 3] = [0, 0, 0];
    let mut state = String::from("created");
    let mut successful_transitions = 0;

    for code in status_codes.iter().copied() {
        let result = jni::errors::jni_error_code_to_result(code);
        assert!(result.is_ok(), "JNI_OK should be accepted at every workflow step");

        successful_transitions += 1;
        state.push_str(" -> ok");
    }

    assert_eq!(successful_transitions, 3);
    assert_eq!(state, "created -> ok -> ok -> ok");
    assert!(jni::errors::jni_error_code_to_result(0 as jint).is_ok());
}

#[test]
fn standard_error_codes_stop_workflow_and_preserve_order_of_failures() {
    let attempted_steps: [(jint, &str); 7] = [
        (0, "initialize"),
        (-1, "generic failure"),
        (-2, "detached thread"),
        (-3, "unsupported version"),
        (-4, "out of memory"),
        (-5, "vm already exists"),
        (-6, "invalid arguments"),
    ];

    let mut completed_steps = Vec::new();
    let mut failed_steps = Vec::new();

    for (code, label) in attempted_steps.iter().copied() {
        match jni::errors::jni_error_code_to_result(code) {
            Ok(()) => completed_steps.push(label),
            Err(_) => failed_steps.push(label),
        }
    }

    assert_eq!(completed_steps, vec!["initialize"]);
    assert_eq!(failed_steps.len(), 6);
    assert_eq!(failed_steps[0], "generic failure");
    assert_eq!(failed_steps[5], "invalid arguments");
    assert!(failed_steps.contains(&"out of memory"));
}

#[test]
fn non_standard_error_codes_are_reported_as_errors_not_successes() {
    let unusual_error_codes: [jint; 5] = [1, 7, -7, jint::MAX, jint::MIN];

    let mut rejected = 0;
    for code in unusual_error_codes.iter().copied() {
        let result = jni::errors::jni_error_code_to_result(code);
        assert!(
            result.is_err(),
            "unexpected JNI status code {code} must not be treated as success"
        );
        rejected += 1;
    }

    assert_eq!(rejected, unusual_error_codes.len());
    assert!(jni::errors::jni_error_code_to_result(jint::MAX).is_err());
    assert!(jni::errors::jni_error_code_to_result(jint::MIN).is_err());
}

#[test]
fn mixed_status_stream_collects_successes_and_errors_separately() {
    let status_stream: [jint; 8] = [0, -1, 0, -4, 0, -6, 0, 42];

    let mut successes = Vec::new();
    let mut failures = Vec::new();

    for (index, code) in status_stream.iter().copied().enumerate() {
        if jni::errors::jni_error_code_to_result(code).is_ok() {
            successes.push(index);
        } else {
            failures.push((index, code));
        }
    }

    assert_eq!(successes, vec![0, 2, 4, 6]);
    assert_eq!(failures, vec![(1, -1), (3, -4), (5, -6), (7, 42)]);
    assert_eq!(successes.len() + failures.len(), 8);
    assert!(failures.iter().all(|(_, code)| *code != 0));
}

#[test]
fn repeated_checks_are_deterministic_for_same_status_code() {
    let ok_first = jni::errors::jni_error_code_to_result(0 as jint).is_ok();
    let ok_second = jni::errors::jni_error_code_to_result(0 as jint).is_ok();
    let err_first = jni::errors::jni_error_code_to_result(-1 as jint).is_err();
    let err_second = jni::errors::jni_error_code_to_result(-1 as jint).is_err();

    assert_eq!(ok_first, ok_second);
    assert_eq!(err_first, err_second);
    assert!(ok_first);
    assert!(err_first);
    assert_ne!(ok_first, !ok_second);
}