use std::collections::{BTreeMap, BTreeSet};

use cssparser::color::{all_named_colors, parse_named_color};

#[test]
fn named_colors_include_css_color_keywords_and_aliases() {
    let colors: BTreeMap<&'static str, (u8, u8, u8)> = all_named_colors().collect();

    assert!(
        colors.len() > 100,
        "expected the full CSS named-color table, got only {} entries",
        colors.len()
    );

    assert_eq!(colors.get("black").copied(), Some((0, 0, 0)));
    assert_eq!(colors.get("white").copied(), Some((255, 255, 255)));
    assert_eq!(colors.get("red").copied(), Some((255, 0, 0)));
    assert_eq!(colors.get("aliceblue").copied(), Some((240, 248, 255)));
    assert_eq!(colors.get("rebeccapurple").copied(), Some((102, 51, 153)));

    assert_eq!(colors.get("gray").copied(), Some((128, 128, 128)));
    assert_eq!(colors.get("grey").copied(), Some((128, 128, 128)));
    assert_eq!(colors.get("cyan").copied(), Some((0, 255, 255)));
    assert_eq!(colors.get("aqua").copied(), Some((0, 255, 255)));
}

#[test]
fn named_color_iterator_has_unique_lowercase_names_and_matches_parser() {
    let entries: Vec<(&'static str, (u8, u8, u8))> = all_named_colors().collect();

    let mut names = BTreeSet::new();
    for (name, rgb) in &entries {
        assert!(
            names.insert(*name),
            "duplicate named color entry found for {name}"
        );
        assert!(
            name.chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit()),
            "named color should be serialized as a lowercase ASCII identifier: {name}"
        );

        let parsed = parse_named_color(name).expect("iterator entry should parse as a named color");
        assert_eq!(
            parsed, *rgb,
            "parse_named_color should agree with all_named_colors for {name}"
        );
    }

    assert_eq!(
        names.len(),
        entries.len(),
        "all_named_colors should not contain duplicate names"
    );
}

#[test]
fn named_color_lookup_workflow_accepts_case_insensitive_input() {
    let colors: BTreeMap<&'static str, (u8, u8, u8)> = all_named_colors().collect();

    let canonical = colors
        .get("rebeccapurple")
        .copied()
        .expect("rebeccapurple should be present in the named color table");

    assert_eq!(parse_named_color("rebeccapurple"), Ok(canonical));
    assert_eq!(parse_named_color("RebeccaPurple"), Ok(canonical));
    assert_eq!(parse_named_color("REBECCAPURPLE"), Ok(canonical));

    assert_eq!(parse_named_color("not-a-css-color"), Err(()));
    assert_eq!(colors.get("not-a-css-color"), None);
}