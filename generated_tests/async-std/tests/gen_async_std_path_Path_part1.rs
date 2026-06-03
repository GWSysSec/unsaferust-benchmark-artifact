use async_std::path::Path;
use async_std::path::PathBuf;
use std::ffi::OsStr;

#[test]
fn test_path_as_os_str_and_to_str() {
    let path = Path::new("/usr/local/bin/rustc");

    let os_str = path.as_os_str();
    assert_eq!(os_str, OsStr::new("/usr/local/bin/rustc"));

    let str_repr = path.to_str();
    assert_eq!(str_repr, Some("/usr/local/bin/rustc"));

    let lossy = path.to_string_lossy();
    assert_eq!(lossy, "/usr/local/bin/rustc");


    let empty_path = Path::new("");
    assert_eq!(empty_path.as_os_str(), OsStr::new(""));
    assert_eq!(empty_path.to_str(), Some(""));
    assert_eq!(empty_path.to_string_lossy(), "");


    let special = Path::new("/tmp/hello world/file.txt");
    assert_eq!(special.to_str(), Some("/tmp/hello world/file.txt"));
    assert_eq!(special.as_os_str().len(), 25);
}

#[test]
fn test_path_is_absolute_and_is_relative() {
    let abs_path = Path::new("/etc/passwd");
    assert!(abs_path.is_absolute());
    assert!(!abs_path.is_relative());

    let rel_path = Path::new("src/main.rs");
    assert!(!rel_path.is_absolute());
    assert!(rel_path.is_relative());

    let root = Path::new("/");
    assert!(root.is_absolute());
    assert!(!root.is_relative());

    let dot = Path::new(".");
    assert!(!dot.is_absolute());
    assert!(dot.is_relative());

    let dotdot = Path::new("../foo");
    assert!(!dotdot.is_absolute());
    assert!(dotdot.is_relative());
}

#[test]
fn test_path_has_root() {
    let abs_path = Path::new("/home/user");
    assert!(abs_path.has_root());

    let rel_path = Path::new("home/user");
    assert!(!rel_path.has_root());

    let root_only = Path::new("/");
    assert!(root_only.has_root());

    let empty = Path::new("");
    assert!(!empty.has_root());

    let dot_relative = Path::new("./foo/bar");
    assert!(!dot_relative.has_root());

    let slash_prefix = Path::new("/foo");
    assert!(slash_prefix.has_root());


    assert_eq!(abs_path.is_absolute(), abs_path.has_root());
    assert_eq!(rel_path.is_absolute(), rel_path.has_root());
}

#[test]
fn test_path_parent_and_ancestors() {
    let path = Path::new("/usr/local/bin/rustc");

    let parent = path.parent();
    assert!(parent.is_some());
    assert_eq!(parent.unwrap(), Path::new("/usr/local/bin"));

    let grandparent = parent.unwrap().parent();
    assert!(grandparent.is_some());
    assert_eq!(grandparent.unwrap(), Path::new("/usr/local"));


    let root = Path::new("/");
    assert_eq!(root.parent(), None);


    let ancestors: Vec<&Path> = path.ancestors().collect();
    assert_eq!(ancestors.len(), 5);
    assert_eq!(ancestors[0], Path::new("/usr/local/bin/rustc"));
    assert_eq!(ancestors[1], Path::new("/usr/local/bin"));
    assert_eq!(ancestors[2], Path::new("/usr/local"));
    assert_eq!(ancestors[3], Path::new("/usr"));
    assert_eq!(ancestors[4], Path::new("/"));
}

#[test]
fn test_path_file_name_and_file_stem_and_extension() {
    let path = Path::new("/tmp/archive.tar.gz");

    let file_name = path.file_name();
    assert_eq!(file_name, Some(OsStr::new("archive.tar.gz")));

    let file_stem = path.file_stem();
    assert_eq!(file_stem, Some(OsStr::new("archive.tar")));

    let extension = path.extension();
    assert_eq!(extension, Some(OsStr::new("gz")));


    let no_ext = Path::new("/usr/bin/rustc");
    assert_eq!(no_ext.file_name(), Some(OsStr::new("rustc")));
    assert_eq!(no_ext.file_stem(), Some(OsStr::new("rustc")));
    assert_eq!(no_ext.extension(), None);


    let dotfile = Path::new("/home/user/.bashrc");
    assert_eq!(dotfile.file_name(), Some(OsStr::new(".bashrc")));
    assert_eq!(dotfile.file_stem(), Some(OsStr::new(".bashrc")));
    assert_eq!(dotfile.extension(), None);


    let dir_path = Path::new("/tmp/mydir");
    assert_eq!(dir_path.file_name(), Some(OsStr::new("mydir")));
}

#[test]
fn test_path_strip_prefix() {
    let path = Path::new("/usr/local/bin/rustc");

    let stripped = path.strip_prefix("/usr/local");
    assert!(stripped.is_ok());
    assert_eq!(stripped.unwrap(), Path::new("bin/rustc"));

    let stripped2 = path.strip_prefix("/usr");
    assert!(stripped2.is_ok());
    assert_eq!(stripped2.unwrap(), Path::new("local/bin/rustc"));

    let stripped3 = path.strip_prefix("/usr/local/bin/rustc");
    assert!(stripped3.is_ok());
    assert_eq!(stripped3.unwrap(), Path::new(""));


    let bad_strip = path.strip_prefix("/etc");
    assert!(bad_strip.is_err());

    let bad_strip2 = path.strip_prefix("/usr/local/lib");
    assert!(bad_strip2.is_err());


    let stripped_root = path.strip_prefix("/");
    assert!(stripped_root.is_ok());
    assert_eq!(stripped_root.unwrap(), Path::new("usr/local/bin/rustc"));
}

#[test]
fn test_path_starts_with_and_ends_with() {
    let path = Path::new("/usr/local/bin/rustc");

    assert!(path.starts_with("/usr"));
    assert!(path.starts_with("/usr/local"));
    assert!(path.starts_with("/usr/local/bin"));
    assert!(path.starts_with("/usr/local/bin/rustc"));
    assert!(path.starts_with("/"));
    assert!(!path.starts_with("/etc"));
    assert!(!path.starts_with("/usr/loc"));

    assert!(path.ends_with("rustc"));
    assert!(path.ends_with("bin/rustc"));
    assert!(path.ends_with("local/bin/rustc"));
    assert!(!path.ends_with("stc"));
    assert!(!path.ends_with("/usr"));
}

#[test]
fn test_path_join() {
    let base = Path::new("/usr/local");

    let joined = base.join("bin");
    assert_eq!(joined, PathBuf::from("/usr/local/bin"));

    let joined2 = base.join("bin/rustc");
    assert_eq!(joined2, PathBuf::from("/usr/local/bin/rustc"));


    let joined_abs = base.join("/etc/passwd");
    assert_eq!(joined_abs, PathBuf::from("/etc/passwd"));


    let joined_empty = base.join("");
    assert_eq!(joined_empty, PathBuf::from("/usr/local"));


    let chained = Path::new("/").join("home").join("user").join("documents");
    assert_eq!(chained, PathBuf::from("/home/user/documents"));


    let rel = Path::new("src");
    let joined_rel = rel.join("main.rs");
    assert_eq!(joined_rel, PathBuf::from("src/main.rs"));
    assert!(!joined_rel.is_absolute());
    assert!(joined_rel.is_relative());
}

#[test]
fn test_path_complex_workflow() {

    let project_root = Path::new("/home/developer/projects/myapp");


    assert!(project_root.is_absolute());
    assert!(project_root.has_root());
    assert!(!project_root.is_relative());


    let src_dir = project_root.join("src");
    assert_eq!(src_dir, PathBuf::from("/home/developer/projects/myapp/src"));


    let main_file = src_dir.join("main.rs");
    assert_eq!(main_file.file_name(), Some(OsStr::new("main.rs")));
    assert_eq!(main_file.file_stem(), Some(OsStr::new("main")));
    assert_eq!(main_file.extension(), Some(OsStr::new("rs")));


    let relative = main_file.strip_prefix(project_root);
    assert!(relative.is_ok());
    assert_eq!(relative.unwrap(), Path::new("src/main.rs"));


    let parent = main_file.parent();
    assert_eq!(parent, Some(Path::new("/home/developer/projects/myapp/src")));


    assert!(main_file.starts_with(project_root));
    assert!(main_file.ends_with("main.rs"));
    assert!(main_file.ends_with("src/main.rs"));


    let ancestor_count = main_file.ancestors().count();
    assert_eq!(ancestor_count, 7);
}

#[test]
fn test_path_edge_cases() {

    let single = Path::new("file.txt");
    assert_eq!(single.parent(), Some(Path::new("")));
    assert_eq!(single.file_name(), Some(OsStr::new("file.txt")));
    assert!(!single.is_absolute());
    assert!(single.is_relative());
    assert!(!single.has_root());


    let multi_dot = Path::new("/path/to/file.backup.2023.tar.gz");
    assert_eq!(multi_dot.file_stem(), Some(OsStr::new("file.backup.2023.tar")));
    assert_eq!(multi_dot.extension(), Some(OsStr::new("gz")));


    let trailing = Path::new("/usr/local/");
    assert_eq!(trailing.file_name(), Some(OsStr::new("local")));
    assert_eq!(trailing.parent(), Some(Path::new("/usr")));


    let valid_utf8 = Path::new("/valid/utf8/path");
    let lossy = valid_utf8.to_string_lossy();
    assert_eq!(&*lossy, "/valid/utf8/path");


    let root_ancestors: Vec<&Path> = Path::new("/").ancestors().collect();
    assert_eq!(root_ancestors.len(), 1);
    assert_eq!(root_ancestors[0], Path::new("/"));
}