use git2::{IndexAddOption, Repository, Signature};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_path(tag: &str) -> PathBuf {
    let pid = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("git2_tgt_{}_{}_{}", tag, pid, n));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn init_repo(path: &Path) -> Repository {
    let repo = Repository::init(path).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test").unwrap();
    cfg.set_str("user.email", "test@example.com").unwrap();
    drop(cfg);
    repo
}

fn commit_file(repo: &Repository, name: &str, content: &str, msg: &str) -> git2::Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    let full = workdir.join(name);
    if let Some(parent) = full.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&full, content).unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test", "test@example.com").unwrap();
    if let Some(oid) = repo.head().ok().and_then(|h| h.target()) {
        let parent = repo.find_commit(oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent])
            .unwrap()
    } else {
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[]).unwrap()
    }
}

#[test]
fn test_is_worktree_and_from_odb() {
    let dir = temp_path("iwt");
    let repo = init_repo(&dir);
    let oid = commit_file(&repo, "a.txt", "hello\n", "initial");

    assert_eq!(repo.is_worktree(), false);
    assert_eq!(repo.is_bare(), false);
    assert_eq!(repo.is_empty().unwrap(), false);


    let odb = repo.odb().unwrap();
    let odb_repo = Repository::from_odb(odb).unwrap();




    assert_eq!(odb_repo.is_worktree(), false);


    let found = odb_repo.find_commit(oid).unwrap();
    assert_eq!(found.id(), oid);
    assert_eq!(found.message().unwrap(), "initial");


    assert_eq!(repo.head().unwrap().target().unwrap(), oid);
}

#[test]
fn test_open_from_env() {
    let dir = temp_path("env");
    let repo = init_repo(&dir);
    let oid = commit_file(&repo, "a.txt", "hello\n", "envmsg");
    let git_dir = dir.join(".git");
    drop(repo);

    let prev = std::env::var_os("GIT_DIR");
    std::env::set_var("GIT_DIR", &git_dir);

    let opened_result = Repository::open_from_env();


    match prev {
        Some(v) => std::env::set_var("GIT_DIR", v),
        None => std::env::remove_var("GIT_DIR"),
    }

    let opened = opened_result.unwrap();
    assert_eq!(opened.is_worktree(), false);
    assert_eq!(opened.is_bare(), false);

    let commit = opened.find_commit(oid).unwrap();
    assert_eq!(commit.id(), oid);
    assert_eq!(commit.message().unwrap(), "envmsg");

    let head_oid = opened.head().unwrap().target().unwrap();
    assert_eq!(head_oid, oid);

    let path_str = opened.path().to_string_lossy().to_string();
    assert!(path_str.contains(".git"));
    assert_ne!(path_str.len(), 0);
}

#[test]
fn test_set_workdir() {
    let dir = temp_path("swd");
    let repo = init_repo(&dir);
    commit_file(&repo, "a.txt", "orig\n", "c1");

    let orig_wd = repo.workdir().unwrap().to_path_buf();
    assert!(orig_wd.exists());
    let canon_orig = fs::canonicalize(&orig_wd).unwrap();
    let canon_dir = fs::canonicalize(&dir).unwrap();
    assert_eq!(canon_orig, canon_dir);


    let new_wd = temp_path("swd_new");
    repo.set_workdir(&new_wd, false).unwrap();

    let current_wd = repo.workdir().unwrap().to_path_buf();
    let canon_current = fs::canonicalize(&current_wd).unwrap();
    let canon_new = fs::canonicalize(&new_wd).unwrap();
    assert_eq!(canon_current, canon_new);
    assert_ne!(canon_current, canon_orig);


    let gitpath = repo.path().to_path_buf();
    assert!(gitpath.exists());
    assert!(gitpath.to_string_lossy().contains(".git"));


    repo.set_workdir(&dir, false).unwrap();
    let wd2 = fs::canonicalize(repo.workdir().unwrap()).unwrap();
    assert_eq!(wd2, canon_dir);
    assert_ne!(wd2, canon_new);
}

#[test]
fn test_namespace_operations() {
    let dir = temp_path("ns");
    let repo = init_repo(&dir);
    let oid = commit_file(&repo, "a.txt", "hi\n", "c1");

    let head_oid_before = repo.head().unwrap().target().unwrap();
    assert_eq!(head_oid_before, oid);


    repo.set_namespace("foo").unwrap();




    let head_in_ns = repo.head();


    let _ = head_in_ns;

    repo.remove_namespace().unwrap();
    let head_after_remove = repo.head().unwrap().target().unwrap();
    assert_eq!(head_after_remove, oid);


    repo.set_namespace_bytes(b"bar-bytes").unwrap();
    let head_in_bytes_ns = repo.head();
    let _ = head_in_bytes_ns;

    repo.remove_namespace().unwrap();
    let head_final = repo.head().unwrap().target().unwrap();
    assert_eq!(head_final, oid);
    assert_eq!(head_final, head_oid_before);


    let noop = repo.remove_namespace();
    assert!(noop.is_ok());
    assert_eq!(repo.is_worktree(), false);
}

#[test]
fn test_remove_message() {
    let dir = temp_path("msg");
    let repo = init_repo(&dir);
    commit_file(&repo, "a.txt", "hi\n", "c1");

    let msg_path = repo.path().join("MERGE_MSG");

    assert_eq!(msg_path.exists(), false);
    let msg_before = repo.message();
    assert!(msg_before.is_err());

    let body = "Merge branch 'feature'\n\nDetails go here.\n";
    fs::write(&msg_path, body).unwrap();
    assert_eq!(msg_path.exists(), true);

    let msg = repo.message().unwrap();
    assert_eq!(msg, body);
    assert_ne!(msg.len(), 0);

    repo.remove_message().unwrap();
    assert_eq!(msg_path.exists(), false);

    let msg_after = repo.message();
    assert!(msg_after.is_err());


    let second = repo.remove_message();
    assert!(second.is_err());
}

#[test]
fn test_remote_with_fetch_and_add_push() {
    let dir = temp_path("rmt");
    let repo = init_repo(&dir);
    commit_file(&repo, "a.txt", "hi\n", "c1");

    let fetch_spec = "+refs/heads/*:refs/remotes/custom/*";
    let url = "https://example.com/repo.git";

    let r = repo.remote_with_fetch("custom", url, fetch_spec).unwrap();
    assert_eq!(r.name().unwrap(), "custom");
    assert_eq!(r.url().unwrap(), url);

    let fetch_specs: Vec<String> = r
        .fetch_refspecs()
        .unwrap()
        .iter()
        .filter_map(|s| s.map(String::from))
        .collect();
    assert_eq!(fetch_specs.len(), 1);
    assert_eq!(fetch_specs[0], fetch_spec);

    let push_specs_empty: Vec<String> = r
        .push_refspecs()
        .unwrap()
        .iter()
        .filter_map(|s| s.map(String::from))
        .collect();
    assert_eq!(push_specs_empty.len(), 0);
    drop(r);

    repo.remote_add_push("custom", "refs/heads/main:refs/heads/main")
        .unwrap();

    let r2 = repo.find_remote("custom").unwrap();
    let push_one: Vec<String> = r2
        .push_refspecs()
        .unwrap()
        .iter()
        .filter_map(|s| s.map(String::from))
        .collect();
    assert_eq!(push_one.len(), 1);
    assert_eq!(push_one[0], "refs/heads/main:refs/heads/main");
    drop(r2);

    repo.remote_add_push("custom", "refs/tags/*:refs/tags/*")
        .unwrap();
    let r3 = repo.find_remote("custom").unwrap();
    let push_two: Vec<String> = r3
        .push_refspecs()
        .unwrap()
        .iter()
        .filter_map(|s| s.map(String::from))
        .collect();
    assert_eq!(push_two.len(), 2);
    assert!(push_two.iter().any(|s| s == "refs/heads/main:refs/heads/main"));
    assert!(push_two.iter().any(|s| s == "refs/tags/*:refs/tags/*"));
    assert_ne!(push_two[0], push_two[1]);
}

#[test]
fn test_reset_default() {
    let dir = temp_path("rd");
    let repo = init_repo(&dir);
    let c1_oid = commit_file(&repo, "a.txt", "original\n", "c1");
    let head_commit = repo.find_commit(c1_oid).unwrap();


    fs::write(dir.join("a.txt"), "modified\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("a.txt")).unwrap();
    index.write().unwrap();

    let staged = index.get_path(Path::new("a.txt"), 0).unwrap();
    let staged_blob = repo.find_blob(staged.id).unwrap();
    assert_eq!(std::str::from_utf8(staged_blob.content()).unwrap(), "modified\n");
    let staged_id = staged.id;
    drop(index);


    repo.reset_default(Some(head_commit.as_object()), ["a.txt"].iter())
        .unwrap();

    let index2 = repo.index().unwrap();
    let after = index2.get_path(Path::new("a.txt"), 0).unwrap();
    let after_blob = repo.find_blob(after.id).unwrap();
    assert_eq!(std::str::from_utf8(after_blob.content()).unwrap(), "original\n");
    assert_ne!(after.id, staged_id);


    let wt = fs::read_to_string(dir.join("a.txt")).unwrap();
    assert_eq!(wt, "modified\n");


    assert_eq!(repo.head().unwrap().target().unwrap(), c1_oid);


    fs::write(dir.join("a.txt"), "second-modification\n").unwrap();
    let mut index3 = repo.index().unwrap();
    index3.add_path(Path::new("a.txt")).unwrap();
    index3.write().unwrap();
    let staged2 = index3.get_path(Path::new("a.txt"), 0).unwrap().id;
    assert_ne!(staged2, after.id);
    drop(index3);

    repo.reset_default(Some(head_commit.as_object()), ["a.txt"].iter())
        .unwrap();
    let index4 = repo.index().unwrap();
    let final_entry = index4.get_path(Path::new("a.txt"), 0).unwrap();
    assert_eq!(final_entry.id, after.id);
}

#[test]
fn test_head_detached_and_set_from_annotated() {
    let dir = temp_path("det");
    let repo = init_repo(&dir);
    let c1 = commit_file(&repo, "a.txt", "v1\n", "c1");
    let c2 = commit_file(&repo, "a.txt", "v2\n", "c2");
    assert_ne!(c1, c2);

    assert_eq!(repo.head_detached().unwrap(), false);
    assert_eq!(repo.head().unwrap().target().unwrap(), c2);

    let annotated = repo.find_annotated_commit(c1).unwrap();
    assert_eq!(annotated.id(), c1);

    repo.set_head_detached_from_annotated(annotated).unwrap();

    assert_eq!(repo.head_detached().unwrap(), true);
    let head_after = repo.head().unwrap();
    assert_eq!(head_after.target().unwrap(), c1);
    assert_ne!(head_after.target().unwrap(), c2);


    assert_eq!(repo.is_worktree(), false);
}

#[test]
fn test_set_index() {
    let dir = temp_path("sidx");
    let repo = init_repo(&dir);
    commit_file(&repo, "a.txt", "hi\n", "c1");
    commit_file(&repo, "b.txt", "bye\n", "c2");

    let idx_before = repo.index().unwrap();
    let count_before = idx_before.len();
    assert!(count_before >= 2);
    drop(idx_before);


    let mut new_index = git2::Index::new().unwrap();
    assert_eq!(new_index.len(), 0);

    repo.set_index(&mut new_index).unwrap();

    let idx_after = repo.index().unwrap();
    assert_eq!(idx_after.len(), 0);
    assert_ne!(idx_after.len(), count_before);


    let head = repo.head().unwrap();
    assert!(head.target().is_some());
}

#[test]
fn test_clone_recurse_local() {
    let src = temp_path("clrec_src");
    let src_repo = init_repo(&src);
    commit_file(&src_repo, "README.md", "hello\n", "c1");
    commit_file(&src_repo, "f.txt", "body\n", "c2");
    let src_head = src_repo.head().unwrap().target().unwrap();
    drop(src_repo);

    let dst = temp_path("clrec_dst");

    let _ = fs::remove_dir_all(&dst);

    let url = src.to_string_lossy().to_string();
    let cloned = Repository::clone_recurse(&url, &dst).unwrap();

    assert_eq!(cloned.is_bare(), false);
    assert_eq!(cloned.is_worktree(), false);
    assert_eq!(cloned.head_detached().unwrap(), false);

    let cloned_head = cloned.head().unwrap().target().unwrap();
    assert_eq!(cloned_head, src_head);

    let readme = dst.join("README.md");
    assert_eq!(readme.exists(), true);
    let content = fs::read_to_string(&readme).unwrap();
    assert_eq!(content, "hello\n");

    let f = dst.join("f.txt");
    assert_eq!(f.exists(), true);
    let fcontent = fs::read_to_string(&f).unwrap();
    assert_eq!(fcontent, "body\n");

    let origin = cloned.find_remote("origin").unwrap();
    assert_eq!(origin.name().unwrap(), "origin");
    assert!(origin.url().unwrap().contains("clrec_src"));
}