use nu_ansi_term::{AnsiByteStrings, AnsiStrings, AnsiGenericString, Style, Color};

#[test]
fn test_ansi_byte_strings_basic_creation() {
    let style1 = Style::default();
    let style2 = Color::Red.bold();
    let style3 = Color::Blue.italic();

    let s1: AnsiGenericString<'_, [u8]> = style1.paint(b"hello " as &[u8]);
    let s2: AnsiGenericString<'_, [u8]> = style2.paint(b"world" as &[u8]);
    let s3: AnsiGenericString<'_, [u8]> = style3.paint(b"!" as &[u8]);

    let strings_vec = vec![s1, s2, s3];
    let ansi_strings = AnsiByteStrings(&strings_vec);

    let mut output: Vec<u8> = Vec::new();
    let result = ansi_strings.write_to(&mut output);
    assert!(result.is_ok());
    assert!(!output.is_empty());

    // The output should contain our raw text bytes
    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.contains("hello "));
    assert!(output_str.contains("world"));
    assert!(output_str.contains("!"));

    // Check that ANSI escape sequences are present for styled parts
    // ESC character is 0x1B
    assert!(output.contains(&0x1B));

    // Verify the total output length is greater than just the raw text
    // "hello world!" = 12 bytes, plus ANSI codes
    assert!(output.len() > 12);

    // Verify we can write to the same buffer again
    let mut output2: Vec<u8> = Vec::new();
    let s4: AnsiGenericString<'_, [u8]> = Style::default().paint(b"plain" as &[u8]);
    let strings_vec2 = vec![s4];
    let ansi_strings2 = AnsiByteStrings(&strings_vec2);
    let result2 = ansi_strings2.write_to(&mut output2);
    assert!(result2.is_ok());
    // Plain text with default style should have no escape codes
    assert_eq!(output2, b"plain");
}

#[test]
fn test_ansi_byte_strings_empty_collection() {
    let strings_vec: Vec<AnsiGenericString<'_, [u8]>> = vec![];
    let ansi_strings = AnsiByteStrings(&strings_vec);

    let mut output: Vec<u8> = Vec::new();
    let result = ansi_strings.write_to(&mut output);
    assert!(result.is_ok());
    assert_eq!(output.len(), 0);
    assert!(output.is_empty());

    // Writing to a pre-filled buffer should not corrupt existing content
    let mut prefilled: Vec<u8> = b"existing".to_vec();
    let original_len = prefilled.len();
    let empty_vec: Vec<AnsiGenericString<'_, [u8]>> = vec![];
    let empty_strings = AnsiByteStrings(&empty_vec);
    let result2 = empty_strings.write_to(&mut prefilled);
    assert!(result2.is_ok());
    assert_eq!(prefilled.len(), original_len);
    assert_eq!(&prefilled, b"existing");
}

#[test]
fn test_ansi_byte_strings_multiple_styles_write() {
    let bold = Style::new().bold();
    let italic = Style::new().italic();
    let underline = Style::new().underline();
    let dimmed = Style::new().dimmed();

    let s1: AnsiGenericString<'_, [u8]> = bold.paint(b"BOLD" as &[u8]);
    let s2: AnsiGenericString<'_, [u8]> = italic.paint(b"ITALIC" as &[u8]);
    let s3: AnsiGenericString<'_, [u8]> = underline.paint(b"UNDER" as &[u8]);
    let s4: AnsiGenericString<'_, [u8]> = dimmed.paint(b"DIM" as &[u8]);

    let strings_vec = vec![s1, s2, s3, s4];
    let ansi_strings = AnsiByteStrings(&strings_vec);

    let mut output: Vec<u8> = Vec::new();
    let result = ansi_strings.write_to(&mut output);
    assert!(result.is_ok());

    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.contains("BOLD"));
    assert!(output_str.contains("ITALIC"));
    assert!(output_str.contains("UNDER"));
    assert!(output_str.contains("DIM"));

    // Each styled segment should have escape sequences
    // Count ESC bytes - at least one per styled segment (opening) plus resets
    let esc_count = output.iter().filter(|&&b| b == 0x1B).count();
    assert!(esc_count >= 4, "Expected at least 4 ESC sequences, got {}", esc_count);

    // Total raw text is "BOLDITALICUNDERDIM" = 18 bytes
    assert!(output.len() > 18);
    // Verify the output is valid UTF-8 since ANSI codes are ASCII
    assert!(std::str::from_utf8(&output).is_ok());
}

#[test]
fn test_ansi_byte_strings_with_color_foreground_background() {
    let fg_style = Color::Green.normal();
    let bg_style = Style::new().on(Color::Yellow);
    let combined = Color::Cyan.on(Color::Magenta).bold();

    let s1: AnsiGenericString<'_, [u8]> = fg_style.paint(b"green-fg" as &[u8]);
    let s2: AnsiGenericString<'_, [u8]> = bg_style.paint(b"yellow-bg" as &[u8]);
    let s3: AnsiGenericString<'_, [u8]> = combined.paint(b"combo" as &[u8]);

    let strings_vec = vec![s1, s2, s3];
    let ansi_strings = AnsiByteStrings(&strings_vec);

    let mut output: Vec<u8> = Vec::new();
    let result = ansi_strings.write_to(&mut output);
    assert!(result.is_ok());

    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.contains("green-fg"));
    assert!(output_str.contains("yellow-bg"));
    assert!(output_str.contains("combo"));

    // Check for color codes - green foreground is 32
    assert!(output_str.contains("32"));
    // Yellow background is 43
    assert!(output_str.contains("43"));
    // Cyan foreground is 36
    assert!(output_str.contains("36"));
}

#[test]
fn test_ansi_byte_strings_single_element() {
    let style = Color::Red.underline().bold();
    let s1: AnsiGenericString<'_, [u8]> = style.paint(b"single" as &[u8]);

    let strings_vec = vec![s1];
    let ansi_strings = AnsiByteStrings(&strings_vec);

    let mut output: Vec<u8> = Vec::new();
    let result = ansi_strings.write_to(&mut output);
    assert!(result.is_ok());

    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.contains("single"));

    // Should contain reset sequence \x1b[0m at the end
    assert!(output_str.contains("\x1b[0m"));

    // Should start with ESC[
    assert_eq!(output[0], 0x1B);
    assert_eq!(output[1], b'[');

    // The raw text "single" is 6 bytes, output must be longer due to ANSI codes
    assert!(output.len() > 6);

    // Verify it ends with the reset sequence bytes
    let end = &output[output.len() - 4..];
    assert_eq!(end, b"\x1b[0m");
}

#[test]
fn test_ansi_byte_strings_adjacent_same_style_optimization() {
    // When adjacent strings share the same style, the library may optimize
    // by not emitting redundant reset/re-open sequences
    let style = Color::Blue.bold();

    let s1: AnsiGenericString<'_, [u8]> = style.paint(b"part1" as &[u8]);
    let s2: AnsiGenericString<'_, [u8]> = style.paint(b"part2" as &[u8]);
    let s3: AnsiGenericString<'_, [u8]> = style.paint(b"part3" as &[u8]);

    let strings_vec = vec![s1, s2, s3];
    let ansi_strings = AnsiByteStrings(&strings_vec);

    let mut output_combined: Vec<u8> = Vec::new();
    let result = ansi_strings.write_to(&mut output_combined);
    assert!(result.is_ok());

    let combined_str = String::from_utf8_lossy(&output_combined);
    assert!(combined_str.contains("part1"));
    assert!(combined_str.contains("part2"));
    assert!(combined_str.contains("part3"));

    // Now write them individually and compare
    let s1_single: AnsiGenericString<'_, [u8]> = style.paint(b"part1" as &[u8]);
    let single_vec = vec![s1_single];
    let single_strings = AnsiByteStrings(&single_vec);
    let mut output_single: Vec<u8> = Vec::new();
    let _ = single_strings.write_to(&mut output_single);

    // Combined output should be shorter than 3x individual outputs
    // because adjacent same-style segments can skip intermediate resets
    assert!(output_combined.len() < output_single.len() * 3);

    // But combined must still contain all text
    assert!(output_combined.len() >= 15); // "part1part2part3" = 15 bytes minimum
}

#[test]
fn test_ansi_byte_strings_binary_data() {
    // Test with non-UTF8 binary data
    let style = Color::Red.normal();
    let binary_data: &[u8] = &[0x00, 0x01, 0xFF, 0xFE, 0x80, 0x7F];

    let s1: AnsiGenericString<'_, [u8]> = style.paint(binary_data);
    let strings_vec = vec![s1];
    let ansi_strings = AnsiByteStrings(&strings_vec);

    let mut output: Vec<u8> = Vec::new();
    let result = ansi_strings.write_to(&mut output);
    assert!(result.is_ok());

    // Output should contain our binary data somewhere in the middle
    // between the ANSI prefix and suffix
    let has_binary = output.windows(binary_data.len()).any(|w| w == binary_data);
    assert!(has_binary, "Output should contain the original binary data");

    // Output should be longer than just the binary data
    assert!(output.len() > binary_data.len());

    // Should still have ESC sequences
    assert!(output.contains(&0x1B));

    // Verify the ANSI prefix for red (color code 31)
    let prefix_str = String::from_utf8_lossy(&output[..10]);
    assert!(prefix_str.contains("31"), "Should contain red color code 31");
}

#[test]
fn test_ansi_byte_strings_comparison_with_ansi_strings() {
    // Verify that AnsiByteStrings and AnsiStrings produce equivalent output
    // for the same ASCII text and style
    let style = Color::Green.bold();
    let text = "hello";

    // Using AnsiStrings (text) — AnsiStrings does not have write_to for str,
    // so use Display (format!) to get the output as a String, then convert to bytes.
    let text_s = style.paint(text);
    let text_vec = vec![text_s];
    let text_ansi = AnsiStrings(&text_vec);
    let text_output: Vec<u8> = format!("{}", text_ansi).into_bytes();

    // Using AnsiByteStrings (bytes)
    let byte_s: AnsiGenericString<'_, [u8]> = style.paint(text.as_bytes());
    let byte_vec = vec![byte_s];
    let byte_ansi = AnsiByteStrings(&byte_vec);
    let mut byte_output: Vec<u8> = Vec::new();
    let _ = byte_ansi.write_to(&mut byte_output);

    // For ASCII text, both should produce identical output
    assert_eq!(text_output, byte_output);
    assert!(!text_output.is_empty());
    assert!(!byte_output.is_empty());

    // Both should contain the text
    assert!(text_output.windows(5).any(|w| w == b"hello"));
    assert!(byte_output.windows(5).any(|w| w == b"hello"));

    // Both should have same length
    assert_eq!(text_output.len(), byte_output.len());
}