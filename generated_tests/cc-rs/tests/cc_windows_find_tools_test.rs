use std::path::{Path, PathBuf};

use cc::windows_registry::{find, find_tool, find_vs_version};

#[test]
fn missing_windows_registry_tool_is_reported_consistently() {
    let missing_tool = "__cc_rs_integration_test_tool_that_should_not_exist_9F4C2A71.exe";

    let by_target = find("x86_64-pc-windows-msvc", missing_tool);
    let tool_by_target = find_tool("x86_64-pc-windows-msvc", missing_tool);
    let by_arch = find("x86_64", missing_tool);
    let tool_by_arch = find_tool("x86_64", missing_tool);

    assert!(by_target.is_none());
    assert!(tool_by_target.is_none());
    assert!(by_arch.is_none());
    assert!(tool_by_arch.is_none());
}

#[test]
fn windows_registry_find_and_find_tool_agree_for_cl_when_available() {
    let command = find("x86_64-pc-windows-msvc", "cl.exe");
    let tool = find_tool("x86_64-pc-windows-msvc", "cl.exe");

    assert_eq!(command.is_some(), tool.is_some());

    if let Some(tool) = tool {
        assert!(!tool.path().as_os_str().is_empty());

        let command_from_tool = tool.to_command();
        assert_eq!(command_from_tool.get_program(), tool.path().as_os_str());

        assert!(!tool.cc_env().is_empty());
        assert!(!tool.cflags_env().is_empty());

        assert!(
            tool.is_like_msvc()
                || tool.is_like_clang_cl()
                || tool.is_like_clang()
                || tool.is_like_gnu()
        );
    }
}

#[test]
fn find_vs_version_returns_useful_success_or_error() {
    match find_vs_version() {
        Ok(_) => {
            let cl = find_tool("x86_64-pc-windows-msvc", "cl.exe")
                .or_else(|| find_tool("x86", "cl.exe"))
                .or_else(|| find_tool("aarch64-pc-windows-msvc", "cl.exe"));

            assert!(
                cl.is_some() || !cfg!(windows),
                "finding a Visual Studio version on Windows should normally make cl.exe discoverable"
            );
        }
        Err(message) => {
            assert!(!message.trim().is_empty());
        }
    }
}

#[test]
fn build_records_files_from_chained_configuration_without_compiling() {
    let root = PathBuf::from("tests").join("fixtures").join("cc_registry_workflow");
    let first = root.join("alpha.c");
    let second = root.join("nested").join("beta.c");
    let third = root.join("gamma.S");

    let mut build = cc::Build::new();
    build
        .include(root.join("include"))
        .includes([root.join("generated"), root.join("vendor").join("include")])
        .define("CC_RS_INTEGRATION_TEST", Some("1"))
        .define("CC_RS_FEATURE_FLAG", None)
        .flag("-DKEPT_FLAG")
        .flag("-DREMOVED_FLAG")
        .remove_flag("-DREMOVED_FLAG")
        .warnings(false)
        .extra_warnings(false)
        .warnings_into_errors(false)
        .debug(false)
        .opt_level(2)
        .pic(true)
        .cargo_metadata(false)
        .cargo_warnings(false)
        .cargo_debug(false)
        .cargo_output(false)
        .emit_rerun_if_env_changed(false)
        .file(&first)
        .files([&second, &third]);

    let files: Vec<&Path> = build.get_files().collect();

    assert_eq!(files.len(), 3);
    assert!(files.iter().any(|path| *path == first.as_path()));
    assert!(files.iter().any(|path| *path == second.as_path()));
    assert!(files.iter().any(|path| *path == third.as_path()));
}

#[test]
fn build_accepts_object_and_tool_configuration_as_downstream_user() {
    let out_dir = std::env::temp_dir().join("cc_rs_integration_build_configuration");
    let object_one = out_dir.join("one.o");
    let object_two = out_dir.join("two.o");

    let mut build = cc::Build::new();
    build
        .out_dir(&out_dir)
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .compiler("cc")
        .archiver("ar")
        .ranlib("ranlib")
        .object(&object_one)
        .objects([&object_two])
        .static_flag(true)
        .shared_flag(false)
        .no_default_flags(true)
        .inherit_rustflags(false)
        .shell_escaped_flags(false)
        .force_frame_pointer(false)
        .use_plt(true)
        .static_crt(false);

    let files: Vec<&Path> = build.get_files().collect();

    assert!(files.is_empty());
    assert!(object_one.ends_with("one.o"));
    assert!(object_two.ends_with("two.o"));
    assert!(out_dir.ends_with("cc_rs_integration_build_configuration"));
}