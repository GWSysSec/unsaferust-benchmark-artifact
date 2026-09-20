use std::mem;

unsafe fn multi_error_from_code(code: i32) -> curl::MultiError {
    unsafe { mem::transmute::<i32, curl::MultiError>(code) }
}

fn collect_multi_error_flags(error: &curl::MultiError) -> Vec<(&'static str, bool)> {
    vec![
        ("is_bad_handle", curl::MultiError::is_bad_handle(error)),
        (
            "is_bad_easy_handle",
            curl::MultiError::is_bad_easy_handle(error),
        ),
        (
            "is_out_of_memory",
            curl::MultiError::is_out_of_memory(error),
        ),
        (
            "is_internal_error",
            curl::MultiError::is_internal_error(error),
        ),
        ("is_bad_socket", curl::MultiError::is_bad_socket(error)),
        (
            "is_unknown_option",
            curl::MultiError::is_unknown_option(error),
        ),
        ("is_call_perform", curl::MultiError::is_call_perform(error)),
    ]
}

fn assert_only_flag(error: &curl::MultiError, expected_name: &'static str) {
    let flags = collect_multi_error_flags(error);
    let enabled: Vec<&'static str> = flags
        .iter()
        .filter_map(|(name, enabled)| if *enabled { Some(*name) } else { None })
        .collect();

    assert_eq!(
        enabled,
        vec![expected_name],
        "expected only {expected_name} for multi error description {:?}",
        curl::MultiError::description(error)
    );
}

#[test]
fn multi_error_specific_predicates_match_their_underlying_codes() {
    curl::init();

    let bad_handle = unsafe { multi_error_from_code(1) };
    assert!(curl::MultiError::is_bad_handle(&bad_handle));
    assert_only_flag(&bad_handle, "is_bad_handle");
    assert_eq!(curl::MultiError::code(&bad_handle) as i32, 1);
    assert!(
        curl::MultiError::description(&bad_handle)
            .to_ascii_lowercase()
            .contains("multi"),
        "unexpected CURLM_BAD_HANDLE description: {:?}",
        curl::MultiError::description(&bad_handle)
    );

    let bad_easy_handle = unsafe { multi_error_from_code(2) };
    assert!(curl::MultiError::is_bad_easy_handle(&bad_easy_handle));
    assert_only_flag(&bad_easy_handle, "is_bad_easy_handle");
    assert_eq!(curl::MultiError::code(&bad_easy_handle) as i32, 2);
    assert!(
        curl::MultiError::description(&bad_easy_handle)
            .to_ascii_lowercase()
            .contains("easy"),
        "unexpected CURLM_BAD_EASY_HANDLE description: {:?}",
        curl::MultiError::description(&bad_easy_handle)
    );

    let out_of_memory = unsafe { multi_error_from_code(3) };
    assert!(curl::MultiError::is_out_of_memory(&out_of_memory));
    assert_only_flag(&out_of_memory, "is_out_of_memory");
    assert_eq!(curl::MultiError::code(&out_of_memory) as i32, 3);
    assert!(
        curl::MultiError::description(&out_of_memory)
            .to_ascii_lowercase()
            .contains("memory"),
        "unexpected CURLM_OUT_OF_MEMORY description: {:?}",
        curl::MultiError::description(&out_of_memory)
    );

    let internal_error = unsafe { multi_error_from_code(4) };
    assert!(curl::MultiError::is_internal_error(&internal_error));
    assert_only_flag(&internal_error, "is_internal_error");
    assert_eq!(curl::MultiError::code(&internal_error) as i32, 4);
    assert!(
        !curl::MultiError::description(&internal_error).trim().is_empty(),
        "CURLM_INTERNAL_ERROR should have a non-empty description"
    );
}

#[test]
fn multi_error_socket_unknown_option_and_call_perform_are_distinct() {
    curl::init();

    let bad_socket = unsafe { multi_error_from_code(5) };
    assert!(curl::MultiError::is_bad_socket(&bad_socket));
    assert_only_flag(&bad_socket, "is_bad_socket");
    assert_eq!(curl::MultiError::code(&bad_socket) as i32, 5);
    assert!(
        curl::MultiError::description(&bad_socket)
            .to_ascii_lowercase()
            .contains("socket"),
        "unexpected CURLM_BAD_SOCKET description: {:?}",
        curl::MultiError::description(&bad_socket)
    );

    let unknown_option = unsafe { multi_error_from_code(6) };
    assert!(curl::MultiError::is_unknown_option(&unknown_option));
    assert_only_flag(&unknown_option, "is_unknown_option");
    assert_eq!(curl::MultiError::code(&unknown_option) as i32, 6);
    assert!(
        curl::MultiError::description(&unknown_option)
            .to_ascii_lowercase()
            .contains("option"),
        "unexpected CURLM_UNKNOWN_OPTION description: {:?}",
        curl::MultiError::description(&unknown_option)
    );

    let call_perform = unsafe { multi_error_from_code(-1) };
    assert!(curl::MultiError::is_call_perform(&call_perform));
    assert_only_flag(&call_perform, "is_call_perform");
    assert_eq!(curl::MultiError::code(&call_perform) as i32, -1);
    assert!(
        !curl::MultiError::description(&call_perform).trim().is_empty(),
        "CURLM_CALL_MULTI_PERFORM should have a non-empty description"
    );

    let descriptions = [
        curl::MultiError::description(&bad_socket),
        curl::MultiError::description(&unknown_option),
        curl::MultiError::description(&call_perform),
    ];
    assert!(
        descriptions.iter().all(|description| !description.is_empty()),
        "all inspected multi errors should expose human-readable descriptions"
    );
}