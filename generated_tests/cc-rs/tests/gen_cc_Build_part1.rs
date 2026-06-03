use std::env;
use std::path::Path;

mod support;
use crate::support::Test;

#[test]
fn test_includes_single_and_multiple() {
    let test = Test::gnu();

    let mut binding = test.gcc();
    let build = binding
        .file("foo.c")
        .include("/usr/include")
        .include("/usr/local/include");

    let compiler = build.get_compiler();
    let args = compiler.args();
    let args_str: Vec<&str> = args.iter().map(|a| a.to_str().unwrap()).collect();

    assert!(args_str.contains(&"-I"));
    assert!(args_str.iter().any(|a| a.contains("/usr/include")));
    assert!(args_str.iter().any(|a| a.contains("/usr/local/include")));


    let test2 = Test::gnu();
    let mut binding2 = test2.gcc();
    let build2 = binding2
        .file("foo.c")
        .includes(&["/opt/include", "/tmp/include"]);

    let compiler2 = build2.get_compiler();
    let args2 = compiler2.args();
    let args2_str: Vec<&str> = args2.iter().map(|a| a.to_str().unwrap()).collect();

    assert!(args2_str.iter().any(|a| a.contains("/opt/include")));
    assert!(args2_str.iter().any(|a| a.contains("/tmp/include")));


    let test3 = Test::gnu();
    let mut binding3 = test3.gcc();
    let build3 = binding3
        .file("foo.c")
        .includes(&["/a", "/b", "/c"]);

    let compiler3 = build3.get_compiler();
    let args3 = compiler3.args();
    let args3_str: Vec<&str> = args3.iter().map(|a| a.to_str().unwrap()).collect();
    assert!(args3_str.iter().any(|a| a.contains("/a")));
    assert!(args3_str.iter().any(|a| a.contains("/b")));
    assert!(args3_str.iter().any(|a| a.contains("/c")));
}

#[test]
fn test_get_files_and_files() {
    let test = Test::gnu();


    let build_empty = test.gcc();
    let file_count_empty = build_empty.get_files().count();
    assert_eq!(file_count_empty, 0);


    let mut build = cc::Build::new();
    build.file("src/foo.c");
    let files: Vec<&Path> = build.get_files().collect();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], Path::new("src/foo.c"));


    let mut build2 = cc::Build::new();
    build2.files(&["a.c", "b.c", "c.c"]);
    let files2: Vec<&Path> = build2.get_files().collect();
    assert_eq!(files2.len(), 3);
    assert_eq!(files2[0], Path::new("a.c"));
    assert_eq!(files2[1], Path::new("b.c"));
    assert_eq!(files2[2], Path::new("c.c"));


    let mut build3 = cc::Build::new();
    build3.file("first.c").files(&["second.c", "third.c"]);
    let files3: Vec<&Path> = build3.get_files().collect();
    assert_eq!(files3.len(), 3);
    assert_eq!(files3[0], Path::new("first.c"));
    assert_eq!(files3[1], Path::new("second.c"));
    assert_eq!(files3[2], Path::new("third.c"));
}

#[test]
fn test_no_default_flags() {
    let test = Test::gnu();


    test.gcc()
        .no_default_flags(true)
        .file("foo.c")
        .compile("foo");


    test.cmd(0).must_not_have("-O2");


    let test2 = Test::gnu();
    test2
        .gcc()
        .no_default_flags(false)
        .file("foo.c")
        .compile("foo");



    test2.cmd(0).must_have("foo.c");


    let test3 = Test::gnu();
    test3
        .gcc()
        .no_default_flags(true)
        .flag("-custom-flag")
        .file("foo.c")
        .compile("foo");

    test3.cmd(0).must_have("-custom-flag");
    test3.cmd(0).must_have("foo.c");
    test3.cmd(0).must_not_have("-O2");
}

#[test]
fn test_remove_flag() {
    let test = Test::gnu();


    test.gcc()
        .no_default_flags(true)
        .flag("-Wall")
        .flag("-Wextra")
        .flag("-Werror")
        .flag_if_supported("-Wextra-removal-placeholder")
        .file("foo.c")
        .compile("foo");





    test.cmd(0).must_have("-Wall");
    test.cmd(0).must_have("-Werror");
    test.cmd(0).must_have("foo.c");


    let test2 = Test::gnu();
    test2
        .gcc()
        .no_default_flags(true)
        .flag("-Wall")
        .file("foo.c")
        .compile("foo");

    test2.cmd(0).must_have("-Wall");
    test2.cmd(0).must_not_have("-nonexistent");
    test2.cmd(0).must_have("foo.c");
}

#[test]
fn test_try_flags_from_environment() {

    env::set_var("CC_TEST_FLAGS_CUSTOM", "-DFOO -DBAR");

    let test = Test::gnu();
    let mut binding = test.gcc();
    let result = binding
        .file("foo.c")
        .try_flags_from_environment("CC_TEST_FLAGS_CUSTOM");

    assert!(result.is_ok());


    env::remove_var("CC_TEST_FLAGS_NONEXISTENT_12345");
    let test2 = Test::gnu();
    let mut binding2 = test2.gcc();
    let result2 = binding2
        .file("foo.c")
        .try_flags_from_environment("CC_TEST_FLAGS_NONEXISTENT_12345");

    assert!(result2.is_err());


    env::set_var("CC_TEST_FLAGS_EMPTY", "");
    let test3 = Test::gnu();
    let mut binding3 = test3.gcc();
    let result3 = binding3
        .file("foo.c")
        .try_flags_from_environment("CC_TEST_FLAGS_EMPTY");


    assert!(result3.is_ok());


    let test4 = Test::gnu();
    env::remove_var("CC_TEST_FLAGS_NONEXISTENT_12345");
    let mut binding4 = test4.gcc();
    let result4 = binding4
        .file("foo.c")
        .try_flags_from_environment("CC_TEST_FLAGS_NONEXISTENT_12345");
    assert!(result4.is_err());
    let err = result4.unwrap_err();
    let err_clone = err.clone();

    assert_eq!(format!("{}", err), format!("{}", err_clone));

    env::remove_var("CC_TEST_FLAGS_CUSTOM");
    env::remove_var("CC_TEST_FLAGS_EMPTY");
}

#[test]
fn test_object_and_objects() {
    let test = Test::gnu();


    test.gcc()
        .file("foo.c")
        .object("extra.o")
        .compile("foo");



    test.cmd(0).must_have("foo.c");


    let test2 = Test::gnu();
    test2
        .gcc()
        .file("foo.c")
        .objects(&["obj1.o", "obj2.o", "obj3.o"])
        .compile("foo");

    test2.cmd(0).must_have("foo.c");


    let mut build = cc::Build::new();
    build
        .file("test.c")
        .object("a.o")
        .object("b.o")
        .objects(&["c.o", "d.o"]);

    let files: Vec<&Path> = build.get_files().collect();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], Path::new("test.c"));
}

#[test]
fn test_ar_flag() {
    let test = Test::gnu();

    test.gcc()
        .file("foo.c")
        .ar_flag("--plugin")
        .ar_flag("someplugin")
        .compile("foo");


    test.cmd(0).must_have("foo.c");


    let test2 = Test::gnu();
    test2
        .gcc()
        .file("foo.c")
        .ar_flag("-v")
        .compile("foo");

    test2.cmd(0).must_have("foo.c");


    let test3 = Test::gnu();
    test3
        .gcc()
        .file("foo.c")
        .ar_flag("--thin")
        .ar_flag("-D")
        .compile("foo");

    test3.cmd(0).must_have("foo.c");
}

#[test]
fn test_cpp_link_stdlib() {
    let test = Test::gnu();


    test.gcc()
        .cpp(true)
        .cpp_link_stdlib(Some("stdc++"))
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("foo.c");


    let test2 = Test::gnu();
    test2
        .gcc()
        .cpp(true)
        .cpp_link_stdlib(None::<&str>)
        .file("foo.c")
        .compile("foo");

    test2.cmd(0).must_have("foo.c");


    let test3 = Test::gnu();
    test3
        .gcc()
        .cpp(true)
        .cpp_link_stdlib(Some("c++"))
        .flag("-std=c++17")
        .file("foo.c")
        .compile("foo");

    test3.cmd(0).must_have("foo.c");
    test3.cmd(0).must_have("-std=c++17");
}

#[test]
fn test_cargo_output_and_cargo_debug() {
    let test = Test::gnu();


    test.gcc()
        .cargo_output(false)
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("foo.c");


    let test2 = Test::gnu();
    test2
        .gcc()
        .cargo_debug(true)
        .file("foo.c")
        .compile("foo");

    test2.cmd(0).must_have("foo.c");


    let test3 = Test::gnu();
    test3
        .gcc()
        .cargo_output(false)
        .cargo_debug(false)
        .file("foo.c")
        .compile("foo");

    test3.cmd(0).must_have("foo.c");


    let test4 = Test::gnu();
    test4
        .gcc()
        .cargo_output(true)
        .cargo_debug(true)
        .flag("-DTEST")
        .file("foo.c")
        .compile("foo");

    test4.cmd(0).must_have("foo.c");
    test4.cmd(0).must_have("-DTEST");
}

#[test]
fn test_ranlib() {
    let test = Test::gnu();

    test.gcc()
        .file("foo.c")
        .ranlib("my-ranlib")
        .compile("foo");

    test.cmd(0).must_have("foo.c");


    let test2 = Test::gnu();
    test2
        .gcc()
        .file("foo.c")
        .ranlib("/usr/bin/ranlib")
        .compile("foo");

    test2.cmd(0).must_have("foo.c");


    let test3 = Test::gnu();
    test3
        .gcc()
        .ranlib("custom-ranlib")
        .flag("-O2")
        .file("foo.c")
        .compile("foo");

    test3.cmd(0).must_have("foo.c");
    test3.cmd(0).must_have("-O2");
}

#[test]
fn test_ccbin() {
    let test = Test::gnu();


    test.gcc()
        .ccbin(true)
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("foo.c");

    let test2 = Test::gnu();
    test2
        .gcc()
        .ccbin(false)
        .file("foo.c")
        .compile("foo");

    test2.cmd(0).must_have("foo.c");


    let test3 = Test::gnu();
    test3
        .gcc()
        .ccbin(true)
        .flag("-DUSE_CCBIN")
        .file("foo.c")
        .compile("foo");

    test3.cmd(0).must_have("foo.c");
    test3.cmd(0).must_have("-DUSE_CCBIN");
}

#[test]
fn test_link_lib_modifier() {
    let test = Test::gnu();

    test.gcc()
        .link_lib_modifier("+whole-archive")
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("foo.c");


    let test2 = Test::gnu();
    test2
        .gcc()
        .link_lib_modifier("+bundle")
        .file("foo.c")
        .compile("foo");

    test2.cmd(0).must_have("foo.c");


    let test3 = Test::gnu();
    test3
        .gcc()
        .link_lib_modifier("+verbatim")
        .flag("-Wall")
        .file("foo.c")
        .compile("foo");

    test3.cmd(0).must_have("foo.c");
    test3.cmd(0).must_have("-Wall");
}

#[test]
fn test_combined_workflow() {
    let test = Test::gnu();


    test.gcc()
        .no_default_flags(true)
        .includes(&["/usr/include", "/opt/local/include"])
        .flag("-Wall")
        .flag("-Wextra")
        .flag("-pedantic")
        .ar_flag("-D")
        .ranlib("ranlib")
        .cargo_output(false)
        .cargo_debug(false)
        .cpp_link_stdlib(Some("stdc++"))
        .ccbin(false)
        .file("main.c")
        .files(&["util.c", "helper.c"])
        .object("prebuilt.o")
        .compile("mylib");


    test.cmd(0).must_have("-Wall");
    test.cmd(0).must_have("-Wextra");
    test.cmd(0).must_have("-pedantic");
    test.cmd(0).must_not_have("-O2");


    let mut build = cc::Build::new();
    build
        .file("alpha.c")
        .files(&["beta.c", "gamma.c"])
        .file("delta.c");

    let files: Vec<&Path> = build.get_files().collect();
    assert_eq!(files.len(), 4);
    assert_eq!(files[0], Path::new("alpha.c"));
    assert_eq!(files[1], Path::new("beta.c"));
    assert_eq!(files[2], Path::new("gamma.c"));
    assert_eq!(files[3], Path::new("delta.c"));
}