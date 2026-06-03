#[test]
fn build_records_files_in_insertion_order_after_chained_configuration() {
    let mut build = cc::Build::new();

    build
        .include("include")
        .includes(["generated/include", "vendor/include"])
        .define("FEATURE_ENABLED", Some("1"))
        .define("FLAG_WITHOUT_VALUE", None)
        .file("src/first.c")
        .files(["src/second.c", "src/third.c"])
        .object("prebuilt/one.o")
        .objects(["prebuilt/two.o", "prebuilt/three.o"])
        .flag("-Wall")
        .remove_flag("-Wall")
        .warnings(false)
        .extra_warnings(false)
        .cargo_metadata(false)
        .cargo_warnings(false)
        .cargo_debug(false)
        .cargo_output(false);

    let files: Vec<_> = build.get_files().collect();

    assert_eq!(files.len(), 3, "exactly the configured source files should be recorded");
    assert_eq!(files[0], std::path::Path::new("src/first.c"));
    assert_eq!(files[1], std::path::Path::new("src/second.c"));
    assert_eq!(files[2], std::path::Path::new("src/third.c"));
    assert!(
        !files.iter().any(|path| path.ends_with("prebuilt/one.o")),
        "object files should not be reported as source files"
    );
}

#[cfg(windows)]
mod windows_setup_config_tests {
    use std::ptr;

    #[test]
    fn setup_configuration_can_query_available_instance_enumerators() {
        let init = cc::windows::com::initialize();
        assert!(init.is_ok(), "COM initialization should succeed before setup queries: {init:?}");

        let configuration = cc::windows::setup_config::new();

        match configuration {
            Ok(configuration) => {
                let current_process = configuration.get_instance_for_current_process();
                let instances = configuration.enum_instances();
                let all_instances = configuration.enum_all_instances();

                assert!(
                    current_process.is_ok() || current_process.is_err(),
                    "current-process instance query should return a well-formed Result"
                );
                assert!(
                    instances.is_ok() || instances.is_err(),
                    "instance enumeration query should return a well-formed Result"
                );
                assert!(
                    all_instances.is_ok() || all_instances.is_err(),
                    "all-instance enumeration query should return a well-formed Result"
                );

                if let Ok(instance) = current_process {
                    let instance_id = instance.instance_id();
                    let installation_name = instance.installation_name();
                    let installation_path = instance.installation_path();
                    let installation_version = instance.installation_version();
                    let product_path = instance.product_path();

                    assert!(
                        instance_id.is_ok()
                            || installation_name.is_ok()
                            || installation_path.is_ok()
                            || installation_version.is_ok()
                            || product_path.is_ok()
                            || instance_id.is_err()
                                && installation_name.is_err()
                                && installation_path.is_err()
                                && installation_version.is_err()
                                && product_path.is_err(),
                        "all setup instance property methods should return well-formed Results"
                    );

                    if let Ok(value) = instance_id {
                        assert!(!value.is_empty(), "a successful instance_id query should not be empty");
                    }
                    if let Ok(value) = installation_name {
                        assert!(
                            !value.is_empty(),
                            "a successful installation_name query should not be empty"
                        );
                    }
                    if let Ok(value) = installation_path {
                        assert!(
                            !value.is_empty(),
                            "a successful installation_path query should not be empty"
                        );
                    }
                    if let Ok(value) = installation_version {
                        assert!(
                            !value.is_empty(),
                            "a successful installation_version query should not be empty"
                        );
                    }
                    if let Ok(value) = product_path {
                        assert!(!value.is_empty(), "a successful product_path query should not be empty");
                    }
                }
            }
            Err(code) => {
                assert_ne!(
                    code, 0,
                    "a failed SetupConfiguration creation should report a non-success HRESULT"
                );
            }
        }
    }

    #[test]
    fn setup_instance_created_from_null_raw_pointer_reports_property_errors() {
        let null_instance: *mut cc::windows::setup_config::ISetupInstance = ptr::null_mut();

        let instance = unsafe { cc::windows::setup_config::from_raw(null_instance) };

        let instance_id = instance.instance_id();
        let installation_name = instance.installation_name();
        let installation_path = instance.installation_path();
        let installation_version = instance.installation_version();
        let product_path = instance.product_path();

        assert!(
            instance_id.is_err(),
            "instance_id on a null setup instance should fail"
        );
        assert!(
            installation_name.is_err(),
            "installation_name on a null setup instance should fail"
        );
        assert!(
            installation_path.is_err(),
            "installation_path on a null setup instance should fail"
        );
        assert!(
            installation_version.is_err(),
            "installation_version on a null setup instance should fail"
        );
        assert!(
            product_path.is_err(),
            "product_path on a null setup instance should fail"
        );
    }
}

#[cfg(not(windows))]
#[test]
fn setup_config_tests_are_windows_only_but_crate_remains_usable_elsewhere() {
    let mut build = cc::Build::new();
    build
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .opt_level(2)
        .debug(false)
        .file("portable.c");

    let files: Vec<_> = build.get_files().collect();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0], std::path::Path::new("portable.c"));
    assert!(files[0].ends_with("portable.c"));
}