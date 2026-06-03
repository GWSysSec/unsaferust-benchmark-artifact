use filetime::*;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("filetime_it_{}_{}_{}", pid, nanos, name));
    p
}

#[test]
fn test_filetime_zero_and_now_basics() {
    let zero1 = FileTime::zero();
    let zero2 = FileTime::zero();


    assert_eq!(zero1.unix_seconds(), 0);
    assert_eq!(zero2.unix_seconds(), 0);
    assert_eq!(zero1, zero2);
    assert_eq!(zero1.unix_seconds(), zero2.unix_seconds());


    let now = FileTime::now();
    let n_secs = now.unix_seconds();
    assert!(n_secs > 1_577_836_800, "now() should be after 2020: {}", n_secs);
    assert!(n_secs < 4_102_444_800, "now() should be before 2100: {}", n_secs);


    assert_ne!(zero1, now);
    assert_ne!(zero1.unix_seconds(), now.unix_seconds());
    assert!(zero1.unix_seconds() < now.unix_seconds());


    let now_b = FileTime::now();
    assert!(now_b.unix_seconds() >= n_secs);
    assert!(now_b.unix_seconds() - n_secs < 5);
}

#[test]
fn test_set_file_mtime_to_epoch_zero() {
    let path = temp_path("mtime_zero");
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello world").unwrap();
    }


    let pre = fs::metadata(&path).unwrap();
    let pre_m = pre.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let now_secs = FileTime::now().unix_seconds();
    assert!(pre_m > 1_577_836_800);
    assert!((pre_m as i64 - now_secs).abs() < 10);
    assert_eq!(pre.len(), 11);


    let target = FileTime::zero();
    assert_eq!(target.unix_seconds(), 0);
    set_file_mtime(&path, target).unwrap();


    let post = fs::metadata(&path).unwrap();
    let post_m = post.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(post_m, 0);
    assert_eq!(post.len(), 11);
    assert_ne!(pre_m, post_m);

    fs::remove_file(&path).unwrap();
}

#[test]
fn test_set_file_times_both_and_atime_override() {
    let path = temp_path("times_both");
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(b"abcd").unwrap();
    }

    let zero = FileTime::zero();
    assert_eq!(zero.unix_seconds(), 0);


    set_file_times(&path, zero, zero).unwrap();

    let meta1 = fs::metadata(&path).unwrap();
    let m1 = meta1.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let a1 = meta1.accessed().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(m1, 0);
    assert_eq!(a1, 0);
    assert_eq!(meta1.len(), 4);


    let now = FileTime::now();
    let now_s = now.unix_seconds();
    assert!(now_s > 1_577_836_800);

    set_file_atime(&path, now).unwrap();

    let meta2 = fs::metadata(&path).unwrap();
    let a2 = meta2.accessed().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let m2 = meta2.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs();


    assert!((a2 - now_s).abs() < 5, "atime={} now={}", a2, now_s);
    assert_eq!(m2, 0);
    assert_ne!(a2, 0);
    assert_ne!(a2 as u64, m2);

    fs::remove_file(&path).unwrap();
}

#[test]
fn test_from_creation_time_on_fresh_file() {
    let path = temp_path("creation");
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(b"x").unwrap();
    }

    let before_now = FileTime::now().unix_seconds();
    assert!(before_now > 1_577_836_800);

    let meta = fs::metadata(&path).unwrap();
    let maybe_ct = FileTime::from_creation_time(&meta);


    let zero = FileTime::zero();
    let now = FileTime::now();
    assert_eq!(zero.unix_seconds(), 0);
    assert!(now.unix_seconds() >= before_now);
    assert_ne!(zero, now);

    match maybe_ct {
        Some(ct) => {
            let ct_s = ct.unix_seconds();

            assert!(ct_s > 1_577_836_800, "ctime={}", ct_s);

            assert!(ct_s <= now.unix_seconds() + 2);

            assert!(ct_s >= before_now - 60);
            assert_ne!(ct, zero);
            assert_ne!(ct_s, 0);
        }
        None => {

            assert_eq!(FileTime::zero().unix_seconds(), 0);
            assert!(FileTime::now().unix_seconds() > before_now - 1);
            assert_ne!(FileTime::zero(), FileTime::now());
            assert_ne!(FileTime::zero().unix_seconds(), FileTime::now().unix_seconds());
        }
    }

    fs::remove_file(&path).unwrap();
}

#[test]
fn test_set_file_handle_times_selective() {
    let path = temp_path("handle");
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(b"handle").unwrap();
    }

    let zero = FileTime::zero();
    assert_eq!(zero.unix_seconds(), 0);


    let f = File::open(&path).unwrap();
    set_file_handle_times(&f, Some(zero), Some(zero)).unwrap();

    let meta1 = fs::metadata(&path).unwrap();
    let m1 = meta1.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let a1 = meta1.accessed().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(m1, 0);
    assert_eq!(a1, 0);


    let now = FileTime::now();
    let now_s = now.unix_seconds();
    assert!(now_s > 1_577_836_800);
    set_file_handle_times(&f, None, Some(now)).unwrap();

    let meta2 = fs::metadata(&path).unwrap();
    let m2 = meta2.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    assert!((m2 - now_s).abs() < 5, "mtime={} now={}", m2, now_s);


    set_file_handle_times(&f, Some(now), None).unwrap();
    let meta3 = fs::metadata(&path).unwrap();
    let a3 = meta3.accessed().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    assert!((a3 - now_s).abs() < 5);
    assert_ne!(a3, 0);
    assert_eq!(meta3.len(), 6);

    drop(f);
    fs::remove_file(&path).unwrap();
}

#[test]
fn test_set_symlink_file_times_does_not_touch_target() {
    let target = temp_path("sym_target");
    {
        let mut f = File::create(&target).unwrap();
        f.write_all(b"payload").unwrap();
    }

    let zero = FileTime::zero();
    let now = FileTime::now();
    assert_eq!(zero.unix_seconds(), 0);
    assert!(now.unix_seconds() > 1_577_836_800);
    assert_ne!(zero, now);
    assert_ne!(zero.unix_seconds(), now.unix_seconds());

    #[cfg(unix)]
    {
        let link = temp_path("sym_link");
        std::os::unix::fs::symlink(&target, &link).unwrap();


        let t_before = fs::metadata(&target).unwrap();
        let t_m_before = t_before
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(t_m_before > 1_577_836_800);


        set_symlink_file_times(&link, zero, zero).unwrap();

        let link_meta = fs::symlink_metadata(&link).unwrap();
        let lm = link_meta
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let la = link_meta
            .accessed()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(lm, 0);
        assert_eq!(la, 0);


        let t_after = fs::metadata(&target).unwrap();
        let t_m_after = t_after
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(t_m_before, t_m_after);
        assert_ne!(t_m_after, 0);

        fs::remove_file(&link).unwrap();
    }

    fs::remove_file(&target).unwrap();
}