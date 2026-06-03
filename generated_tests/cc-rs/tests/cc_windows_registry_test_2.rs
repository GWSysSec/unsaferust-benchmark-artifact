#[cfg(windows)]
mod windows_registry_tests {
    use std::ffi::OsStr;

    use cc::windows::registry::LOCAL_MACHINE;

    #[test]
    fn opens_well_known_keys_iterates_subkeys_and_queries_values() {
        let software = LOCAL_MACHINE
            .open(OsStr::new("SOFTWARE"))
            .expect("HKLM\\SOFTWARE should be readable");

        let subkeys: Vec<_> = software
            .iter()
            .map(|entry| entry.expect("iterating HKLM\\SOFTWARE subkeys should succeed"))
            .collect();

        assert!(
            !subkeys.is_empty(),
            "HKLM\\SOFTWARE should contain at least one subkey"
        );
        assert!(
            subkeys.iter().all(|name| !name.is_empty()),
            "registry subkey names yielded by iter should not be empty"
        );
        assert!(
            subkeys
                .iter()
                .any(|name| name.to_string_lossy().eq_ignore_ascii_case("Microsoft")),
            "HKLM\\SOFTWARE should contain a Microsoft subkey"
        );

        let current_version = LOCAL_MACHINE
            .open(OsStr::new("SOFTWARE\\Microsoft\\Windows\\CurrentVersion"))
            .expect("Windows CurrentVersion registry key should be readable");

        let program_files = current_version
            .query_str("ProgramFilesDir")
            .expect("ProgramFilesDir should be present in CurrentVersion");

        assert!(
            !program_files.is_empty(),
            "ProgramFilesDir registry value should not be empty"
        );
        assert!(
            program_files.to_string_lossy().contains(':')
                || program_files.to_string_lossy().starts_with(r"\\"),
            "ProgramFilesDir should look like an absolute Windows path: {:?}",
            program_files
        );

        let missing_value = current_version.query_str(
            "__cc_rs_integration_test_value_that_should_not_exist_9F4C2A71",
        );
        assert!(
            missing_value.is_err(),
            "querying a deliberately missing registry value should fail"
        );
    }

    #[test]
    fn opening_missing_subkey_returns_error_without_breaking_parent_iteration() {
        let current_version = LOCAL_MACHINE
            .open(OsStr::new("SOFTWARE\\Microsoft\\Windows\\CurrentVersion"))
            .expect("Windows CurrentVersion registry key should be readable");

        let missing = current_version.open(OsStr::new(
            "__cc_rs_integration_test_subkey_that_should_not_exist_9F4C2A71",
        ));
        assert!(
            missing.is_err(),
            "opening a deliberately missing registry subkey should fail"
        );

        let child_names: Vec<_> = current_version
            .iter()
            .map(|entry| entry.expect("iterating CurrentVersion subkeys should succeed"))
            .collect();

        assert!(
            child_names
                .iter()
                .all(|name| name != OsStr::new("__cc_rs_integration_test_subkey_that_should_not_exist_9F4C2A71")),
            "the deliberately missing subkey should not appear during iteration"
        );
    }
}

#[cfg(not(windows))]
#[test]
fn windows_registry_api_tests_are_windows_only() {
    assert!(
        !cfg!(windows),
        "registry integration tests that call cc::windows::registry run only on Windows"
    );
}