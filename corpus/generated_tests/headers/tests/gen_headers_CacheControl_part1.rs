use std::time::Duration;

use headers::CacheControl;

#[test]
fn test_cache_control_new_defaults_all_false() {
    let cc = CacheControl::new();

    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
    assert_eq!(cc.max_stale(), None);
    assert_eq!(cc.min_fresh(), None);
    assert_eq!(cc.s_max_age(), None);
}

#[test]
fn test_cache_control_with_no_store() {
    let cc = CacheControl::new();
    assert_eq!(cc.no_store(), false);

    let cc = cc.with_no_store();
    assert_eq!(cc.no_store(), true);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
    assert_eq!(cc.max_stale(), None);
    assert_eq!(cc.min_fresh(), None);
}

#[test]
fn test_cache_control_with_no_transform() {
    let cc = CacheControl::new();
    assert_eq!(cc.no_transform(), false);

    let cc = cc.with_no_transform();
    assert_eq!(cc.no_transform(), true);
    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
    assert_eq!(cc.max_stale(), None);
    assert_eq!(cc.min_fresh(), None);
}

#[test]
fn test_cache_control_with_only_if_cached() {
    let cc = CacheControl::new();
    assert_eq!(cc.only_if_cached(), false);

    let cc = cc.with_only_if_cached();
    assert_eq!(cc.only_if_cached(), true);
    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
    assert_eq!(cc.max_stale(), None);
    assert_eq!(cc.min_fresh(), None);
}

#[test]
fn test_cache_control_with_public() {
    let cc = CacheControl::new();
    assert_eq!(cc.public(), false);

    let cc = cc.with_public();
    assert_eq!(cc.public(), true);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.max_age(), None);
    assert_eq!(cc.max_stale(), None);
    assert_eq!(cc.min_fresh(), None);
}

#[test]
fn test_cache_control_with_max_stale() {
    let cc = CacheControl::new();
    assert_eq!(cc.max_stale(), None);

    let duration = Duration::from_secs(3600);
    let cc = cc.with_max_stale(duration);
    assert_eq!(cc.max_stale(), Some(Duration::from_secs(3600)));
    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
    assert_eq!(cc.min_fresh(), None);
}

#[test]
fn test_cache_control_with_min_fresh() {
    let cc = CacheControl::new();
    assert_eq!(cc.min_fresh(), None);

    let duration = Duration::from_secs(120);
    let cc = cc.with_min_fresh(duration);
    assert_eq!(cc.min_fresh(), Some(Duration::from_secs(120)));
    assert_eq!(cc.max_stale(), None);
    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
}

#[test]
fn test_cache_control_chained_builders() {
    let cc = CacheControl::new()
        .with_no_store()
        .with_no_transform()
        .with_only_if_cached()
        .with_public()
        .with_max_stale(Duration::from_secs(600))
        .with_min_fresh(Duration::from_secs(60));

    assert_eq!(cc.no_store(), true);
    assert_eq!(cc.no_transform(), true);
    assert_eq!(cc.only_if_cached(), true);
    assert_eq!(cc.public(), true);
    assert_eq!(cc.max_stale(), Some(Duration::from_secs(600)));
    assert_eq!(cc.min_fresh(), Some(Duration::from_secs(60)));
    assert_eq!(cc.max_age(), None);
    assert_eq!(cc.s_max_age(), None);
    assert_eq!(cc.private(), false);
}

#[test]
fn test_cache_control_max_stale_zero_duration() {
    let cc = CacheControl::new().with_max_stale(Duration::from_secs(0));
    assert_eq!(cc.max_stale(), Some(Duration::from_secs(0)));
    assert_eq!(cc.min_fresh(), None);
    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
}

#[test]
fn test_cache_control_min_fresh_large_duration() {
    let large_duration = Duration::from_secs(86400 * 365);
    let cc = CacheControl::new().with_min_fresh(large_duration);
    assert_eq!(cc.min_fresh(), Some(large_duration));
    assert_eq!(cc.max_stale(), None);
    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
}

#[test]
fn test_cache_control_multiple_flags_independence() {
    let cc_store = CacheControl::new().with_no_store();
    let cc_transform = CacheControl::new().with_no_transform();
    let cc_cached = CacheControl::new().with_only_if_cached();
    let cc_public = CacheControl::new().with_public();

    assert_eq!(cc_store.no_store(), true);
    assert_eq!(cc_store.no_transform(), false);
    assert_eq!(cc_transform.no_transform(), true);
    assert_eq!(cc_transform.no_store(), false);
    assert_eq!(cc_cached.only_if_cached(), true);
    assert_eq!(cc_cached.public(), false);
    assert_eq!(cc_public.public(), true);
    assert_eq!(cc_public.only_if_cached(), false);
}

#[test]
fn test_cache_control_with_max_stale_then_min_fresh_both_present() {
    let stale_dur = Duration::from_secs(300);
    let fresh_dur = Duration::from_secs(60);
    let cc = CacheControl::new()
        .with_max_stale(stale_dur)
        .with_min_fresh(fresh_dur);

    assert_eq!(cc.max_stale(), Some(stale_dur));
    assert_eq!(cc.min_fresh(), Some(fresh_dur));
    assert_ne!(cc.max_stale(), cc.min_fresh());
    assert_eq!(cc.no_store(), false);
    assert_eq!(cc.no_transform(), false);
    assert_eq!(cc.only_if_cached(), false);
    assert_eq!(cc.public(), false);
    assert_eq!(cc.private(), false);
    assert_eq!(cc.max_age(), None);
}