use nu_ansi_term::{AnsiByteStrings, Color, Style};

#[test]
fn byte_strings_can_be_composed_and_written() {
    let red_bytes = Color::Red.paint(&b"red"[..]);
    let bold_bytes = Style::new().bold().paint(&b"bold"[..]);
    let plain_bytes = Style::new().paint(&b"plain"[..]);

    let parts = [red_bytes, bold_bytes, plain_bytes];
    let grouped = AnsiByteStrings(&parts);

    let mut output = Vec::new();
    grouped.write_to(&mut output).expect("writing ANSI byte strings should succeed");

    assert!(output.starts_with(b"\x1b[31mred"));
    assert!(output.windows(b"bold".len()).any(|window| window == b"bold"));
    assert!(output.ends_with(b"plain\x1b[0m") || output.ends_with(b"plain"));
    assert!(output.contains(&b'\x1b'));
}

#[test]
fn style_ref_mut_updates_a_painted_string_style() {
    let mut message = Style::new().paint("warning");

    assert_eq!(message.as_str(), "warning");
    assert!(message.style_ref().is_plain());

    let updated = message.style_ref().bold().fg(Color::Yellow).on(Color::Blue);
    *message.style_ref_mut() = updated;

    assert_eq!(message.as_str(), "warning");
    assert!(!message.style_ref().is_plain());

    let rendered = message.to_string();
    assert!(rendered.contains("warning"));
    assert!(rendered.contains("\x1b["));
    assert!(rendered.ends_with("\x1b[0m"));
}

#[test]
fn hyperlink_preserves_visible_text_and_exposes_url() {
    let linked = Color::Cyan
        .underline()
        .paint("nu-ansi-term docs")
        .hyperlink("https://docs.rs/nu-ansi-term");

    assert_eq!(linked.as_str(), "nu-ansi-term docs");
    assert_eq!(linked.url_string(), Some("https://docs.rs/nu-ansi-term"));

    let rendered = linked.to_string();
    assert!(rendered.contains("nu-ansi-term docs"));
    assert!(rendered.contains("https://docs.rs/nu-ansi-term"));
    assert!(rendered.contains("\x1b]8;;"));
}

#[test]
fn unlinked_strings_report_no_url_and_can_be_mutated_after_creation() {
    let mut plain = Style::new().italic().paint("local text");

    assert_eq!(plain.as_str(), "local text");
    assert_eq!(plain.url_string(), None);
    assert!(!plain.style_ref().is_plain());

    *plain.style_ref_mut() = Style::new();

    assert_eq!(plain.as_str(), "local text");
    assert_eq!(plain.url_string(), None);
    assert!(plain.style_ref().is_plain());
    assert_eq!(plain.to_string(), "local text");
}