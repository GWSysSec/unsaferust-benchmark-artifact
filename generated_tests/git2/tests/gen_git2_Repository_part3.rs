use git2::*;
use std::fs;
use std::path::{Path, PathBuf};

fn unique_tmp(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!(
        "git2-test-{}-{}-{}",
        name,
        std::process::id(),
        nanos
    ));
    p
}

fn make_repo(name: &str) -> (PathBuf, Repository) {
    let path = unique_tmp(name);
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();
    }
    (path, repo)
}

fn add_and_commit(
    path: &Path,
    repo: &Repository,
    file: &str,
    content: &[u8],
    msg: &str,
    parents: &[&Commit],
) -> Oid {
    fs::write(path.join(file), content).unwrap();
    let mut idx = repo.index().unwrap();
    idx.add_path(Path::new(file)).unwrap();
    idx.write().unwrap();
    let tree_id = idx.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, parents)
        .unwrap()
}

#[test]
fn test_cleanup_state() {
    let (path, repo) = make_repo("cleanup");
    let oid = add_and_commit(&path, &repo, "a.txt", b"x\n", "init", &[]);

    let r1 = repo.cleanup_state();
    assert!(r1.is_ok());
    let r2 = repo.cleanup_state();
    assert!(r2.is_ok());
    assert_eq!(repo.state(), RepositoryState::Clean);

    let head = repo.head().unwrap();
    assert!(head.target().is_some());
    assert_eq!(head.target(), Some(oid));
    assert!(head.is_branch());
    assert!(head.name().unwrap().starts_with("refs/heads/"));
    assert!(head.shorthand().is_some());

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_branch_upstream_and_remote_names() {
    let (path, repo) = make_repo("brnames");
    let oid = add_and_commit(&path, &repo, "a.txt", b"x\n", "init", &[]);

    let head_name = repo.head().unwrap().name().unwrap().to_string();
    let short = head_name
        .trim_start_matches("refs/heads/")
        .to_string();

    repo.remote("origin", "https://example.com/foo.git").unwrap();
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str(&format!("branch.{}.remote", short), "origin")
            .unwrap();
        cfg.set_str(&format!("branch.{}.merge", short), &head_name)
            .unwrap();
    }
    let remote_ref = format!("refs/remotes/origin/{}", short);
    repo.reference(&remote_ref, oid, true, "rt").unwrap();

    let upstream_buf = repo.branch_upstream_name(&head_name).unwrap();
    let upstream_str = std::str::from_utf8(&upstream_buf).unwrap();
    assert_eq!(upstream_str, remote_ref);

    let upstream_remote_buf = repo.branch_upstream_remote(&head_name).unwrap();
    let upstream_remote_str = std::str::from_utf8(&upstream_remote_buf).unwrap();
    assert_eq!(upstream_remote_str, "origin");

    let bn_buf = repo.branch_remote_name(&remote_ref).unwrap();
    let bn_str = std::str::from_utf8(&bn_buf).unwrap();
    assert_eq!(bn_str, "origin");

    let bad1 = repo.branch_upstream_name("refs/heads/nonexistent");
    assert!(bad1.is_err());

    let bad2 = repo.branch_remote_name("refs/heads/local-only");
    assert!(bad2.is_err());

    let bad3 = repo.branch_upstream_remote("refs/heads/nonexistent");
    assert!(bad3.is_err());

    assert!(!upstream_str.is_empty());
    assert!(!upstream_remote_str.is_empty());

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_merge_analysis() {
    let (path, repo) = make_repo("ma");
    let oid1 = add_and_commit(&path, &repo, "a.txt", b"v1\n", "c1", &[]);
    let c1 = repo.find_commit(oid1).unwrap();
    let head_name = repo.head().unwrap().name().unwrap().to_string();


    let ac1 = repo.find_annotated_commit(oid1).unwrap();
    let (a, _p) = repo.merge_analysis(&[&ac1]).unwrap();
    assert!(a.contains(MergeAnalysis::ANALYSIS_UP_TO_DATE));
    assert!(!a.contains(MergeAnalysis::ANALYSIS_UNBORN));


    let oid2 = add_and_commit(&path, &repo, "a.txt", b"v2\n", "c2", &[&c1]);
    assert_ne!(oid1, oid2);


    let ac1b = repo.find_annotated_commit(oid1).unwrap();
    let (a2, _) = repo.merge_analysis(&[&ac1b]).unwrap();
    assert!(a2.contains(MergeAnalysis::ANALYSIS_UP_TO_DATE));


    repo.reference(&head_name, oid1, true, "reset").unwrap();
    let ac2 = repo.find_annotated_commit(oid2).unwrap();
    let (a3, _) = repo.merge_analysis(&[&ac2]).unwrap();
    assert!(a3.contains(MergeAnalysis::ANALYSIS_FASTFORWARD));
    assert!(a3.contains(MergeAnalysis::ANALYSIS_NORMAL));
    assert!(!a3.contains(MergeAnalysis::ANALYSIS_UP_TO_DATE));

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_merge_trees() {
    let (path, repo) = make_repo("mt");
    let odb = repo.odb().unwrap();

    let base_blob = odb.write(ObjectType::Blob, b"base\n").unwrap();
    let our_blob = odb.write(ObjectType::Blob, b"ours\n").unwrap();
    let their_blob = odb.write(ObjectType::Blob, b"theirs\n").unwrap();

    let mut tb_b = repo.treebuilder(None).unwrap();
    tb_b.insert("base.txt", base_blob, 0o100644).unwrap();
    let base_tree_id = tb_b.write().unwrap();
    let base_tree = repo.find_tree(base_tree_id).unwrap();

    let mut tb_o = repo.treebuilder(None).unwrap();
    tb_o.insert("base.txt", base_blob, 0o100644).unwrap();
    tb_o.insert("ours.txt", our_blob, 0o100644).unwrap();
    let our_tree_id = tb_o.write().unwrap();
    let our_tree = repo.find_tree(our_tree_id).unwrap();

    let mut tb_t = repo.treebuilder(None).unwrap();
    tb_t.insert("base.txt", base_blob, 0o100644).unwrap();
    tb_t.insert("theirs.txt", their_blob, 0o100644).unwrap();
    let their_tree_id = tb_t.write().unwrap();
    let their_tree = repo.find_tree(their_tree_id).unwrap();

    let merged = repo
        .merge_trees(&base_tree, &our_tree, &their_tree, None)
        .unwrap();
    assert!(!merged.has_conflicts());

    let entries: Vec<_> = merged.iter().collect();
    let names: Vec<String> = entries
        .iter()
        .map(|e| String::from_utf8_lossy(&e.path).to_string())
        .collect();
    assert_eq!(entries.len(), 3);
    assert!(names.contains(&"base.txt".to_string()));
    assert!(names.contains(&"ours.txt".to_string()));
    assert!(names.contains(&"theirs.txt".to_string()));
    assert_ne!(base_tree_id, our_tree_id);
    assert_ne!(base_tree_id, their_tree_id);
    assert_ne!(our_tree_id, their_tree_id);

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_cherrypick_commit() {
    let (path, repo) = make_repo("cp");
    let oid1 = add_and_commit(&path, &repo, "a.txt", b"1\n", "c1", &[]);
    let c1 = repo.find_commit(oid1).unwrap();
    let oid2 = add_and_commit(&path, &repo, "b.txt", b"2\n", "c2", &[&c1]);
    let c2 = repo.find_commit(oid2).unwrap();

    let our = repo.find_commit(oid1).unwrap();
    let mut idx = repo.cherrypick_commit(&c2, &our, 0, None).unwrap();
    assert!(!idx.has_conflicts());

    let entries: Vec<_> = idx.iter().collect();
    let names: Vec<String> = entries
        .iter()
        .map(|e| String::from_utf8_lossy(&e.path).to_string())
        .collect();
    assert_eq!(entries.len(), 2);
    assert!(names.contains(&"a.txt".to_string()));
    assert!(names.contains(&"b.txt".to_string()));
    assert_ne!(oid1, oid2);

    let tree_id = idx.write_tree_to(&repo).unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    assert_eq!(tree.len(), 2);
    assert!(tree.get_name("a.txt").is_some());
    assert!(tree.get_name("b.txt").is_some());

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_diff_index_to_index() {
    let (path, repo) = make_repo("d2i");
    let oid1 = add_and_commit(&path, &repo, "a.txt", b"hello\n", "c1", &[]);
    let c1 = repo.find_commit(oid1).unwrap();
    let t1 = c1.tree().unwrap();

    let oid2 = add_and_commit(&path, &repo, "a.txt", b"hello world\n", "c2", &[&c1]);
    let c2 = repo.find_commit(oid2).unwrap();
    let t2 = c2.tree().unwrap();

    let mut idx1 = Index::new().unwrap();
    idx1.read_tree(&t1).unwrap();
    let mut idx2 = Index::new().unwrap();
    idx2.read_tree(&t2).unwrap();

    let diff = repo.diff_index_to_index(&idx1, &idx2, None).unwrap();
    assert_eq!(diff.deltas().len(), 1);
    let stats = diff.stats().unwrap();
    assert_eq!(stats.files_changed(), 1);
    assert!(stats.insertions() >= 1);

    let diff_same = repo.diff_index_to_index(&idx1, &idx1, None).unwrap();
    assert_eq!(diff_same.deltas().len(), 0);
    let s_same = diff_same.stats().unwrap();
    assert_eq!(s_same.files_changed(), 0);
    assert_eq!(s_same.insertions(), 0);
    assert_eq!(s_same.deletions(), 0);
    assert_ne!(oid1, oid2);

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_diff_tree_to_workdir_with_index() {
    let (path, repo) = make_repo("dtw");
    let oid1 = add_and_commit(&path, &repo, "a.txt", b"v1\n", "c1", &[]);
    let c1 = repo.find_commit(oid1).unwrap();
    let t1 = c1.tree().unwrap();

    fs::write(path.join("a.txt"), b"v2\n").unwrap();

    let diff = repo
        .diff_tree_to_workdir_with_index(Some(&t1), None)
        .unwrap();
    assert!(diff.deltas().len() >= 1);
    let s = diff.stats().unwrap();
    assert_eq!(s.files_changed(), 1);

    let diff_full = repo
        .diff_tree_to_workdir_with_index(None, None)
        .unwrap();
    assert!(diff_full.deltas().len() >= 1);

    fs::write(path.join("a.txt"), b"v1\n").unwrap();
    let diff_clean = repo
        .diff_tree_to_workdir_with_index(Some(&t1), None)
        .unwrap();
    let cs = diff_clean.stats().unwrap();
    assert_eq!(cs.files_changed(), 0);
    assert_eq!(cs.insertions(), 0);
    assert_eq!(cs.deletions(), 0);
    assert_eq!(diff_clean.deltas().len(), 0);
    assert_ne!(oid1, Oid::zero());

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_diff_blobs_no_callbacks() {
    let (path, repo) = make_repo("db");
    let odb = repo.odb().unwrap();
    let old_oid = odb.write(ObjectType::Blob, b"a\nb\n").unwrap();
    let new_oid = odb.write(ObjectType::Blob, b"a\nB\nc\n").unwrap();
    assert_ne!(old_oid, new_oid);

    let old_blob = repo.find_blob(old_oid).unwrap();
    let new_blob = repo.find_blob(new_oid).unwrap();
    assert_eq!(old_blob.id(), old_oid);
    assert_eq!(new_blob.id(), new_oid);

    let r1 = repo.diff_blobs(
        Some(&old_blob),
        Some("file.txt"),
        Some(&new_blob),
        Some("file.txt"),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(r1.is_ok());

    let r2 = repo.diff_blobs(
        None,
        None,
        Some(&new_blob),
        Some("file.txt"),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(r2.is_ok());

    let r3 = repo.diff_blobs(
        Some(&old_blob),
        Some("file.txt"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert!(r3.is_ok());

    let r4 = repo.diff_blobs(
        Some(&old_blob),
        Some("file.txt"),
        Some(&old_blob),
        Some("file.txt"),
        None,
        None,
        None,
        None,
        None,
    );
    assert!(r4.is_ok());

    assert_eq!(old_blob.is_binary(), false);

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_stash_foreach_empty() {
    let (path, mut repo) = make_repo("stash");
    let oid = add_and_commit(&path, &repo, "a.txt", b"x\n", "init", &[]);

    let mut count: u32 = 0;
    let mut entries: Vec<(Oid, String)> = Vec::new();
    repo.stash_foreach(|_index, msg, target| {
        count += 1;
        entries.push((*target, msg.to_string()));
        true
    })
    .unwrap();

    assert_eq!(count, 0);
    assert_eq!(entries.len(), 0);
    assert!(entries.is_empty());
    assert_eq!(count as usize, entries.len());

    let head = repo.head().unwrap();
    assert_eq!(head.target(), Some(oid));
    assert!(head.is_branch());
    assert!(head.name().is_some());
    assert!(head.name().unwrap().starts_with("refs/heads/"));

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_note_delete() {
    let (path, repo) = make_repo("note");
    let oid = add_and_commit(&path, &repo, "a.txt", b"x\n", "c1", &[]);
    let sig = Signature::now("Test", "test@example.com").unwrap();

    let note_oid = repo
        .note(&sig, &sig, None, oid, "my note", false)
        .unwrap();
    assert_ne!(note_oid, Oid::zero());

    let note = repo.find_note(None, oid).unwrap();
    assert_eq!(note.message(), Some("my note"));
    drop(note);

    let r = repo.note_delete(oid, None, &sig, &sig);
    assert!(r.is_ok());

    let after = repo.find_note(None, oid);
    assert!(after.is_err());

    let again = repo.note_delete(oid, None, &sig, &sig);
    assert!(again.is_err());

    let custom_ref = "refs/notes/custom";
    let note_oid2 = repo
        .note(&sig, &sig, Some(custom_ref), oid, "custom note", false)
        .unwrap();
    assert_ne!(note_oid2, Oid::zero());

    let r2 = repo.note_delete(oid, Some(custom_ref), &sig, &sig);
    assert!(r2.is_ok());

    let after2 = repo.find_note(Some(custom_ref), oid);
    assert!(after2.is_err());

    assert_ne!(note_oid, note_oid2);

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_open_rebase_no_active() {
    let (path, repo) = make_repo("rebase");
    let oid = add_and_commit(&path, &repo, "a.txt", b"x\n", "c1", &[]);

    let r1 = repo.open_rebase(None);
    assert!(r1.is_err());

    let r2 = repo.open_rebase(None);
    assert!(r2.is_err());

    assert_eq!(repo.state(), RepositoryState::Clean);
    let head = repo.head().unwrap();
    assert_eq!(head.target(), Some(oid));
    assert!(head.is_branch());
    assert!(head.name().unwrap().starts_with("refs/heads/"));
    assert!(head.shorthand().is_some());
    assert_ne!(head.target(), Some(Oid::zero()));

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_checkout_tree() {
    let (path, repo) = make_repo("co");
    let oid1 = add_and_commit(&path, &repo, "a.txt", b"v1\n", "c1", &[]);
    let c1 = repo.find_commit(oid1).unwrap();
    let oid2 = add_and_commit(&path, &repo, "a.txt", b"v2\n", "c2", &[&c1]);

    let target1 = repo.find_object(oid1, None).unwrap();
    let mut co = build::CheckoutBuilder::new();
    co.force();
    let r1 = repo.checkout_tree(&target1, Some(&mut co));
    assert!(r1.is_ok());

    let content_after1 = fs::read_to_string(path.join("a.txt")).unwrap();
    assert_eq!(content_after1, "v1\n");
    assert_ne!(content_after1, "v2\n");

    let target2 = repo.find_object(oid2, None).unwrap();
    let mut co2 = build::CheckoutBuilder::new();
    co2.force();
    let r2 = repo.checkout_tree(&target2, Some(&mut co2));
    assert!(r2.is_ok());
    let content_after2 = fs::read_to_string(path.join("a.txt")).unwrap();
    assert_eq!(content_after2, "v2\n");

    let mut co3 = build::CheckoutBuilder::new();
    co3.force();
    let r3 = repo.checkout_tree(&target1, Some(&mut co3));
    assert!(r3.is_ok());

    assert_ne!(oid1, oid2);
    assert_ne!(oid1, Oid::zero());
    assert_ne!(oid2, Oid::zero());

    let _ = fs::remove_dir_all(&path);
}

#[test]
fn test_merge_basic_then_cleanup() {
    let (path, repo) = make_repo("merge");
    let oid_base = add_and_commit(&path, &repo, "base.txt", b"b\n", "base", &[]);
    let base = repo.find_commit(oid_base).unwrap();
    let oid_our = add_and_commit(&path, &repo, "ours.txt", b"ours\n", "ours", &[&base]);


    let odb = repo.odb().unwrap();
    let base_blob = odb.write(ObjectType::Blob, b"b\n").unwrap();
    let their_blob = odb.write(ObjectType::Blob, b"theirs\n").unwrap();
    let mut tb = repo.treebuilder(None).unwrap();
    tb.insert("base.txt", base_blob, 0o100644).unwrap();
    tb.insert("theirs.txt", their_blob, 0o100644).unwrap();
    let their_tree_id = tb.write().unwrap();
    let their_tree = repo.find_tree(their_tree_id).unwrap();
    let sig = Signature::now("Test", "test@example.com").unwrap();
    let oid_their = repo
        .commit(None, &sig, &sig, "their", &their_tree, &[&base])
        .unwrap();

    let ac_their = repo.find_annotated_commit(oid_their).unwrap();
    let mut co_opts = build::CheckoutBuilder::new();
    co_opts.force();
    let r = repo.merge(&[&ac_their], None, Some(&mut co_opts));
    assert!(r.is_ok());

    assert_eq!(repo.state(), RepositoryState::Merge);
    assert!(path.join("base.txt").exists());
    assert!(path.join("ours.txt").exists());
    assert!(path.join("theirs.txt").exists());

    let cleanup = repo.cleanup_state();
    assert!(cleanup.is_ok());
    assert_eq!(repo.state(), RepositoryState::Clean);

    assert_ne!(oid_our, oid_their);
    assert_ne!(oid_base, oid_their);
    assert_ne!(oid_base, oid_our);

    let _ = fs::remove_dir_all(&path);
}