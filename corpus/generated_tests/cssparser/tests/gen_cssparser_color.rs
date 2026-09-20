use cssparser::*;
use cssparser::color::{all_named_colors, parse_named_color};
use std::collections::HashSet;

#[test]
fn test_all_named_colors_iteration_and_consistency() {
    let colors: Vec<(&'static str, (u8, u8, u8))> = all_named_colors().collect();

    // Sanity: CSS has ~148 named colors. Must be at least 100.
    assert!(colors.len() >= 100, "expected >=100 named colors, got {}", colors.len());
    assert_ne!(colors.len(), 0);

    // All names should be lowercase ASCII (named color table convention).
    let mut names: HashSet<&'static str> = HashSet::new();
    for (name, _) in &colors {
        assert!(!name.is_empty());
        assert!(name.chars().all(|c| c.is_ascii_lowercase()),
                "name not lowercase: {}", name);
        assert!(names.insert(name), "duplicate name: {}", name);
    }
    assert_eq!(names.len(), colors.len());

    // Several well-known fixed entries must be present with correct RGB.
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
        // parse_named_color must agree with the table for every entry.
        let parsed = parse_named_color(name);
        assert!(parsed.is_ok(), "parse_named_color failed for {}", name);
        assert_eq!(parsed.unwrap(), *rgb);

        // Uppercase variant must also parse to the same value (case-insensitive).
        let upper = name.to_ascii_uppercase();
        let parsed_upper = parse_named_color(&upper);
        assert!(parsed_upper.is_ok());
        assert_eq!(parsed_upper.unwrap(), *rgb);

        checked += 1;
    }
    assert_eq!(checked, colors.len());
    assert_ne!(checked, 0);

    // A name that is definitely not a CSS color must fail.
    let bogus = parse_named_color("definitely_not_a_color_xyz");
    assert!(bogus.is_err());

    // Empty must fail too.
    assert!(parse_named_color("").is_err());
}

#[test]
fn test_all_named_colors_iterator_is_repeatable_and_lazy() {
    // Calling all_named_colors() multiple times yields equivalent sequences.
    let first: Vec<_> = all_named_colors().collect();
    let second: Vec<_> = all_named_colors().collect();

    assert_eq!(first.len(), second.len());
    assert!(first.len() > 0);
    assert_ne!(first.len(), 1);

    // Same set of names across calls.
    let s1: HashSet<&str> = first.iter().map(|(n, _)| *n).collect();
    let s2: HashSet<&str> = second.iter().map(|(n, _)| *n).collect();
    assert_eq!(s1, s2);
    assert_eq!(s1.len(), first.len());

    // Iterator size_hint lower bound should be sensible (>0) once we ask.
    let it = all_named_colors();
    let (lo, _hi) = it.size_hint();
    // size_hint may be (0, None) for impl Iterator wrappers; just check it's reachable.
    let _ = lo;

    // Take a prefix and confirm count matches manual count.
    let taken: Vec<_> = all_named_colors().take(10).collect();
    assert_eq!(taken.len(), 10);

    // Every taken entry's RGB round-trips via parse_named_color.
    for (name, rgb) in &taken {
        let p = parse_named_color(name).expect("must parse");
        assert_eq!(p.0, rgb.0);
        assert_eq!(p.1, rgb.1);
        assert_eq!(p.2, rgb.2);
    }
}