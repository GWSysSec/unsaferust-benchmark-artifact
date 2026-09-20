use nu_ansi_term::{AnsiByteString, AnsiByteStrings, Color, Style};

#[test]
fn ansi_byte_strings_write_mixed_styles_in_order() {
    let red = Color::Red.paint(&b"red"[..]);
    let bold_green = Color::Green.bold().paint(&b"green"[..]);
    let plain = Style::new().paint(&b"plain"[..]);

    let parts = [red, bold_green, plain];
    let grouped = AnsiByteStrings(&parts);

    let mut output = Vec::new();
    grouped
        .write_to(&mut output)
        .expect("writing grouped ANSI byte strings should succeed");

    let rendered = String::from_utf8(output.clone()).expect("ANSI output should be valid UTF-8");

    assert!(rendered.contains("red"));
    assert!(rendered.contains("green"));
    assert!(rendered.contains("plain"));
    assert!(
        rendered.find("red").unwrap() < rendered.find("green").unwrap(),
        "red segment should be written before green segment"
    );
    assert!(
        rendered.find("green").unwrap() < rendered.find("plain").unwrap(),
        "green segment should be written before plain segment"
    );
    assert!(
        output.windows(b"\x1b[31m".len()).any(|w| w == b"\x1b[31m"),
        "red foreground ANSI prefix should be emitted"
    );
    assert!(
        output.windows(b"\x1b[".len()).any(|w| w == b"\x1b["),
        "styled grouped output should contain ANSI escape sequences"
    );
}

#[test]
fn ansi_byte_strings_handles_empty_and_plain_byte_segments() {
    let empty_styled = Color::Blue.underline().paint(&b""[..]);
    let plain_left = Style::new().paint(&b"left"[..]);
    let plain_right = Style::new().paint(&b"right"[..]);

    let parts = [plain_left, empty_styled, plain_right];
    let grouped = AnsiByteStrings(&parts);

    let mut output = Vec::new();
    grouped
        .write_to(&mut output)
        .expect("writing byte strings with an empty segment should succeed");

    let rendered = String::from_utf8(output).expect("plain byte output should be UTF-8");

    assert!(rendered.contains("left"));
    assert!(rendered.contains("right"));
    assert!(
        rendered.find("left").unwrap() < rendered.find("right").unwrap(),
        "non-empty byte segments should preserve their relative order"
    );
    assert_eq!(rendered.matches("left").count(), 1);
    assert_eq!(rendered.matches("right").count(), 1);
}

#[test]
fn ansi_byte_strings_empty_group_writes_nothing() {
    let parts: [AnsiByteString<'_>; 0] = [];
    let grouped = AnsiByteStrings(&parts);

    let mut output = Vec::new();
    grouped
        .write_to(&mut output)
        .expect("writing an empty AnsiByteStrings group should succeed");

    assert!(output.is_empty());
    assert_eq!(output.len(), 0);
}