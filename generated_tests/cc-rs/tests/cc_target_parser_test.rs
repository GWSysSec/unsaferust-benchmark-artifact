use std::path::{Path, PathBuf};

#[test]
fn build_tracks_files_added_through_chained_workflow() {
    let mut build = cc::Build::new();

    build
        .include("include")
        .includes(["generated/include", "vendor/include"])
        .define("FEATURE_ENABLED", Some("1"))
        .define("EMPTY_DEFINE", None)
        .file("src/alpha.c")
        .files(["src/beta.c", "src/gamma.c"])
        .object("build/existing_alpha.o")
        .objects(["build/existing_beta.o", "build/existing_gamma.o"])
        .flag("-Wall")
        .flag("-Wextra")
        .remove_flag("-Wextra")
        .warnings(true)
        .extra_warnings(false)
        .warnings_into_errors(false)
        .debug(true)
        .opt_level(2)
        .pic(true)
        .cargo_metadata(false)
        .cargo_warnings(false)
        .cargo_debug(false)
        .cargo_output(false)
        .emit_rerun_if_env_changed(false)
        .inherit_rustflags(false);

    let files: Vec<PathBuf> = build.get_files().map(Path::to_path_buf).collect();

    assert_eq!(files.len(), 3);
    assert!(files.contains(&PathBuf::from("src/alpha.c")));
    assert!(files.contains(&PathBuf::from("src/beta.c")));
    assert!(files.contains(&PathBuf::from("src/gamma.c")));
    assert!(!files.contains(&PathBuf::from("build/existing_alpha.o")));
}

#[test]
fn new_build_starts_without_tracked_source_files() {
    let build = cc::Build::new();

    assert_eq!(build.get_files().count(), 0);
}