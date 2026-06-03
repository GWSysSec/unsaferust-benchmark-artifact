use git2::{
    Commit, FileFavor, IndexAddOption, MergeOptions, Oid, Repository, ResetType, Signature,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_path(tag: &str) -> PathBuf {
    let pid = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("git2_mo_{}_{}_{}", tag, pid, n));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn init_repo(path: &Path) -> Repository {
    let repo = Repository::init(path).unwrap();
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();
    }
    repo
}

fn write_and_commit(
    repo: &Repository,
    files: &[(&str, &str)],
    msg: &str,
    parents: &[&Commit<'_>],
) -> Oid {
    let workdir = repo.workdir().unwrap().to_path_buf();
    for (name, content) in files {
        let full = workdir.join(name);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full, content).unwrap();
    }
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, parents)
        .unwrap()
}

fn read_index_blob(
    repo: &Repository,
    index: &git2::Index,
    path: &str,
    stage: i32,
) -> Option<String> {
    let entry = index.get_path(Path::new(path), stage)?;
    let blob = repo.find_blob(entry.id).ok()?;
    Some(String::from_utf8_lossy(blob.content()).into_owned())
}

#[test]
fn test_merge_options_file_favor_ours_theirs_union() {
    let dir = temp_path("favor");
    let repo = init_repo(&dir);

    let base_oid = write_and_commit(&repo, &[("file.txt", "A\nB\nC\n")], "base", &[]);
    let base = repo.find_commit(base_oid).unwrap();

    let ours_oid = write_and_commit(
        &repo,
        &[("file.txt", "OURS\nB\nC\n")],
        "ours",
        &[&base],
    );
    let ours = repo.find_commit(ours_oid).unwrap();

    repo.reset(base.as_object(), ResetType::Hard, None).unwrap();

    let theirs_oid = write_and_commit(
        &repo,
        &[("file.txt", "THEIRS\nB\nC\n")],
        "theirs",
        &[&base],
    );
    let theirs = repo.find_commit(theirs_oid).unwrap();


    let default_idx = repo.merge_commits(&ours, &theirs, None).unwrap();
    assert_eq!(default_idx.has_conflicts(), true);


    let mut opts_ours = MergeOptions::new();
    opts_ours.file_favor(FileFavor::Ours);
    let idx_ours = repo
        .merge_commits(&ours, &theirs, Some(&opts_ours))
        .unwrap();
    assert_eq!(idx_ours.has_conflicts(), false);
    let merged_ours = read_index_blob(&repo, &idx_ours, "file.txt", 0).unwrap();
    assert_eq!(merged_ours, "OURS\nB\nC\n");


    let mut opts_theirs = MergeOptions::new();
    opts_theirs.file_favor(FileFavor::Theirs);
    let idx_theirs = repo
        .merge_commits(&ours, &theirs, Some(&opts_theirs))
        .unwrap();
    assert_eq!(idx_theirs.has_conflicts(), false);
    let merged_theirs = read_index_blob(&repo, &idx_theirs, "file.txt", 0).unwrap();
    assert_eq!(merged_theirs, "THEIRS\nB\nC\n");
    assert_ne!(merged_ours, merged_theirs);


    let mut opts_union = MergeOptions::new();
    opts_union.file_favor(FileFavor::Union);
    let idx_union = repo
        .merge_commits(&ours, &theirs, Some(&opts_union))
        .unwrap();
    assert_eq!(idx_union.has_conflicts(), false);
    let merged_union = read_index_blob(&repo, &idx_union, "file.txt", 0).unwrap();
    assert!(merged_union.contains("OURS"));
    assert!(merged_union.contains("THEIRS"));
    assert!(merged_union.contains('B'));
}

#[test]
fn test_merge_options_fail_on_conflict() {
    let dir = temp_path("fail");
    let repo = init_repo(&dir);

    let base_oid = write_and_commit(&repo, &[("a.txt", "base\n")], "base", &[]);
    let base = repo.find_commit(base_oid).unwrap();

    let ours_oid = write_and_commit(&repo, &[("a.txt", "ours\n")], "o", &[&base]);
    let ours = repo.find_commit(ours_oid).unwrap();

    repo.reset(base.as_object(), ResetType::Hard, None).unwrap();

    let theirs_oid = write_and_commit(&repo, &[("a.txt", "theirs\n")], "t", &[&base]);
    let theirs = repo.find_commit(theirs_oid).unwrap();


    let normal_idx = repo.merge_commits(&ours, &theirs, None).unwrap();
    assert_eq!(normal_idx.has_conflicts(), true);
    let s1 = normal_idx.get_path(Path::new("a.txt"), 1);
    let s2 = normal_idx.get_path(Path::new("a.txt"), 2);
    let s3 = normal_idx.get_path(Path::new("a.txt"), 3);
    assert!(s1.is_some());
    assert!(s2.is_some());
    assert!(s3.is_some());
    assert_ne!(s2.as_ref().unwrap().id, s3.as_ref().unwrap().id);


    let mut opts_no_fail = MergeOptions::new();
    opts_no_fail.fail_on_conflict(false);
    let idx_no_fail = repo
        .merge_commits(&ours, &theirs, Some(&opts_no_fail))
        .unwrap();
    assert_eq!(idx_no_fail.has_conflicts(), true);


    let mut opts_fail = MergeOptions::new();
    opts_fail.fail_on_conflict(true);
    let result = repo.merge_commits(&ours, &theirs, Some(&opts_fail));
    assert!(result.is_err());
    let err = result.err().unwrap();
    let msg = err.message().to_string();
    assert!(!msg.is_empty());
}

#[test]
fn test_merge_options_conflict_styles_and_stages() {
    let dir = temp_path("style");
    let repo = init_repo(&dir);

    let base_oid = write_and_commit(
        &repo,
        &[("f.txt", "common1\noriginal\ncommon2\n")],
        "base",
        &[],
    );
    let base = repo.find_commit(base_oid).unwrap();

    let ours_oid = write_and_commit(
        &repo,
        &[("f.txt", "common1\nours_line\ncommon2\n")],
        "ours",
        &[&base],
    );
    let ours = repo.find_commit(ours_oid).unwrap();

    repo.reset(base.as_object(), ResetType::Hard, None).unwrap();

    let theirs_oid = write_and_commit(
        &repo,
        &[("f.txt", "common1\ntheirs_line\ncommon2\n")],
        "theirs",
        &[&base],
    );
    let theirs = repo.find_commit(theirs_oid).unwrap();


    let mut opts_std = MergeOptions::new();
    opts_std.standard_style(true).patience(false);
    let idx_std = repo
        .merge_commits(&ours, &theirs, Some(&opts_std))
        .unwrap();
    assert_eq!(idx_std.has_conflicts(), true);
    let s1_std = idx_std.get_path(Path::new("f.txt"), 1);
    let s2_std = idx_std.get_path(Path::new("f.txt"), 2);
    let s3_std = idx_std.get_path(Path::new("f.txt"), 3);
    assert!(s1_std.is_some());
    assert!(s2_std.is_some());
    assert!(s3_std.is_some());
    assert_ne!(s1_std.as_ref().unwrap().id, s2_std.as_ref().unwrap().id);
    assert_ne!(s2_std.as_ref().unwrap().id, s3_std.as_ref().unwrap().id);
    assert_ne!(s1_std.as_ref().unwrap().id, s3_std.as_ref().unwrap().id);


    let mut opts_d3 = MergeOptions::new();
    opts_d3.diff3_style(true);
    let idx_d3 = repo
        .merge_commits(&ours, &theirs, Some(&opts_d3))
        .unwrap();
    assert_eq!(idx_d3.has_conflicts(), true);
    let s1_d3 = idx_d3.get_path(Path::new("f.txt"), 1).unwrap();
    let s2_d3 = idx_d3.get_path(Path::new("f.txt"), 2).unwrap();
    let s3_d3 = idx_d3.get_path(Path::new("f.txt"), 3).unwrap();


    let base_content = read_index_blob(&repo, &idx_d3, "f.txt", 1).unwrap();
    let ours_content = read_index_blob(&repo, &idx_d3, "f.txt", 2).unwrap();
    let theirs_content = read_index_blob(&repo, &idx_d3, "f.txt", 3).unwrap();
    assert_eq!(base_content, "common1\noriginal\ncommon2\n");
    assert_eq!(ours_content, "common1\nours_line\ncommon2\n");
    assert_eq!(theirs_content, "common1\ntheirs_line\ncommon2\n");


    assert_ne!(s1_d3.id, s2_d3.id);
    assert_ne!(s2_d3.id, s3_d3.id);
}

#[test]
fn test_merge_options_all_builder_methods_chain() {

    let mut opts = MergeOptions::new();
    let reference: &mut MergeOptions = opts
        .find_renames(true)
        .fail_on_conflict(false)
        .skip_reuc(true)
        .no_recursive(false)
        .rename_threshold(50)
        .target_limit(200)
        .recursion_limit(10)
        .file_favor(FileFavor::Normal)
        .standard_style(true)
        .diff3_style(false)
        .simplify_alnum(true)
        .ignore_whitespace(true)
        .ignore_whitespace_change(false)
        .ignore_whitespace_eol(true)
        .patience(true);

    reference.rename_threshold(75);



    let dir = temp_path("allopts");
    let repo = init_repo(&dir);

    let base_oid = write_and_commit(
        &repo,
        &[("f.txt", "alpha\nbeta\ngamma\n")],
        "base",
        &[],
    );
    let base = repo.find_commit(base_oid).unwrap();

    let a_oid = write_and_commit(
        &repo,
        &[("f.txt", "alpha\nBETA_CHANGED\ngamma\n")],
        "a",
        &[&base],
    );
    let a = repo.find_commit(a_oid).unwrap();

    repo.reset(base.as_object(), ResetType::Hard, None).unwrap();

    let b_oid = write_and_commit(
        &repo,
        &[("g.txt", "added_on_b\n")],
        "b",
        &[&base],
    );
    let b = repo.find_commit(b_oid).unwrap();

    let mut opts2 = MergeOptions::new();
    opts2
        .find_renames(true)
        .rename_threshold(50)
        .target_limit(1000)
        .recursion_limit(5)
        .no_recursive(false)
        .skip_reuc(true)
        .simplify_alnum(false)
        .ignore_whitespace(true)
        .ignore_whitespace_change(true)
        .ignore_whitespace_eol(true)
        .patience(true)
        .standard_style(true)
        .diff3_style(false)
        .file_favor(FileFavor::Normal)
        .fail_on_conflict(false);

    let idx = repo.merge_commits(&a, &b, Some(&opts2)).unwrap();
    assert_eq!(idx.has_conflicts(), false);
    let f_entry = idx.get_path(Path::new("f.txt"), 0);
    let g_entry = idx.get_path(Path::new("g.txt"), 0);
    assert!(f_entry.is_some());
    assert!(g_entry.is_some());

    let f_blob = read_index_blob(&repo, &idx, "f.txt", 0).unwrap();
    let g_blob = read_index_blob(&repo, &idx, "g.txt", 0).unwrap();
    assert_eq!(f_blob, "alpha\nBETA_CHANGED\ngamma\n");
    assert_eq!(g_blob, "added_on_b\n");
    assert_ne!(f_blob, g_blob);
    assert!(f_blob.contains("BETA_CHANGED"));
    assert!(g_blob.starts_with("added_on_b"));


    assert_eq!(f_entry.as_ref().unwrap().id.is_zero(), false);
    assert_eq!(g_entry.as_ref().unwrap().id.is_zero(), false);
    assert_ne!(f_entry.as_ref().unwrap().id, g_entry.as_ref().unwrap().id);
}