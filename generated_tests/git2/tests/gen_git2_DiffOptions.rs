use git2::{Diff, DiffFormat, DiffOptions, Repository, Signature};
use std::fs;
use std::path::{Path, PathBuf};

fn temp_path(tag: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("git2_do2_{}_{}_{}", tag, pid, nanos));
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

fn write_file(root: &Path, rel: &str, contents: &[u8]) {
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

fn diff_to_patch_string(diff: &Diff) -> String {
    let mut s = String::new();
    diff.print(DiffFormat::Patch, |_d, _h, line| {
        let origin = line.origin();
        if origin == 'F' || origin == 'H' {
            s.push_str(&String::from_utf8_lossy(line.content()));
        } else {
            s.push(origin);
            s.push_str(&String::from_utf8_lossy(line.content()));
        }
        true
    })
    .unwrap();
    s
}

#[test]
fn test_diff_options_prefixes_and_context_lines() {
    let dir = temp_path("prefix");
    let repo = init_repo(&dir);

    let v1: String = (0..10).map(|i| format!("line {}\n", i)).collect();
    write_file(&dir, "file.txt", v1.as_bytes());
    let c1 = commit_all(&repo, "c1");

    let v2: String = (0..10)
        .map(|i| if i == 5 { "line CHANGED\n".to_string() } else { format!("line {}\n", i) })
        .collect();
    write_file(&dir, "file.txt", v2.as_bytes());
    let c2 = commit_all(&repo, "c2");

    let t1 = repo.find_commit(c1).unwrap().tree().unwrap();
    let t2 = repo.find_commit(c2).unwrap().tree().unwrap();


    let mut opts = DiffOptions::new();
    opts.old_prefix("custom_old/")
        .new_prefix("custom_new/")
        .context_lines(0u32)
        .interhunk_lines(0u32);
    let diff = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts))
        .unwrap();
    let patch = diff_to_patch_string(&diff);

    assert!(patch.contains("custom_old/file.txt"));
    assert!(patch.contains("custom_new/file.txt"));
    assert_eq!(patch.contains("a/file.txt"), false);
    assert_eq!(patch.contains("b/file.txt"), false);
    assert!(patch.contains("CHANGED"));

    assert_eq!(patch.contains("line 0\n"), false);
    assert_eq!(patch.contains("line 9\n"), false);


    let mut opts_def = DiffOptions::new();
    opts_def.context_lines(3u32);
    let diff_def = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_def))
        .unwrap();
    let patch_def = diff_to_patch_string(&diff_def);
    assert!(patch_def.contains("a/file.txt"));
    assert!(patch_def.contains("b/file.txt"));
    assert!(patch_def.contains("line 4"));
    assert!(patch_def.contains("line 6"));
    assert_ne!(patch.len(), patch_def.len());
}

#[test]
fn test_diff_options_force_binary_and_force_text() {
    let dir = temp_path("binary");
    let repo = init_repo(&dir);


    write_file(&dir, "t.txt", b"hello\nworld\n");
    let c1 = commit_all(&repo, "c1");

    write_file(&dir, "t.txt", b"hello\nrust world\n");
    let c2 = commit_all(&repo, "c2");

    let t1 = repo.find_commit(c1).unwrap().tree().unwrap();
    let t2 = repo.find_commit(c2).unwrap().tree().unwrap();


    let mut opts_bin = DiffOptions::new();
    opts_bin.force_binary(true).force_text(false);
    let diff_bin = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_bin))
        .unwrap();
    let patch_bin = diff_to_patch_string(&diff_bin);
    assert!(patch_bin.to_lowercase().contains("binary"));

    assert_eq!(patch_bin.contains("rust world"), false);


    let mut opts_txt = DiffOptions::new();
    opts_txt.force_binary(false).force_text(true);
    let diff_txt = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_txt))
        .unwrap();
    let patch_txt = diff_to_patch_string(&diff_txt);
    assert!(patch_txt.contains("rust world"));
    assert_eq!(
        patch_txt.to_lowercase().contains("binary files"),
        false
    );
    assert_ne!(patch_bin.len(), patch_txt.len());
}

#[test]
fn test_diff_options_id_abbrev_and_max_size() {
    let dir = temp_path("idabbr");
    let repo = init_repo(&dir);

    write_file(&dir, "a.txt", b"v1\n");
    let c1 = commit_all(&repo, "c1");
    write_file(&dir, "a.txt", b"v2\n");
    let c2 = commit_all(&repo, "c2");

    let t1 = repo.find_commit(c1).unwrap().tree().unwrap();
    let t2 = repo.find_commit(c2).unwrap().tree().unwrap();


    let mut opts_full = DiffOptions::new();
    opts_full.id_abbrev(40u16);
    let diff_full = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_full))
        .unwrap();
    let patch_full = diff_to_patch_string(&diff_full);
    assert!(patch_full.contains("index "));


    let mut opts_abbr = DiffOptions::new();
    opts_abbr.id_abbrev(7u16);
    let diff_abbr = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_abbr))
        .unwrap();
    let patch_abbr = diff_to_patch_string(&diff_abbr);
    assert!(patch_abbr.contains("index "));
    assert_ne!(patch_full.len(), patch_abbr.len());


    let full_idx_line = patch_full
        .lines()
        .find(|l| l.starts_with("index "))
        .unwrap();
    let abbr_idx_line = patch_abbr
        .lines()
        .find(|l| l.starts_with("index "))
        .unwrap();
    assert!(full_idx_line.len() > abbr_idx_line.len());


    let mut opts_max = DiffOptions::new();
    opts_max.max_size(1i64);
    let diff_max = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_max))
        .unwrap();
    let patch_max = diff_to_patch_string(&diff_max);
    assert!(patch_max.to_lowercase().contains("binary"));


    let mut opts_normal = DiffOptions::new();
    opts_normal.max_size(1_000_000i64);
    let diff_normal = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts_normal))
        .unwrap();
    let patch_normal = diff_to_patch_string(&diff_normal);
    assert!(patch_normal.contains("v2"));
    assert_eq!(
        patch_normal.to_lowercase().contains("binary files"),
        false
    );
}

#[test]
fn test_diff_options_show_untracked_content() {
    let dir = temp_path("show_untracked");
    let repo = init_repo(&dir);

    write_file(&dir, "tracked.txt", b"tracked\n");
    commit_all(&repo, "c1");


    write_file(&dir, "newfile.txt", b"UNTRACKED_CONTENT_MARKER\n");


    let mut opts_no = DiffOptions::new();
    opts_no
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(false);
    let diff_no = repo
        .diff_index_to_workdir(None, Some(&mut opts_no))
        .unwrap();
    let patch_no = diff_to_patch_string(&diff_no);
    assert_eq!(patch_no.contains("UNTRACKED_CONTENT_MARKER"), false);


    let mut opts_yes = DiffOptions::new();
    opts_yes
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    let diff_yes = repo
        .diff_index_to_workdir(None, Some(&mut opts_yes))
        .unwrap();
    let patch_yes = diff_to_patch_string(&diff_yes);
    assert!(patch_yes.contains("UNTRACKED_CONTENT_MARKER"));
    assert!(patch_yes.len() > patch_no.len());


    assert_eq!(diff_yes.deltas().len(), 1);
    let d = diff_yes.get_delta(0).unwrap();
    assert_eq!(d.status(), git2::Delta::Untracked);
    assert_eq!(d.new_file().path().unwrap(), Path::new("newfile.txt"));
}

#[test]
fn test_diff_options_all_remaining_builder_methods_chain() {

    let mut opts = DiffOptions::new();
    let r: &mut DiffOptions = opts
        .include_unreadable_as_untracked(false)
        .force_text(false)
        .force_binary(false)
        .ignore_blank_lines(true)
        .show_untracked_content(false)
        .show_unmodified(false)
        .minimal(true)
        .indent_heuristic(true)
        .context_lines(3u32)
        .interhunk_lines(1u32)
        .id_abbrev(7u16)
        .max_size(1_000_000i64)
        .old_prefix("a/")
        .new_prefix("b/");
    r.context_lines(5u32);


    let dir = temp_path("allopts");
    let repo = init_repo(&dir);

    let v1: String = (0..20).map(|i| format!("l{}\n", i)).collect();
    write_file(&dir, "f.txt", v1.as_bytes());
    let c1 = commit_all(&repo, "c1");

    let v2: String = (0..20)
        .map(|i| if i == 10 { "l10_changed\n".into() } else { format!("l{}\n", i) })
        .collect();
    write_file(&dir, "f.txt", v2.as_bytes());
    let c2 = commit_all(&repo, "c2");

    let t1 = repo.find_commit(c1).unwrap().tree().unwrap();
    let t2 = repo.find_commit(c2).unwrap().tree().unwrap();

    let diff = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut opts))
        .unwrap();
    assert_eq!(diff.deltas().len(), 1);
    let delta = diff.get_delta(0).unwrap();
    assert_eq!(delta.status(), git2::Delta::Modified);
    assert_eq!(delta.new_file().path().unwrap(), Path::new("f.txt"));

    let patch = diff_to_patch_string(&diff);
    assert!(patch.contains("l10_changed"));
    assert!(patch.contains("a/f.txt"));
    assert!(patch.contains("b/f.txt"));

    assert!(patch.contains("l9") || patch.contains("l11"));

    let stats = diff.stats().unwrap();
    assert_eq!(stats.files_changed(), 1);
    assert_eq!(stats.insertions(), 1);
    assert_eq!(stats.deletions(), 1);
}