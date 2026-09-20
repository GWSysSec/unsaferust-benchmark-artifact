use nu_ansi_term::{unstyle, unstyled_len, AnsiStrings, Color, Style};

#[test]
fn mutable_style_access_preserves_text_and_updates_rendering() {
    let mut item = Style::new().paint("deploy");

    assert_eq!(item.as_str(), "deploy");
    assert_eq!(item.url_string(), None);
    assert!(item.style_ref().is_plain());

    let replacement_style = item
        .style_ref()
        .bold()
        .underline()
        .fg(Color::Green)
        .on(Color::Black);
    *item.style_ref_mut() = replacement_style;

    assert_eq!(item.as_str(), "deploy");
    assert_eq!(item.url_string(), None);
    assert!(!item.style_ref().is_plain());

    let rendered = item.to_string();
    assert!(rendered.contains("deploy"));
    assert!(rendered.contains("\x1b["));
    assert!(rendered.ends_with("\x1b[0m"));

    let written = format!("{}", item);

    assert_eq!(written, rendered);
}

#[test]
fn hyperlink_url_and_underlying_text_survive_style_mutation() {
    let mut link = Color::Blue
        .underline()
        .paint("release notes")
        .hyperlink("https://example.com/releases");

    assert_eq!(link.as_str(), "release notes");
    assert_eq!(link.url_string(), Some("https://example.com/releases"));
    assert!(!link.style_ref().is_plain());

    let changed_style = link.style_ref().bold().fg(Color::Yellow);
    *link.style_ref_mut() = changed_style;

    assert_eq!(link.as_str(), "release notes");
    assert_eq!(link.url_string(), Some("https://example.com/releases"));
    assert!(!link.style_ref().is_plain());

    let rendered = link.to_string();
    assert!(rendered.contains("release notes"));
    assert!(rendered.contains("https://example.com/releases"));
    assert!(rendered.contains("\x1b]8;;"));
    assert!(rendered.contains("\x1b["));
}

#[test]
fn styled_strings_can_be_grouped_unstyled_and_mutated_independently() {
    let mut status = Color::Red.paint("failed");
    let reason = Style::new().italic().paint(": timeout");
    let suffix = Style::new().paint("!");

    assert_eq!(status.as_str(), "failed");
    assert_eq!(status.url_string(), None);

    let updated_status_style = status.style_ref().bold().fg(Color::Purple);
    *status.style_ref_mut() = updated_status_style;

    let parts = [status, reason, suffix];
    let grouped = AnsiStrings(&parts);

    assert_eq!(unstyle(&grouped), "failed: timeout!");
    assert_eq!(unstyled_len(&grouped), "failed: timeout!".len());

    let rendered = grouped.to_string();
    assert!(rendered.contains("failed"));
    assert!(rendered.contains(": timeout"));
    assert!(rendered.contains("!"));
    assert!(rendered.contains("\x1b["));
    assert!(rendered.ends_with("\x1b[0m") || rendered.ends_with('!'));
}