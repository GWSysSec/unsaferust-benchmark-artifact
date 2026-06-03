use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_out_dir(test_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "cc_tool_env_integration_{test_name}_{}_{}",
        std::process::id(),
        nonce
    ))
}

fn os_string_contains(haystack: &OsStr, needle: &str) -> bool {
    haystack.to_string_lossy().contains(needle)
}

#[test]
fn clang_cl_tool_reports_driver_kind_and_environment_strings() {
    let out_dir = unique_out_dir("clang_cl");
    std::fs::create_dir_all(&out_dir).expect("temporary output directory should be creatable");

    let mut build = cc::Build::new();
    build
        .target("x86_64-pc-windows-msvc")
        .host("x86_64-pc-windows-msvc")
        .out_dir(&out_dir)
        .compiler("clang-cl")
        .cpp(true)
        .warnings(false)
        .cargo_metadata(false)
        .cargo_warnings(false)
        .cargo_output(false)
        .no_default_flags(true)
        .debug(false)
        .opt_level(0)
        .flag("/nologo")
        .flag("/O2");

    let tool = build
        .try_get_compiler()
        .expect("an explicitly configured compiler should produce a Tool");

    assert_eq!(tool.path(), std::path::Path::new("clang-cl"));
    assert!(tool.is_like_clang_cl());
    assert!(tool.is_like_msvc());
    assert!(!tool.is_like_gnu());




    assert!(!tool.is_like_clang());

    let cc_env = tool.cc_env();
    let cflags_env = tool.cflags_env();




    assert!(
        cc_env.is_empty(),
        "directly configured compiler should not add CC env fragments: {:?}",
        cc_env
    );

    assert!(os_string_contains(&cflags_env, "/nologo"));
    assert!(os_string_contains(&cflags_env, "/O2"));

    let args_as_strings: Vec<_> = tool
        .args()
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    assert!(args_as_strings.iter().any(|arg| arg == "/nologo"));
    assert!(args_as_strings.iter().any(|arg| arg == "/O2"));

    let command = tool.to_command();
    assert_eq!(command.get_program(), tool.path().as_os_str());
}

#[test]
fn gnu_like_tool_environment_strings_track_configured_flags() {
    let out_dir = unique_out_dir("gnu_like");
    std::fs::create_dir_all(&out_dir).expect("temporary output directory should be creatable");

    let mut build = cc::Build::new();
    build
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .out_dir(&out_dir)
        .compiler("cc")
        .cargo_metadata(false)
        .cargo_warnings(false)
        .cargo_output(false)
        .no_default_flags(true)
        .debug(false)
        .opt_level(0)
        .warnings(false)
        .flag("-DVALUE_FROM_INTEGRATION_TEST=123")
        .flag("-fPIC")
        .remove_flag("-fPIC")
        .flag("-fno-omit-frame-pointer");

    let tool = build
        .try_get_compiler()
        .expect("an explicitly configured compiler should produce a Tool");

    assert_eq!(tool.path(), std::path::Path::new("cc"));
    assert!(tool.is_like_gnu());
    assert!(!tool.is_like_msvc());
    assert!(!tool.is_like_clang_cl());

    let cc_env = tool.cc_env();
    let cflags_env = tool.cflags_env();




    assert!(
        cc_env.is_empty(),
        "directly configured compiler should not add CC env fragments: {:?}",
        cc_env
    );

    assert!(os_string_contains(
        &cflags_env,
        "-DVALUE_FROM_INTEGRATION_TEST=123"
    ));
    assert!(os_string_contains(&cflags_env, "-fno-omit-frame-pointer"));
    assert!(
        !os_string_contains(&cflags_env, "-fPIC"),
        "removed flag should not be present in cflags_env: {:?}",
        cflags_env
    );

    let args_as_strings: Vec<_> = tool
        .args()
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    assert!(args_as_strings
        .iter()
        .any(|arg| arg == "-DVALUE_FROM_INTEGRATION_TEST=123"));
    assert!(args_as_strings
        .iter()
        .any(|arg| arg == "-fno-omit-frame-pointer"));
    assert!(!args_as_strings.iter().any(|arg| arg == "-fPIC"));
}