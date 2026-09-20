#![cfg(not(target_os = "unknown"))]

use async_std::fs;
use async_std::task;
use async_std::os::unix::fs::symlink;
use async_std::stream::StreamExt;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(prefix: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("async_std_test_{}_{}", prefix, ts))
}

#[test]
fn test_create_dir_and_remove_dir() {
    task::block_on(async {
        let dir = unique_dir("create_remove_dir");

        // Pre-state: directory does not exist
        let meta_result = fs::metadata(&dir).await;
        assert!(meta_result.is_err());

        // Create directory
        let create_result = fs::create_dir(&dir).await;
        assert!(create_result.is_ok());

        // Post-state: directory exists and is a directory
        let meta = fs::metadata(&dir).await.unwrap();
        assert!(meta.is_dir());
        assert!(!meta.is_file());

        // Creating the same directory again should fail
        let dup_result = fs::create_dir(&dir).await;
        assert!(dup_result.is_err());

        // Remove the directory
        let remove_result = fs::remove_dir(&dir).await;
        assert!(remove_result.is_ok());

        // Post-state: directory no longer exists
        let meta_after = fs::metadata(&dir).await;
        assert!(meta_after.is_err());

        // Removing non-existent directory should fail
        let remove_again = fs::remove_dir(&dir).await;
        assert!(remove_again.is_err());
    });
}

#[test]
fn test_create_dir_all_and_remove_dir_all() {
    task::block_on(async {
        let base = unique_dir("create_dir_all");
        let nested = base.join("a").join("b").join("c");

        // Pre-state: none of the directories exist
        assert!(fs::metadata(&base).await.is_err());
        assert!(fs::metadata(&nested).await.is_err());

        // Create nested directories
        let result = fs::create_dir_all(&nested).await;
        assert!(result.is_ok());

        // All levels exist
        let base_meta = fs::metadata(&base).await.unwrap();
        assert!(base_meta.is_dir());
        let nested_meta = fs::metadata(&nested).await.unwrap();
        assert!(nested_meta.is_dir());

        // Create a file inside the nested dir
        let file_path = nested.join("test.txt");
        fs::write(&file_path, b"hello").await.unwrap();
        assert!(fs::metadata(&file_path).await.unwrap().is_file());

        // remove_dir_all removes everything recursively
        let remove_result = fs::remove_dir_all(&base).await;
        assert!(remove_result.is_ok());

        // Post-state: base no longer exists
        assert!(fs::metadata(&base).await.is_err());
        assert!(fs::metadata(&nested).await.is_err());
        assert!(fs::metadata(&file_path).await.is_err());
    });
}

#[test]
fn test_read_to_string_and_remove_file() {
    task::block_on(async {
        let dir = unique_dir("read_to_string");
        fs::create_dir(&dir).await.unwrap();

        let file_path = dir.join("greeting.txt");
        let content = "Hello, async-std world!\nLine two.";

        // Write file
        fs::write(&file_path, content.as_bytes()).await.unwrap();

        // Read it back as string
        let read_back = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(read_back, content);
        assert_eq!(read_back.len(), content.len());
        assert!(read_back.contains("async-std"));
        assert!(read_back.starts_with("Hello"));

        // Remove the file
        let remove_result = fs::remove_file(&file_path).await;
        assert!(remove_result.is_ok());

        // Post-state: file no longer exists
        let read_after = fs::read_to_string(&file_path).await;
        assert!(read_after.is_err());

        // Removing again should fail
        let remove_again = fs::remove_file(&file_path).await;
        assert!(remove_again.is_err());

        // Cleanup
        fs::remove_dir(&dir).await.unwrap();
    });
}

#[test]
fn test_set_permissions() {
    task::block_on(async {
        let dir = unique_dir("set_perms");
        fs::create_dir(&dir).await.unwrap();

        let file_path = dir.join("perms_test.txt");
        fs::write(&file_path, b"permissions test").await.unwrap();

        // Check initial permissions (should be readable)
        let meta_before = fs::metadata(&file_path).await.unwrap();
        let perms_before = meta_before.permissions();
        assert!(!perms_before.readonly());

        // Set to readonly (mode 0o444)
        let readonly_perms = std::fs::Permissions::from_mode(0o444);
        let set_result = fs::set_permissions(&file_path, readonly_perms).await;
        assert!(set_result.is_ok());

        // Verify permissions changed
        let meta_after = fs::metadata(&file_path).await.unwrap();
        let perms_after = meta_after.permissions();
        assert!(perms_after.readonly());
        assert_eq!(perms_after.mode() & 0o777, 0o444);

        // Set back to read-write (mode 0o644)
        let rw_perms = std::fs::Permissions::from_mode(0o644);
        fs::set_permissions(&file_path, rw_perms).await.unwrap();

        let meta_final = fs::metadata(&file_path).await.unwrap();
        assert!(!meta_final.permissions().readonly());
        assert_eq!(meta_final.permissions().mode() & 0o777, 0o644);

        // Cleanup
        fs::remove_file(&file_path).await.unwrap();
        fs::remove_dir(&dir).await.unwrap();
    });
}

#[test]
fn test_hard_link() {
    task::block_on(async {
        let dir = unique_dir("hard_link");
        fs::create_dir(&dir).await.unwrap();

        let original = dir.join("original.txt");
        let link_path = dir.join("hardlink.txt");
        let content = "hard link content data";

        fs::write(&original, content.as_bytes()).await.unwrap();

        // Create hard link
        let link_result = fs::hard_link(&original, &link_path).await;
        assert!(link_result.is_ok());

        // Both files should have the same content
        let original_content = fs::read_to_string(&original).await.unwrap();
        let link_content = fs::read_to_string(&link_path).await.unwrap();
        assert_eq!(original_content, content);
        assert_eq!(link_content, content);
        assert_eq!(original_content, link_content);

        // Both should be regular files
        let orig_meta = fs::metadata(&original).await.unwrap();
        let link_meta = fs::metadata(&link_path).await.unwrap();
        assert!(orig_meta.is_file());
        assert!(link_meta.is_file());

        // They should have the same length
        assert_eq!(orig_meta.len(), link_meta.len());
        assert_eq!(orig_meta.len(), content.len() as u64);

        // Removing original should not affect the hard link
        fs::remove_file(&original).await.unwrap();
        let link_still = fs::read_to_string(&link_path).await.unwrap();
        assert_eq!(link_still, content);

        // Cleanup
        fs::remove_file(&link_path).await.unwrap();
        fs::remove_dir(&dir).await.unwrap();
    });
}

#[test]
fn test_canonicalize() {
    task::block_on(async {
        let dir = unique_dir("canonicalize");
        fs::create_dir_all(&dir).await.unwrap();

        let file_path = dir.join("canon_test.txt");
        fs::write(&file_path, b"canon").await.unwrap();

        // Canonicalize the file path
        let canonical = fs::canonicalize(&file_path).await.unwrap();
        assert!(canonical.is_absolute());
        assert!(canonical.to_str().unwrap().len() > 0);

        // Canonicalize with a relative component
        let with_dots = dir.join(".").join("canon_test.txt");
        let canonical2 = fs::canonicalize(&with_dots).await.unwrap();
        assert_eq!(canonical, canonical2);

        // Canonicalize the directory itself
        let dir_canonical = fs::canonicalize(&dir).await.unwrap();
        assert!(dir_canonical.is_absolute());

        // The file's canonical parent should match the dir's canonical path
        let file_parent = canonical.parent().unwrap();
        assert_eq!(file_parent, dir_canonical.as_path());

        // Canonicalizing a non-existent path should fail
        let nonexistent = dir.join("does_not_exist.txt");
        let err_result = fs::canonicalize(&nonexistent).await;
        assert!(err_result.is_err());

        // Cleanup
        fs::remove_file(&file_path).await.unwrap();
        fs::remove_dir(&dir).await.unwrap();
    });
}

#[test]
fn test_rename() {
    task::block_on(async {
        let dir = unique_dir("rename");
        fs::create_dir(&dir).await.unwrap();

        let src = dir.join("source.txt");
        let dst = dir.join("destination.txt");
        let content = "rename me please";

        fs::write(&src, content.as_bytes()).await.unwrap();

        // Pre-state
        assert!(fs::metadata(&src).await.is_ok());
        assert!(fs::metadata(&dst).await.is_err());

        // Rename
        let rename_result = fs::rename(&src, &dst).await;
        assert!(rename_result.is_ok());

        // Post-state: source gone, destination exists with same content
        assert!(fs::metadata(&src).await.is_err());
        let dst_content = fs::read_to_string(&dst).await.unwrap();
        assert_eq!(dst_content, content);

        // Rename to overwrite an existing file
        let dst2 = dir.join("other.txt");
        fs::write(&dst2, b"will be overwritten").await.unwrap();
        fs::rename(&dst, &dst2).await.unwrap();

        let final_content = fs::read_to_string(&dst2).await.unwrap();
        assert_eq!(final_content, content);
        assert!(fs::metadata(&dst).await.is_err());

        // Cleanup
        fs::remove_file(&dst2).await.unwrap();
        fs::remove_dir(&dir).await.unwrap();
    });
}

#[test]
fn test_read_dir() {
    task::block_on(async {
        let dir = unique_dir("read_dir");
        fs::create_dir(&dir).await.unwrap();

        // Create several files
        let names = vec!["alpha.txt", "beta.txt", "gamma.txt"];
        for name in &names {
            fs::write(dir.join(name), format!("content of {}", name).as_bytes())
                .await
                .unwrap();
        }

        // Create a subdirectory
        let subdir = dir.join("subdir");
        fs::create_dir(&subdir).await.unwrap();

        // Read directory entries
        let mut entries = fs::read_dir(&dir).await.unwrap();
        let mut found_names: Vec<String> = Vec::new();

        while let Some(entry) = entries.next().await {
            let entry = entry.unwrap();
            found_names.push(entry.file_name().to_string_lossy().to_string());
        }

        found_names.sort();

        assert_eq!(found_names.len(), 4); // 3 files + 1 subdir
        assert!(found_names.contains(&"alpha.txt".to_string()));
        assert!(found_names.contains(&"beta.txt".to_string()));
        assert!(found_names.contains(&"gamma.txt".to_string()));
        assert!(found_names.contains(&"subdir".to_string()));

        // Reading a non-existent directory should fail
        let bad_dir = dir.join("nonexistent");
        let err = fs::read_dir(&bad_dir).await;
        assert!(err.is_err());

        // Cleanup
        for name in &names {
            fs::remove_file(dir.join(name)).await.unwrap();
        }
        fs::remove_dir(&subdir).await.unwrap();
        fs::remove_dir(&dir).await.unwrap();
    });
}

#[test]
fn test_read_link_and_symlink_metadata() {
    task::block_on(async {
        let dir = unique_dir("read_link");
        fs::create_dir(&dir).await.unwrap();

        let target = dir.join("target.txt");
        let link_path = dir.join("symlink.txt");
        let content = "symlink target content";

        fs::write(&target, content.as_bytes()).await.unwrap();

        // Create symlink
        symlink(&target, &link_path).await.unwrap();

        // read_link should return the target path
        let read_link_result = fs::read_link(&link_path).await.unwrap();
        // Convert async_std::path::PathBuf to std::path::PathBuf for comparison
        let read_link_std: std::path::PathBuf = read_link_result.into();
        assert_eq!(read_link_std, target);

        // Reading through the symlink should give the same content
        let link_content = fs::read_to_string(&link_path).await.unwrap();
        assert_eq!(link_content, content);

        // symlink_metadata should report it as a symlink
        let sym_meta = fs::symlink_metadata(&link_path).await.unwrap();
        assert!(sym_meta.file_type().is_symlink());
        assert!(!sym_meta.is_dir());

        // Regular metadata follows the symlink
        let regular_meta = fs::metadata(&link_path).await.unwrap();
        assert!(regular_meta.is_file());
        assert!(!regular_meta.file_type().is_symlink());

        // Target metadata should be a regular file
        let target_meta = fs::symlink_metadata(&target).await.unwrap();
        assert!(!target_meta.file_type().is_symlink());
        assert!(target_meta.is_file());
        assert_eq!(target_meta.len(), content.len() as u64);

        // read_link on a non-symlink should fail
        let non_link_result = fs::read_link(&target).await;
        assert!(non_link_result.is_err());

        // Cleanup
        fs::remove_file(&link_path).await.unwrap();
        fs::remove_file(&target).await.unwrap();
        fs::remove_dir(&dir).await.unwrap();
    });
}

#[test]
fn test_remove_file_edge_cases() {
    task::block_on(async {
        let dir = unique_dir("remove_file_edge");
        fs::create_dir(&dir).await.unwrap();

        // Cannot remove a directory with remove_file
        let subdir = dir.join("subdir");
        fs::create_dir(&subdir).await.unwrap();
        let remove_dir_as_file = fs::remove_file(&subdir).await;
        assert!(remove_dir_as_file.is_err());

        // Can remove a symlink with remove_file (without affecting target)
        let target = dir.join("target.txt");
        fs::write(&target, b"keep me").await.unwrap();
        let sym = dir.join("sym.txt");
        symlink(&target, &sym).await.unwrap();

        fs::remove_file(&sym).await.unwrap();
        assert!(fs::metadata(&sym).await.is_err());

        // Target should still exist
        let target_content = fs::read_to_string(&target).await.unwrap();
        assert_eq!(target_content, "keep me");

        // Remove an empty file
        let empty_file = dir.join("empty.txt");
        fs::write(&empty_file, b"").await.unwrap();
        let empty_meta = fs::metadata(&empty_file).await.unwrap();
        assert_eq!(empty_meta.len(), 0);
        assert!(empty_meta.is_file());

        fs::remove_file(&empty_file).await.unwrap();
        assert!(fs::metadata(&empty_file).await.is_err());

        // Cleanup
        fs::remove_file(&target).await.unwrap();
        fs::remove_dir(&subdir).await.unwrap();
        fs::remove_dir(&dir).await.unwrap();
    });
}

#[test]
fn test_create_dir_all_idempotent() {
    task::block_on(async {
        let dir = unique_dir("create_dir_all_idem");
        let deep = dir.join("x").join("y").join("z");

        // First call creates everything
        fs::create_dir_all(&deep).await.unwrap();
        assert!(fs::metadata(&deep).await.unwrap().is_dir());

        // Second call should succeed (idempotent)
        let second = fs::create_dir_all(&deep).await;
        assert!(second.is_ok());

        // Intermediate directories exist
        assert!(fs::metadata(dir.join("x")).await.unwrap().is_dir());
        assert!(fs::metadata(dir.join("x").join("y")).await.unwrap().is_dir());

        // Put a file in the deepest dir
        let file = deep.join("data.bin");
        fs::write(&file, vec![0u8; 1024]).await.unwrap();
        let meta = fs::metadata(&file).await.unwrap();
        assert_eq!(meta.len(), 1024);
        assert!(meta.is_file());

        // Cleanup
        fs::remove_dir_all(&dir).await.unwrap();
        assert!(fs::metadata(&dir).await.is_err());
    });
}

#[test]
fn test_remove_dir_non_empty_fails() {
    task::block_on(async {
        let dir = unique_dir("remove_dir_nonempty");
        fs::create_dir(&dir).await.unwrap();

        let file = dir.join("blocker.txt");
        fs::write(&file, b"I block removal").await.unwrap();

        // remove_dir on non-empty directory should fail
        let result = fs::remove_dir(&dir).await;
        assert!(result.is_err());

        // Directory still exists
        assert!(fs::metadata(&dir).await.unwrap().is_dir());

        // File still exists
        let content = fs::read_to_string(&file).await.unwrap();
        assert_eq!(content, "I block removal");

        // remove_dir_all should succeed
        fs::remove_dir_all(&dir).await.unwrap();
        assert!(fs::metadata(&dir).await.is_err());
    });
}