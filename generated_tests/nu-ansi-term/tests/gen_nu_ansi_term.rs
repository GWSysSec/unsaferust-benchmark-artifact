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


    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.contains("hello "));
    assert!(output_str.contains("world"));
    assert!(output_str.contains("!"));



    assert!(output.contains(&0x1B));



    assert!(output.len() > 12);


    let mut output2: Vec<u8> = Vec::new();
    let s4: AnsiGenericString<'_, [u8]> = Style::default().paint(b"plain" as &[u8]);
    let strings_vec2 = vec![s4];
    let ansi_strings2 = AnsiByteStrings(&strings_vec2);
    let result2 = ansi_strings2.write_to(&mut output2);
    assert!(result2.is_ok());

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



    let esc_count = output.iter().filter(|&&b| b == 0x1B).count();
    assert!(esc_count >= 4, "Expected at least 4 ESC sequences, got {}", esc_count);


    assert!(output.len() > 18);

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


    assert!(output_str.contains("32"));

    assert!(output_str.contains("43"));

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


    assert!(output_str.contains("\x1b[0m"));


    assert_eq!(output[0], 0x1B);
    assert_eq!(output[1], b'[');


    assert!(output.len() > 6);


    let end = &output[output.len() - 4..];
    assert_eq!(end, b"\x1b[0m");
}

#[test]
fn test_ansi_byte_strings_adjacent_same_style_optimization() {


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


    let s1_single: AnsiGenericString<'_, [u8]> = style.paint(b"part1" as &[u8]);
    let single_vec = vec![s1_single];
    let single_strings = AnsiByteStrings(&single_vec);
    let mut output_single: Vec<u8> = Vec::new();
    let _ = single_strings.write_to(&mut output_single);



    assert!(output_combined.len() < output_single.len() * 3);


    assert!(output_combined.len() >= 15);
}

#[test]
fn test_ansi_byte_strings_binary_data() {

    let style = Color::Red.normal();
    let binary_data: &[u8] = &[0x00, 0x01, 0xFF, 0xFE, 0x80, 0x7F];

    let s1: AnsiGenericString<'_, [u8]> = style.paint(binary_data);
    let strings_vec = vec![s1];
    let ansi_strings = AnsiByteStrings(&strings_vec);

    let mut output: Vec<u8> = Vec::new();
    let result = ansi_strings.write_to(&mut output);
    assert!(result.is_ok());



    let has_binary = output.windows(binary_data.len()).any(|w| w == binary_data);
    assert!(has_binary, "Output should contain the original binary data");


    assert!(output.len() > binary_data.len());


    assert!(output.contains(&0x1B));


    let prefix_str = String::from_utf8_lossy(&output[..10]);
    assert!(prefix_str.contains("31"), "Should contain red color code 31");
}

#[test]
fn test_ansi_byte_strings_comparison_with_ansi_strings() {


    let style = Color::Green.bold();
    let text = "hello";



    let text_s = style.paint(text);
    let text_vec = vec![text_s];
    let text_ansi = AnsiStrings(&text_vec);
    let text_output: Vec<u8> = format!("{}", text_ansi).into_bytes();


    let byte_s: AnsiGenericString<'_, [u8]> = style.paint(text.as_bytes());
    let byte_vec = vec![byte_s];
    let byte_ansi = AnsiByteStrings(&byte_vec);
    let mut byte_output: Vec<u8> = Vec::new();
    let _ = byte_ansi.write_to(&mut byte_output);


    assert_eq!(text_output, byte_output);
    assert!(!text_output.is_empty());
    assert!(!byte_output.is_empty());


    assert!(text_output.windows(5).any(|w| w == b"hello"));
    assert!(byte_output.windows(5).any(|w| w == b"hello"));


    assert_eq!(text_output.len(), byte_output.len());
}