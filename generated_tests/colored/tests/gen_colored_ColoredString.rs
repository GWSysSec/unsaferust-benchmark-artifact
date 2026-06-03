use colored::*;

#[test]
fn test_clear_fgcolor_workflow() {
    colored::control::set_override(true);

    let mut cs: ColoredString = "hello world".red().on_blue().bold();


    assert_eq!(cs.fgcolor(), Some(Color::Red));
    assert_ne!(cs.fgcolor(), None);
    assert_eq!(cs.is_plain(), false);
    let before = cs.to_string();
    assert_ne!(before, "hello world");
    assert!(before.contains("hello world"));

    cs.fgcolor = None;


    assert_eq!(cs.fgcolor(), None);
    assert_eq!(cs.is_plain(), false);
    let after = cs.to_string();
    assert_ne!(after, before);
    assert!(after.contains("hello world"));

    colored::control::unset_override();
}

#[test]
fn test_clear_bgcolor_then_fgcolor_then_style_to_plain() {
    colored::control::set_override(true);

    let mut cs: ColoredString = "data".green().on_yellow().italic().underline();


    assert_eq!(cs.fgcolor(), Some(Color::Green));
    assert_eq!(cs.is_plain(), false);
    let original = cs.to_string();
    assert_ne!(original, "data");
    assert!(original.contains("data"));
    let fg_str_before = Color::Green.to_fg_str();
    assert_eq!(fg_str_before.as_ref(), "32");


    cs.bgcolor = None;
    assert_eq!(cs.fgcolor(), Some(Color::Green));
    assert_eq!(cs.is_plain(), false);
    let after_bg = cs.to_string();
    assert_ne!(after_bg, original);


    cs.fgcolor = None;
    assert_eq!(cs.fgcolor(), None);
    assert_eq!(cs.is_plain(), false);


    cs.style = Style::default();
    assert_eq!(cs.fgcolor(), None);
    assert_eq!(cs.is_plain(), true);
    assert_eq!(cs.to_string(), "data");

    colored::control::unset_override();
}

#[test]
fn test_is_plain_initial_and_after_color() {
    colored::control::set_override(true);

    let plain: ColoredString = "plain text".normal();
    assert_eq!(plain.is_plain(), true);
    assert_eq!(plain.fgcolor(), None);
    assert_eq!(plain.to_string(), "plain text");

    let mut colored_str: ColoredString = "plain text".blue();
    assert_eq!(colored_str.is_plain(), false);
    assert_eq!(colored_str.fgcolor(), Some(Color::Blue));
    assert_ne!(colored_str.to_string(), "plain text");

    colored_str.fgcolor = None;
    assert_eq!(colored_str.is_plain(), true);
    assert_eq!(colored_str.fgcolor(), None);
    assert_eq!(colored_str.to_string(), "plain text");

    colored::control::unset_override();
}

#[test]
fn test_clear_style_preserves_colors() {
    colored::control::set_override(true);

    let mut cs: ColoredString = "styled".magenta().on_cyan().bold().underline().italic();

    assert_eq!(cs.fgcolor(), Some(Color::Magenta));
    assert_eq!(cs.is_plain(), false);
    let with_style = cs.to_string();
    assert!(with_style.contains("styled"));
    assert_ne!(with_style, "styled");

    cs.style = Style::default();


    assert_eq!(cs.fgcolor(), Some(Color::Magenta));
    assert_eq!(cs.is_plain(), false);
    let after = cs.to_string();
    assert_ne!(after, "styled");
    assert_ne!(after, with_style);
    assert!(after.contains("styled"));

    colored::control::unset_override();
}

#[test]
fn test_custom_color_then_clear() {
    colored::control::set_override(true);

    let custom = colored::customcolors::CustomColor::new(123, 45, 67);
    let mut cs: ColoredString = "custom".custom_color(custom).on_custom_color(colored::customcolors::CustomColor::new(10, 20, 30));

    assert_eq!(cs.is_plain(), false);
    assert_ne!(cs.fgcolor(), None);
    let before = cs.to_string();
    assert!(before.contains("custom"));
    assert_ne!(before, "custom");

    cs.bgcolor = None;
    let mid = cs.to_string();
    assert_ne!(mid, before);
    assert!(mid.contains("custom"));
    assert_eq!(cs.is_plain(), false);

    cs.fgcolor = None;
    assert_eq!(cs.fgcolor(), None);
    assert_eq!(cs.is_plain(), true);
    assert_eq!(cs.to_string(), "custom");

    colored::control::unset_override();
}

#[test]
fn test_repeated_clears_idempotent() {
    colored::control::set_override(true);

    let mut cs: ColoredString = "abc".red().on_green().bold();

    assert_eq!(cs.fgcolor(), Some(Color::Red));
    assert_eq!(cs.is_plain(), false);

    cs.fgcolor = None;
    cs.fgcolor = None;
    assert_eq!(cs.fgcolor(), None);
    assert_eq!(cs.is_plain(), false);

    cs.bgcolor = None;
    cs.bgcolor = None;
    assert_eq!(cs.is_plain(), false);

    cs.style = Style::default();
    cs.style = Style::default();
    assert_eq!(cs.is_plain(), true);
    assert_eq!(cs.fgcolor(), None);
    assert_eq!(cs.to_string(), "abc");

    colored::control::unset_override();
}