use nu_ansi_term::{sub_string, unstyle, unstyled_len, AnsiStrings, Color, Style};

#[test]
fn ansi_rendering_can_be_used_repeatedly_without_panicking() {
    let first = Color::Red.paint("first");
    let second = Color::Blue.paint("second");
    let parts = [first, second];
    let strings = AnsiStrings(&parts);

    let rendered_once = strings.to_string();
    let rendered_twice = strings.to_string();

    assert_eq!(unstyle(&strings), "firstsecond");
    assert_eq!(unstyled_len(&strings), "firstsecond".len());
    assert_eq!(rendered_once, rendered_twice);
    assert!(rendered_once.contains("first"));
    assert!(rendered_once.contains("second"));
    assert!(rendered_once.contains("\u{1b}["));
}

#[test]
fn styled_string_workflow_preserves_plain_text_semantics() {
    let red = Color::Red.paint("red");
    let bold = Style::new().bold().paint(" bold");
    let blue_underlined = Color::Blue.underline().paint(" blue");

    let parts = [red, bold, blue_underlined];
    let strings = AnsiStrings(&parts);

    assert_eq!(unstyle(&strings), "red bold blue");
    assert_eq!(unstyled_len(&strings), "red bold blue".len());

    let rendered = strings.to_string();
    assert!(rendered.contains("red"));
    assert!(rendered.contains(" bold"));
    assert!(rendered.contains(" blue"));
    assert!(rendered.contains("\u{1b}["));
    assert!(rendered.ends_with("\u{1b}[0m"));
}

#[test]
fn substring_across_multiple_styled_segments_keeps_visible_text_correct() {
    let parts = [
        Color::Green.paint("alpha"),
        Style::new().italic().paint("-beta-"),
        Color::Purple.bold().paint("gamma"),
    ];
    let strings = AnsiStrings(&parts);

    let middle = sub_string(3, 8, &strings);
    let middle_strings = AnsiStrings(&middle);

    assert_eq!(unstyle(&strings), "alpha-beta-gamma");
    assert_eq!(unstyled_len(&strings), 16);
    assert_eq!(unstyle(&middle_strings), "ha-beta-");
    assert_eq!(unstyled_len(&middle_strings), 8);

    let rendered_middle = middle_strings.to_string();
    assert!(rendered_middle.contains("ha"));
    assert!(rendered_middle.contains("beta"));
    assert!(rendered_middle.contains("\u{1b}["));
}