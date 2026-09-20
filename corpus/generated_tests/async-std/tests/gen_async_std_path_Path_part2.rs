use async_std::path::Path;
use async_std::task::block_on;
use std::path::PathBuf;

#[test]
fn test_with_file_name_and_extension() {
    let p = Path::new("/tmp/foo/bar.txt");
    let renamed: PathBuf = p.with_file_name("baz.md").into();
    assert_eq!(renamed, PathBuf::from("/tmp/foo/baz.md"));

    let re_ext: PathBuf = p.with_extension("rs").into();
    assert_eq!(re_ext, PathBuf::from("/tmp/foo/bar.rs"));

    let no_ext: PathBuf = p.with_extension("").into();
    assert_eq!(no_ext, PathBuf::from("/tmp/foo/bar"));

    let p2 = Path::new("relative/file");
    let wfn: PathBuf = p2.with_file_name("other").into();
    assert_eq!(wfn, PathBuf::from("relative/other"));
    let wext: PathBuf = p2.with_extension("log").into();
    assert_eq!(wext, PathBuf::from("relative/file.log"));

    assert_ne!(renamed, PathBuf::from("/tmp/foo/bar.txt"));
    assert_ne!(re_ext, PathBuf::from("/tmp/foo/bar.txt"));
    let wfn2: PathBuf = p.with_file_name("bar.txt").into();
    assert_eq!(wfn2, PathBuf::from("/tmp/foo/bar.txt"));
}

#[test]
fn test_components_and_iter() {
    let p = Path::new("/a/b/c.txt");
    let comps: Vec<_> = p
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(comps.len(), 4);
    assert_eq!(comps[0], "/");
    assert_eq!(comps[1], "a");
    assert_eq!(comps[2], "b");
    assert_eq!(comps[3], "c.txt");

    let iter_items: Vec<_> = p.iter().map(|s| s.to_string_lossy().into_owned()).collect();
    assert_eq!(iter_items.len(), 4);
    assert_eq!(iter_items[3], "c.txt");
    assert_eq!(iter_items[1], "a");

    let rel = Path::new("foo/bar");
    let rel_items: Vec<_> = rel.iter().map(|s| s.to_string_lossy().into_owned()).collect();
    assert_eq!(rel_items, vec!["foo".to_string(), "bar".to_string()]);
    assert_ne!(rel_items.len(), 3);
}

#[test]
fn test_exists_is_file_is_dir() {
    block_on(async {
        let tmp = std::env::temp_dir().join("async_std_path_test_xyz_12345");
        let _ = async_std::fs::remove_dir_all(&tmp).await;

        let tmp_p = Path::new(tmp.to_str().unwrap());
        assert_eq!(tmp_p.exists().await, false);
        assert_eq!(tmp_p.is_dir().await, false);
        assert_eq!(tmp_p.is_file().await, false);

        async_std::fs::create_dir_all(&tmp).await.unwrap();
        assert_eq!(tmp_p.exists().await, true);
        assert_eq!(tmp_p.is_dir().await, true);
        assert_eq!(tmp_p.is_file().await, false);

        let file_path = tmp.join("hello.txt");
        async_std::fs::write(&file_path, b"hello").await.unwrap();
        let fp = Path::new(file_path.to_str().unwrap());
        assert_eq!(fp.exists().await, true);
        assert_eq!(fp.is_file().await, true);
        assert_eq!(fp.is_dir().await, false);

        let canon = fp.canonicalize().await.unwrap();
        assert!(canon.to_string_lossy().ends_with("hello.txt"));

        let smeta = fp.symlink_metadata().await.unwrap();
        assert_eq!(smeta.is_file(), true);
        assert_eq!(smeta.len(), 5);

        let mut rd = tmp_p.read_dir().await.unwrap();
        use async_std::stream::StreamExt;
        let mut count = 0;
        while let Some(entry) = rd.next().await {
            let _ = entry.unwrap();
            count += 1;
        }
        assert_eq!(count, 1);

        async_std::fs::remove_dir_all(&tmp).await.unwrap();
        assert_eq!(tmp_p.exists().await, false);
    });
}

#[test]
#[cfg(unix)]
fn test_read_link_and_into_path_buf() {
    block_on(async {
        let tmp = std::env::temp_dir().join("async_std_path_symlink_test_987");
        let _ = async_std::fs::remove_dir_all(&tmp).await;
        async_std::fs::create_dir_all(&tmp).await.unwrap();

        let target = tmp.join("target.txt");
        async_std::fs::write(&target, b"data").await.unwrap();

        let link = tmp.join("link.txt");
        async_std::os::unix::fs::symlink(&target, &link).await.unwrap();

        let link_p = Path::new(link.to_str().unwrap());
        assert_eq!(link_p.exists().await, true);
        assert_eq!(link_p.is_file().await, true);

        let resolved: PathBuf = link_p.read_link().await.unwrap().into();
        assert_eq!(resolved, PathBuf::from(&target));

        let smeta = link_p.symlink_metadata().await.unwrap();
        assert_eq!(smeta.file_type().is_symlink(), true);

        let async_pb = async_std::path::PathBuf::from("/foo/bar");
        let pb: PathBuf = async_pb.into();
        assert_eq!(pb, PathBuf::from("/foo/bar"));
        assert_ne!(pb, PathBuf::from("/foo"));

        async_std::fs::remove_dir_all(&tmp).await.unwrap();
    });
}