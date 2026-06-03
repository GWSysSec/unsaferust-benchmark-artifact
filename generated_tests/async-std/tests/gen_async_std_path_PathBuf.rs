use async_std::path::{Path, PathBuf};
use std::ffi::OsString;

#[test]
fn push_extends_path_and_as_path_reflects_changes() {
    let mut pb = PathBuf::from("/home");
    assert_eq!(pb.as_path().to_str(), Some("/home"));
    assert_eq!(pb.as_path().to_str().unwrap().len(), 5);

    pb.push("user");
    assert_eq!(pb.as_path().to_str(), Some("/home/user"));
    assert_ne!(pb.as_path().to_str(), Some("/home"));
    assert_eq!(pb.as_path().to_str().unwrap().len(), 10);

    pb.push("docs");
    assert_eq!(pb.as_path().to_str(), Some("/home/user/docs"));

    pb.push("file.txt");
    assert_eq!(pb.as_path().to_str(), Some("/home/user/docs/file.txt"));
    assert_eq!(pb.as_path().to_str().unwrap().len(), 24);


    pb.push("/etc");
    assert_eq!(pb.as_path().to_str(), Some("/etc"));
    assert_ne!(pb.as_path().to_str(), Some("/home/user/docs/file.txt/etc"));
}

#[test]
fn pop_removes_last_component_from_absolute_path() {
    let mut pb = PathBuf::from("/a/b/c/d");
    assert_eq!(pb.as_path().to_str(), Some("/a/b/c/d"));
    assert_eq!(pb.as_path().to_str().unwrap().len(), 8);

    let r1 = pb.pop();
    assert_eq!(r1, true);
    assert_eq!(pb.as_path().to_str(), Some("/a/b/c"));

    let r2 = pb.pop();
    assert_eq!(r2, true);
    assert_eq!(pb.as_path().to_str(), Some("/a/b"));

    let r3 = pb.pop();
    assert_eq!(r3, true);
    assert_eq!(pb.as_path().to_str(), Some("/a"));

    let r4 = pb.pop();
    assert_eq!(r4, true);
    assert_eq!(pb.as_path().to_str(), Some("/"));


    let r5 = pb.pop();
    assert_eq!(r5, false);
    assert_eq!(pb.as_path().to_str(), Some("/"));
}

#[test]
fn pop_on_relative_and_empty_paths() {
    let mut pb = PathBuf::from("a/b/c");
    assert_eq!(pb.as_path().to_str(), Some("a/b/c"));

    let r1 = pb.pop();
    assert_eq!(r1, true);
    assert_eq!(pb.as_path().to_str(), Some("a/b"));

    let r2 = pb.pop();
    assert_eq!(r2, true);
    assert_eq!(pb.as_path().to_str(), Some("a"));

    let r3 = pb.pop();
    assert_eq!(r3, true);
    assert_eq!(pb.as_path().to_str(), Some(""));


    let r4 = pb.pop();
    assert_eq!(r4, false);
    assert_eq!(pb.as_path().to_str(), Some(""));


    pb.push("only");
    assert_eq!(pb.as_path().to_str(), Some("only"));
    assert_eq!(pb.pop(), true);
    assert_eq!(pb.as_path().to_str(), Some(""));
}

#[test]
fn set_file_name_changes_last_component() {
    let mut pb = PathBuf::from("/tmp/foo.txt");
    assert_eq!(pb.as_path().to_str(), Some("/tmp/foo.txt"));

    pb.set_file_name("bar.log");
    assert_eq!(pb.as_path().to_str(), Some("/tmp/bar.log"));
    assert_ne!(pb.as_path().to_str(), Some("/tmp/foo.txt"));

    pb.set_file_name("baz");
    assert_eq!(pb.as_path().to_str(), Some("/tmp/baz"));

    pb.set_file_name("final.dat");
    assert_eq!(pb.as_path().to_str(), Some("/tmp/final.dat"));
    assert_eq!(pb.as_path().to_str().unwrap().len(), 14);


    let mut pb2 = PathBuf::from("/");
    assert_eq!(pb2.as_path().to_str(), Some("/"));
    pb2.set_file_name("etc");
    assert_eq!(pb2.as_path().to_str(), Some("/etc"));


    let mut pb3 = PathBuf::new();
    assert_eq!(pb3.as_path().to_str(), Some(""));
    pb3.set_file_name("only.ext");
    assert_eq!(pb3.as_path().to_str(), Some("only.ext"));
}

#[test]
fn set_extension_modifies_extension_and_reports_success() {
    let mut pb = PathBuf::from("/tmp/foo.txt");
    assert_eq!(pb.as_path().to_str(), Some("/tmp/foo.txt"));

    let r1 = pb.set_extension("rs");
    assert_eq!(r1, true);
    assert_eq!(pb.as_path().to_str(), Some("/tmp/foo.rs"));

    let r2 = pb.set_extension("bak");
    assert_eq!(r2, true);
    assert_eq!(pb.as_path().to_str(), Some("/tmp/foo.bak"));


    let r3 = pb.set_extension("");
    assert_eq!(r3, true);
    assert_eq!(pb.as_path().to_str(), Some("/tmp/foo"));


    let r4 = pb.set_extension("dat");
    assert_eq!(r4, true);
    assert_eq!(pb.as_path().to_str(), Some("/tmp/foo.dat"));


    let mut pb2 = PathBuf::from("/");
    let r5 = pb2.set_extension("txt");
    assert_eq!(r5, false);
    assert_eq!(pb2.as_path().to_str(), Some("/"));
}

#[test]
fn into_os_string_consumes_pathbuf() {
    let pb = PathBuf::from("/hello/world.txt");
    assert_eq!(pb.as_path().to_str(), Some("/hello/world.txt"));
    assert_eq!(pb.as_path().to_str().unwrap().len(), 16);

    let os_str: OsString = pb.into_os_string();
    assert_eq!(os_str.len(), 16);
    assert_eq!(os_str.to_str(), Some("/hello/world.txt"));
    assert_ne!(os_str.to_str(), Some("/hello/world"));
    assert!(!os_str.is_empty());


    let pb2 = PathBuf::from(&os_str);
    assert_eq!(pb2.as_path().to_str(), Some("/hello/world.txt"));


    let empty = PathBuf::new();
    let os2 = empty.into_os_string();
    assert_eq!(os2.len(), 0);
    assert!(os2.is_empty());
    assert_eq!(os2.to_str(), Some(""));
}

#[test]
fn into_boxed_path_produces_box_with_same_contents() {
    let pb = PathBuf::from("/a/b/c");
    assert_eq!(pb.as_path().to_str(), Some("/a/b/c"));
    assert_eq!(pb.as_path().to_str().unwrap().len(), 6);

    let boxed: Box<Path> = pb.into_boxed_path();
    assert_eq!(boxed.to_str(), Some("/a/b/c"));
    assert_ne!(boxed.to_str(), Some("/a/b"));

    let s = boxed.to_str().unwrap();
    assert_eq!(s.len(), 6);
    assert_eq!(s.chars().filter(|c| *c == '/').count(), 3);
    assert_eq!(s.chars().next(), Some('/'));


    let pb2 = PathBuf::from("/x/y");
    let boxed2: Box<Path> = pb2.into_boxed_path();
    assert_eq!(boxed2.to_str(), Some("/x/y"));
    assert_ne!(boxed2.to_str(), boxed.to_str());
    assert_eq!(boxed2.to_str().unwrap().len(), 4);
}

#[test]
fn complex_workflow_combining_methods() {
    let mut pb = PathBuf::from("/projects");
    assert_eq!(pb.as_path().to_str(), Some("/projects"));

    pb.push("rust");
    pb.push("async-std");
    pb.push("src");
    assert_eq!(pb.as_path().to_str(), Some("/projects/rust/async-std/src"));

    pb.push("main.rs");
    assert_eq!(
        pb.as_path().to_str(),
        Some("/projects/rust/async-std/src/main.rs")
    );

    let changed = pb.set_extension("bak");
    assert_eq!(changed, true);
    assert_eq!(
        pb.as_path().to_str(),
        Some("/projects/rust/async-std/src/main.bak")
    );

    pb.set_file_name("lib.rs");
    assert_eq!(
        pb.as_path().to_str(),
        Some("/projects/rust/async-std/src/lib.rs")
    );

    assert_eq!(pb.pop(), true);
    assert_eq!(pb.as_path().to_str(), Some("/projects/rust/async-std/src"));
    assert_eq!(pb.pop(), true);
    assert_eq!(pb.as_path().to_str(), Some("/projects/rust/async-std"));

    let os = pb.into_os_string();
    assert_eq!(os.to_str(), Some("/projects/rust/async-std"));
    assert_ne!(os.to_str(), Some("/projects/rust"));
    assert_eq!(os.len(), 24);
}