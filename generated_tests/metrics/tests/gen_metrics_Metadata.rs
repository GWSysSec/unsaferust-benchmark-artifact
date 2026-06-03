use metrics::Level;
use metrics::Metadata;

#[test]
fn test_metadata_level_returns_correct_level() {
    let meta = Metadata::new(
        module_path!(),
        Level::INFO,
        Some(module_path!()),
    );
    let level = meta.level();
    assert_eq!(*level, Level::INFO);
    assert_ne!(*level, Level::DEBUG);
    assert_ne!(*level, Level::TRACE);
    assert_ne!(*level, Level::WARN);
    assert_ne!(*level, Level::ERROR);

    let meta_debug = Metadata::new(
        module_path!(),
        Level::DEBUG,
        Some(module_path!()),
    );
    assert_eq!(*meta_debug.level(), Level::DEBUG);
    assert_ne!(*meta_debug.level(), Level::INFO);
    assert_ne!(*meta_debug.level(), Level::TRACE);
}

#[test]
fn test_metadata_target_returns_correct_target() {
    let target_str = "my_crate::my_module";
    let meta = Metadata::new(
        target_str,
        Level::WARN,
        Some(module_path!()),
    );
    let target = meta.target();
    assert_eq!(target, "my_crate::my_module");
    assert_ne!(target, "");
    assert_ne!(target, "other_target");
    assert_eq!(target.len(), "my_crate::my_module".len());
    assert!(target.contains("my_crate"));
    assert!(target.starts_with("my_crate"));
    assert!(target.ends_with("my_module"));
    assert_eq!(target.split("::").count(), 2);
}

#[test]
fn test_metadata_module_path_some() {
    let mod_path = "metrics_tests::integration::submodule";
    let meta = Metadata::new(
        module_path!(),
        Level::ERROR,
        Some(mod_path),
    );
    let result = meta.module_path();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "metrics_tests::integration::submodule");
    assert_ne!(result.unwrap(), "");
    assert_eq!(result.unwrap().split("::").count(), 3);
    assert!(result.unwrap().contains("integration"));
    assert!(result.unwrap().starts_with("metrics_tests"));
    assert!(result.unwrap().ends_with("submodule"));
    assert_eq!(result.unwrap().len(), mod_path.len());
}

#[test]
fn test_metadata_module_path_none() {
    let meta = Metadata::new(
        module_path!(),
        Level::TRACE,
        None,
    );
    let result = meta.module_path();
    assert!(result.is_none());
    assert_eq!(result, None);
    assert_ne!(result, Some("something"));
    assert_eq!(*meta.level(), Level::TRACE);
    assert_ne!(*meta.level(), Level::INFO);
    assert_ne!(*meta.level(), Level::DEBUG);
    assert_ne!(*meta.level(), Level::WARN);
    assert_ne!(*meta.level(), Level::ERROR);
}

#[test]
fn test_metadata_all_levels_comprehensive() {
    let levels = [Level::TRACE, Level::DEBUG, Level::INFO, Level::WARN, Level::ERROR];
    let targets = [
        "app::server",
        "app::client",
        "app::db",
        "app::cache",
        "app::auth",
    ];

    for (i, &ref level) in levels.iter().enumerate() {
        let meta = Metadata::new(
            targets[i],
            level.clone(),
            Some(targets[i]),
        );
        assert_eq!(*meta.level(), *level);
        assert_eq!(meta.target(), targets[i]);
        assert_eq!(meta.module_path(), Some(targets[i]));
    }


    let meta_trace = Metadata::new("t", Level::TRACE, None);
    let meta_debug = Metadata::new("t", Level::DEBUG, None);
    let meta_info = Metadata::new("t", Level::INFO, None);
    let meta_warn = Metadata::new("t", Level::WARN, None);
    let meta_error = Metadata::new("t", Level::ERROR, None);

    assert_ne!(*meta_trace.level(), *meta_debug.level());
    assert_ne!(*meta_debug.level(), *meta_info.level());
    assert_ne!(*meta_info.level(), *meta_warn.level());
    assert_ne!(*meta_warn.level(), *meta_error.level());
    assert_ne!(*meta_trace.level(), *meta_error.level());
    assert_eq!(meta_trace.target(), "t");
    assert_eq!(meta_error.target(), "t");
    assert!(meta_trace.module_path().is_none());
}

#[test]
fn test_metadata_target_various_formats() {
    let meta_empty = Metadata::new("", Level::INFO, Some("mod"));
    assert_eq!(meta_empty.target(), "");
    assert_eq!(meta_empty.target().len(), 0);
    assert_eq!(meta_empty.module_path(), Some("mod"));

    let meta_long = Metadata::new(
        "very::deeply::nested::module::path::target",
        Level::DEBUG,
        Some("very::deeply::nested::module::path::target"),
    );
    assert_eq!(meta_long.target(), "very::deeply::nested::module::path::target");
    assert_eq!(meta_long.target().split("::").count(), 6);
    assert_eq!(meta_long.module_path().unwrap(), meta_long.target());
    assert_eq!(*meta_long.level(), Level::DEBUG);

    let meta_special = Metadata::new("target-with-dashes", Level::WARN, None);
    assert_eq!(meta_special.target(), "target-with-dashes");
    assert!(meta_special.target().contains('-'));
    assert!(meta_special.module_path().is_none());
}