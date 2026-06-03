use git2::{DiffOptions, Repository, Signature};
use std::fs;
use std::path::{Path, PathBuf};

fn temp_path(tag: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("git2_do_{}_{}_{}", tag, pid, nanos));
    if p.exists() {
        let _ = fs::remove_dir_all(&p);
    }
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

fn write_file(root: &Path, rel: &str, contents: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, contents).unwrap();
}

fn commit_all(repo: &Repository, msg: &str) -> git2::Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test", "test@example.com").unwrap();
    let parents: Vec<git2::Commit> = match repo.head() {
        Ok(h) => vec![h.peel_to_commit().unwrap()],
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)
        .unwrap()
}

#[test]
fn test_diff_options_reverse_swaps_old_and_new() {
    let dir = temp_path("reverse");
    let repo = init_repo(&dir);

    write_file(&dir, "a.txt", "hello\n");
    let c1 = commit_all(&repo, "c1");

    write_file(&dir, "a.txt", "hello world\n");
    let c2 = commit_all(&repo, "c2");

    let t1 = repo.find_commit(c1).unwrap().tree().unwrap();
    let t2 = repo.find_commit(c2).unwrap().tree().unwrap();


    let mut opts_fwd = DiffOptions::new();
    opts_fwd.reverse(false);
    let diff_fwd = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_fwd))
        .unwrap();
    assert_eq!(diff_fwd.deltas().len(), 1);
    let fwd = diff_fwd.get_delta(0).unwrap();
    let fwd_old_id = fwd.old_file().id();
    let fwd_new_id = fwd.new_file().id();
    assert_ne!(fwd_old_id, fwd_new_id);
    assert_ne!(fwd_old_id, git2::Oid::zero());
    assert_ne!(fwd_new_id, git2::Oid::zero());


    let mut opts_rev = DiffOptions::new();
    opts_rev.reverse(true);
    let diff_rev = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_rev))
        .unwrap();
    assert_eq!(diff_rev.deltas().len(), 1);
    let rev = diff_rev.get_delta(0).unwrap();
    assert_eq!(rev.old_file().id(), fwd_new_id);
    assert_eq!(rev.new_file().id(), fwd_old_id);
    assert_ne!(rev.old_file().id(), rev.new_file().id());
}

#[test]
fn test_diff_options_include_ignored_and_recurse_dirs() {
    let dir = temp_path("ignored");
    let repo = init_repo(&dir);

    write_file(&dir, ".gitignore", "ignored_dir/\n*.log\n");
    write_file(&dir, "tracked.txt", "hi\n");
    commit_all(&repo, "init");

    write_file(&dir, "untracked_dir/u1.txt", "u1\n");
    write_file(&dir, "untracked_dir/u2.txt", "u2\n");
    write_file(&dir, "ignored_dir/i1.txt", "i1\n");
    write_file(&dir, "plain.log", "log\n");


    let mut opts_no_ign = DiffOptions::new();
    opts_no_ign
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .recurse_ignored_dirs(false);
    let diff_no_ign = repo
        .diff_index_to_workdir(None, Some(&mut opts_no_ign))
        .unwrap();
    let paths_no_ign: Vec<String> = diff_no_ign
        .deltas()
        .filter_map(|d| d.new_file().path().map(|p| p.to_string_lossy().into_owned()))
        .collect();
    assert!(!paths_no_ign.is_empty());
    let has_log = paths_no_ign.iter().any(|p| p == "plain.log");
    let has_ign = paths_no_ign.iter().any(|p| p.contains("ignored_dir"));
    assert_eq!(has_log, false);
    assert_eq!(has_ign, false);

    let has_untracked_file = paths_no_ign
        .iter()
        .any(|p| p.starts_with("untracked_dir/"));
    assert_eq!(has_untracked_file, true);


    let mut opts_ign = DiffOptions::new();
    opts_ign
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(true)
        .recurse_ignored_dirs(true);
    let diff_ign = repo
        .diff_index_to_workdir(None, Some(&mut opts_ign))
        .unwrap();
    let paths_ign: Vec<String> = diff_ign
        .deltas()
        .filter_map(|d| d.new_file().path().map(|p| p.to_string_lossy().into_owned()))
        .collect();
    let has_log2 = paths_ign.iter().any(|p| p == "plain.log");
    let has_ign2 = paths_ign
        .iter()
        .any(|p| p.starts_with("ignored_dir/") || p == "ignored_dir");
    assert_eq!(has_log2, true);
    assert_eq!(has_ign2, true);
    assert!(paths_ign.len() > paths_no_ign.len());
}

#[test]
fn test_diff_options_pathspec_and_disable_pathspec_match() {
    let dir = temp_path("pathspec");
    let repo = init_repo(&dir);

    write_file(&dir, "keep.txt", "keep\n");
    write_file(&dir, "skip.txt", "skip\n");
    let c1 = commit_all(&repo, "c1");

    write_file(&dir, "keep.txt", "keep changed\n");
    write_file(&dir, "skip.txt", "skip changed\n");
    let c2 = commit_all(&repo, "c2");

    let t1 = repo.find_commit(c1).unwrap().tree().unwrap();
    let t2 = repo.find_commit(c2).unwrap().tree().unwrap();


    let mut opts_ps = DiffOptions::new();
    opts_ps
        .pathspec("keep.txt")
        .ignore_filemode(true)
        .ignore_submodules(true)
        .ignore_case(false)
        .skip_binary_check(true)
        .enable_fast_untracked_dirs(true)
        .update_index(false)
        .include_unreadable(false)
        .disable_pathspec_match(false);
    let diff_ps = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_ps))
        .unwrap();
    assert_eq!(diff_ps.deltas().len(), 1);
    let d_ps = diff_ps.get_delta(0).unwrap();
    assert_eq!(d_ps.new_file().path().unwrap(), Path::new("keep.txt"));
    assert_eq!(d_ps.status(), git2::Delta::Modified);


    let mut opts_disable = DiffOptions::new();
    opts_disable
        .pathspec("keep.txt")
        .disable_pathspec_match(true);
    let diff_disable = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_disable))
        .unwrap();
    assert_eq!(diff_disable.deltas().len(), 1);
    assert_eq!(
        diff_disable.get_delta(0).unwrap().new_file().path().unwrap(),
        Path::new("keep.txt")
    );


    let mut opts_all = DiffOptions::new();
    let diff_all = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_all))
        .unwrap();
    assert_eq!(diff_all.deltas().len(), 2);
    assert_ne!(diff_all.deltas().len(), diff_ps.deltas().len());
}

#[test]
fn test_diff_options_include_typechange_and_unmodified_flags_compile() {




    let dir = temp_path("typechange");
    let repo = init_repo(&dir);

    write_file(&dir, "same.txt", "same\n");
    write_file(&dir, "edit.txt", "v1\n");
    let c1 = commit_all(&repo, "c1");

    write_file(&dir, "edit.txt", "v2\n");
    let c2 = commit_all(&repo, "c2");

    let t1 = repo.find_commit(c1).unwrap().tree().unwrap();
    let t2 = repo.find_commit(c2).unwrap().tree().unwrap();

    let mut opts = DiffOptions::new();
    opts.include_unmodified(false)
        .include_typechange(true)
        .include_typechange_trees(false);
    let diff = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts))
        .unwrap();
    let count = diff.deltas().len();
    assert_eq!(count, 1);
    let d = diff.get_delta(0).unwrap();
    assert_eq!(d.new_file().path().unwrap(), Path::new("edit.txt"));
    assert_eq!(d.status(), git2::Delta::Modified);
    assert_ne!(d.old_file().id(), d.new_file().id());


    let mut opts2 = DiffOptions::new();
    opts2
        .include_typechange(false)
        .include_typechange_trees(true);
    let diff2 = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts2))
        .unwrap();
    assert_eq!(diff2.deltas().len(), 1);
    let d2 = diff2.get_delta(0).unwrap();
    assert_eq!(d2.new_file().path().unwrap(), Path::new("edit.txt"));
    assert_eq!(diff.deltas().len(), diff2.deltas().len());
}

#[test]
fn test_diff_options_all_builder_methods_chain() {

    let mut opts = DiffOptions::new();
    let r: &mut DiffOptions = opts
        .reverse(false)
        .include_ignored(false)
        .recurse_ignored_dirs(false)
        .recurse_untracked_dirs(true)
        .include_unmodified(false)
        .include_typechange(false)
        .include_typechange_trees(false)
        .ignore_filemode(true)
        .ignore_submodules(true)
        .ignore_case(false)
        .disable_pathspec_match(false)
        .skip_binary_check(true)
        .enable_fast_untracked_dirs(true)
        .update_index(false)
        .include_unreadable(false);
    r.include_untracked(true);

    let dir = temp_path("chain");
    let repo = init_repo(&dir);

    write_file(&dir, "one.txt", "1\n");
    write_file(&dir, "two.txt", "2\n");
    commit_all(&repo, "initial");


    write_file(&dir, "one.txt", "1-modified\n");
    write_file(&dir, "new.txt", "new\n");

    let diff = repo
        .diff_index_to_workdir(None, Some(&mut opts))
        .unwrap();
    let paths: Vec<(git2::Delta, String)> = diff
        .deltas()
        .map(|d| {
            (
                d.status(),
                d.new_file()
                    .path()
                    .or_else(|| d.old_file().path())
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            )
        })
        .collect();

    assert!(paths.len() >= 2);
    let has_modified = paths
        .iter()
        .any(|(s, p)| *s == git2::Delta::Modified && p == "one.txt");
    let has_untracked = paths
        .iter()
        .any(|(s, p)| *s == git2::Delta::Untracked && p == "new.txt");
    assert_eq!(has_modified, true);
    assert_eq!(has_untracked, true);

    let has_two = paths.iter().any(|(_, p)| p == "two.txt");
    assert_eq!(has_two, false);

    assert_eq!(paths.len(), 2);
}