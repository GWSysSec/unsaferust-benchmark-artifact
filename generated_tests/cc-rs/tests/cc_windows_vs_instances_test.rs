use std::path::Path;

#[test]
fn build_file_tracking_remains_stable_through_realistic_configuration() {
    let mut build = cc::Build::new();

    build
        .include("include")
        .includes(["generated/include", "vendor/include"])
        .define("FEATURE_ENABLED", Some("1"))
        .define("HEADER_ONLY_MARKER", None)
        .file("src/alpha.c")
        .files(["src/beta.c", "src/gamma.c"])
        .object("prebuilt/alpha.o")
        .objects(["prebuilt/beta.o", "prebuilt/gamma.o"])
        .flag("-Wall")
        .flag("-Wextra")
        .remove_flag("-Wextra")
        .warnings(false)
        .extra_warnings(false)
        .warnings_into_errors(false)
        .cargo_metadata(false)
        .cargo_warnings(false)
        .cargo_debug(false)
        .cargo_output(false)
        .emit_rerun_if_env_changed(false)
        .inherit_rustflags(false);

    let files: Vec<_> = build.get_files().collect();

    assert_eq!(
        files.len(),
        3,
        "only source files added with file/files should be reported"
    );
    assert_eq!(files[0], Path::new("src/alpha.c"));
    assert_eq!(files[1], Path::new("src/beta.c"));
    assert_eq!(files[2], Path::new("src/gamma.c"));
    assert!(
        files.iter().all(|path| path.extension().and_then(|ext| ext.to_str()) == Some("c")),
        "reported build inputs should be the configured C source files"
    );
    assert!(
        !files.iter().any(|path| path.starts_with("prebuilt")),
        "prebuilt object files are linker inputs, not source files"
    );
}

#[test]
fn chained_cpp_configuration_preserves_declared_source_order() {
    let mut build = cc::Build::new();

    build
        .cpp(true)
        .std("c++17")
        .cpp_link_stdlib(None)
        .cpp_set_stdlib(None)
        .pic(true)
        .shared_flag(false)
        .static_flag(false)
        .debug(false)
        .opt_level(2)
        .force_frame_pointer(false)
        .file("src/main.cc")
        .files(["src/detail.cc", "src/platform.cc"])
        .cargo_metadata(false)
        .cargo_output(false);

    let files: Vec<_> = build.get_files().map(Path::to_path_buf).collect();

    assert_eq!(
        files,
        vec![
            Path::new("src/main.cc").to_path_buf(),
            Path::new("src/detail.cc").to_path_buf(),
            Path::new("src/platform.cc").to_path_buf(),
        ],
        "C++ source files should be yielded in insertion order"
    );
    assert!(
        files.iter().all(|path| path.extension().and_then(|ext| ext.to_str()) == Some("cc")),
        "all configured C++ source files should retain their extensions"
    );
}

#[cfg(windows)]
#[test]
fn visual_studio_instance_accessors_are_public_downstream_methods() {
    use std::borrow::Cow;
    use std::path::PathBuf;

    type Instance = cc::windows::vs_instances::VsInstance;

    let installation_name:
        for<'a> fn(&'a Instance) -> Option<Cow<'a, str>> =
        cc::windows::vs_instances::VsInstance::installation_name;
    let installation_path:
        for<'a> fn(&'a Instance) -> Option<PathBuf> =
        cc::windows::vs_instances::VsInstance::installation_path;
    let installation_version:
        for<'a> fn(&'a Instance) -> Option<Cow<'a, str>> =
        cc::windows::vs_instances::VsInstance::installation_version;

    assert_ne!(
        installation_name as usize, 0,
        "installation_name should be callable as a public downstream method"
    );
    assert_ne!(
        installation_path as usize, 0,
        "installation_path should be callable as a public downstream method"
    );
    assert_ne!(
        installation_version as usize, 0,
        "installation_version should be callable as a public downstream method"
    );
}