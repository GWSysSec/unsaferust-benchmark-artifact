use cssparser::*;
use cssparser::color::{all_named_colors, parse_named_color};
use std::collections::HashSet;

#[test]
fn test_all_named_colors_iteration_and_consistency() {
    let colors: Vec<(&'static str, (u8, u8, u8))> = all_named_colors().collect();


    assert!(colors.len() >= 100, "expected >=100 named colors, got {}", colors.len());
    assert_ne!(colors.len(), 0);


    let mut names: HashSet<&'static str> = HashSet::new();
    for (name, _) in &colors {
        assert!(!name.is_empty());
        assert!(name.chars().all(|c| c.is_ascii_lowercase()),
                "name not lowercase: {}", name);
        assert!(names.insert(name), "duplicate name: {}", name);
    }
    assert_eq!(names.len(), colors.len());


    let map: std::collections::HashMap<&str, (u8, u8, u8)> =
        colors.iter().cloned().collect();

    assert_eq!(map.get("red"), Some(&(255, 0, 0)));
    assert_eq!(map.get("green"), Some(&(0, 128, 0)));
    assert_eq!(map.get("blue"), Some(&(0, 0, 255)));
    assert_eq!(map.get("white"), Some(&(255, 255, 255)));
    assert_eq!(map.get("black"), Some(&(0, 0, 0)));
    assert_eq!(map.get("aqua"), Some(&(0, 255, 255)));
    assert_eq!(map.get("cyan"), Some(&(0, 255, 255)));
    assert_eq!(map.get("magenta"), Some(&(255, 0, 255)));
    assert_eq!(map.get("transparent"), None);
}

#[test]
fn test_all_named_colors_match_parse_named_color() {
    let colors: Vec<(&'static str, (u8, u8, u8))> = all_named_colors().collect();
    assert!(colors.len() > 50);

    let mut checked = 0usize;
    for (name, rgb) in &colors {

        let parsed = parse_named_color(name);
        assert!(parsed.is_ok(), "parse_named_color failed for {}", name);
        assert_eq!(parsed.unwrap(), *rgb);


        let upper = name.to_ascii_uppercase();
        let parsed_upper = parse_named_color(&upper);
        assert!(parsed_upper.is_ok());
        assert_eq!(parsed_upper.unwrap(), *rgb);

        checked += 1;
    }
    assert_eq!(checked, colors.len());
    assert_ne!(checked, 0);


    let bogus = parse_named_color("definitely_not_a_color_xyz");
    assert!(bogus.is_err());


    assert!(parse_named_color("").is_err());
}

#[test]
fn test_all_named_colors_iterator_is_repeatable_and_lazy() {

    let first: Vec<_> = all_named_colors().collect();
    let second: Vec<_> = all_named_colors().collect();

    assert_eq!(first.len(), second.len());
    assert!(first.len() > 0);
    assert_ne!(first.len(), 1);


    let s1: HashSet<&str> = first.iter().map(|(n, _)| *n).collect();
    let s2: HashSet<&str> = second.iter().map(|(n, _)| *n).collect();
    assert_eq!(s1, s2);
    assert_eq!(s1.len(), first.len());


    let it = all_named_colors();
    let (lo, _hi) = it.size_hint();

    let _ = lo;


    let taken: Vec<_> = all_named_colors().take(10).collect();
    assert_eq!(taken.len(), 10);


    for (name, rgb) in &taken {
        let p = parse_named_color(name).expect("must parse");
        assert_eq!(p.0, rgb.0);
        assert_eq!(p.1, rgb.1);
        assert_eq!(p.2, rgb.2);
    }
}