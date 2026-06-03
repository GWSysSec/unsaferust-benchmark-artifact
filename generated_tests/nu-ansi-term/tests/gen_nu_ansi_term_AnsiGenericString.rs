use nu_ansi_term::{Style, Color, AnsiStrings, AnsiString, unstyle, unstyled_len, sub_string};
use std::io::Write;

#[test]
fn test_style_ref_mut_basic_modifications() {
    let style = Style::new().bold();
    let mut styled: AnsiString = style.paint("hello world");


    assert_eq!(styled.style_ref().is_bold, true);
    assert_eq!(styled.style_ref().is_underline, false);
    assert_eq!(styled.style_ref().is_italic, false);
    assert_eq!(styled.style_ref().is_dimmed, false);


    styled.style_ref_mut().is_underline = true;
    styled.style_ref_mut().is_italic = true;
    styled.style_ref_mut().is_bold = false;


    assert_eq!(styled.style_ref().is_bold, false);
    assert_eq!(styled.style_ref().is_underline, true);
    assert_eq!(styled.style_ref().is_italic, true);
    assert_eq!(styled.style_ref().is_dimmed, false);
}

#[test]
fn test_style_ref_mut_foreground_background() {
    let mut styled: AnsiString = Style::new().paint("colored text");


    assert_eq!(styled.style_ref().foreground, None);
    assert_eq!(styled.style_ref().background, None);


    styled.style_ref_mut().foreground = Some(Color::Red);
    styled.style_ref_mut().background = Some(Color::Blue);


    assert_eq!(styled.style_ref().foreground, Some(Color::Red));
    assert_eq!(styled.style_ref().background, Some(Color::Blue));


    let output = format!("{}", styled);
    assert!(output.contains("\x1b["));
    assert!(output.contains("colored text"));
}

#[test]
fn test_style_ref_mut_multiple_mutations_chain() {
    let mut styled: AnsiString = Style::default().paint("test string");


    assert_eq!(styled.style_ref().is_bold, false);
    assert_eq!(styled.style_ref().is_strikethrough, false);
    assert_eq!(styled.style_ref().is_hidden, false);
    assert_eq!(styled.style_ref().is_reverse, false);


    styled.style_ref_mut().is_bold = true;
    styled.style_ref_mut().is_strikethrough = true;
    styled.style_ref_mut().is_hidden = true;
    styled.style_ref_mut().is_reverse = true;

    assert_eq!(styled.style_ref().is_bold, true);
    assert_eq!(styled.style_ref().is_strikethrough, true);
    assert_eq!(styled.style_ref().is_hidden, true);
    assert_eq!(styled.style_ref().is_reverse, true);
}

#[test]
fn test_as_str_returns_inner_string() {
    let styled: AnsiString = Style::new().bold().paint("hello world");
    let inner: &str = styled.as_str();

    assert_eq!(inner, "hello world");
    assert_eq!(inner.len(), 11);
    assert!(inner.starts_with("hello"));
    assert!(inner.ends_with("world"));
    assert!(inner.contains(" "));
    assert_eq!(inner.chars().count(), 11);
    assert_eq!(inner.as_bytes()[0], b'h');
    assert_eq!(inner.as_bytes()[10], b'd');
}

#[test]
fn test_as_str_empty_and_special_strings() {
    let empty: AnsiString = Style::new().paint("");
    assert_eq!(empty.as_str(), "");
    assert_eq!(empty.as_str().len(), 0);

    let unicode: AnsiString = Style::new().paint("héllo wörld 🌍");
    let inner = unicode.as_str();
    assert_eq!(inner, "héllo wörld 🌍");
    assert!(inner.contains("🌍"));
    assert!(inner.starts_with("héllo"));

    let whitespace: AnsiString = Style::new().paint("  \t\n  ");
    assert_eq!(whitespace.as_str(), "  \t\n  ");
    assert_eq!(whitespace.as_str().trim(), "");
    assert_eq!(whitespace.as_str().len(), 6);
}

#[test]
fn test_as_str_with_various_styles() {
    let bold: AnsiString = Style::new().bold().paint("bold text");
    let italic: AnsiString = Style::new().italic().paint("italic text");
    let colored: AnsiString = Color::Green.paint("green text");


    assert_eq!(bold.as_str(), "bold text");
    assert_eq!(italic.as_str(), "italic text");
    assert_eq!(colored.as_str(), "green text");


    assert_eq!(bold.as_str().len(), 9);
    assert_eq!(italic.as_str().len(), 11);
    assert_eq!(colored.as_str().len(), 10);
}

#[test]
fn test_url_string_none_by_default() {
    let styled: AnsiString = Style::new().bold().paint("no url here");


    assert!(styled.url_string().is_none());
    assert_eq!(styled.url_string(), None);

    let plain: AnsiString = Style::default().paint("plain text");
    assert!(plain.url_string().is_none());

    let colored: AnsiString = Color::Red.paint("red text");
    assert!(colored.url_string().is_none());

    let complex: AnsiString = Style::new()
        .bold()
        .italic()
        .underline()
        .paint("complex style");
    assert!(complex.url_string().is_none());


    assert_eq!(styled.as_str(), "no url here");
    assert_eq!(plain.as_str(), "plain text");
    assert_eq!(colored.as_str(), "red text");
    assert_eq!(complex.as_str(), "complex style");
}

#[test]
fn test_style_ref_mut_with_unstyle_integration() {
    let mut s1: AnsiString = Style::new().bold().paint("hello ");
    let s2: AnsiString = Color::Blue.paint("world");


    s1.style_ref_mut().foreground = Some(Color::Red);
    s1.style_ref_mut().is_italic = true;


    assert_eq!(s1.style_ref().foreground, Some(Color::Red));
    assert_eq!(s1.style_ref().is_italic, true);
    assert_eq!(s1.style_ref().is_bold, true);


    let strings_vec = vec![s1, s2];
    let ansi_strings = AnsiStrings(&strings_vec);
    let plain = unstyle(&ansi_strings);

    assert_eq!(plain, "hello world");
    assert_eq!(unstyled_len(&ansi_strings), 11);
}

#[test]
fn test_as_str_and_style_ref_mut_combined_workflow() {
    let mut styled: AnsiString = Style::new().paint("mutable content");


    assert_eq!(styled.as_str(), "mutable content");
    assert_eq!(styled.style_ref().is_bold, false);
    assert_eq!(styled.style_ref().foreground, None);


    styled.style_ref_mut().is_bold = true;
    styled.style_ref_mut().foreground = Some(Color::Yellow);


    assert_eq!(styled.as_str(), "mutable content");
    assert_eq!(styled.as_str().len(), 15);


    assert_eq!(styled.style_ref().is_bold, true);
    assert_eq!(styled.style_ref().foreground, Some(Color::Yellow));
}

#[test]
fn test_sub_string_with_style_ref_mut() {
    let mut s1: AnsiString = Style::new().bold().paint("ABCDE");
    let s2: AnsiString = Color::Green.paint("FGHIJ");


    s1.style_ref_mut().is_underline = true;

    let strings_vec = vec![s1, s2];
    let ansi_strings = AnsiStrings(&strings_vec);


    let sub = sub_string(2, 6, &ansi_strings);


    let sub_ansi = AnsiStrings(&sub);
    let plain = unstyle(&sub_ansi);
    assert_eq!(plain, "CDEFGH");
    assert_eq!(unstyled_len(&sub_ansi), 6);
    assert_eq!(plain.len(), 6);
    assert!(plain.starts_with("CDE"));
    assert!(plain.ends_with("FGH"));
}

#[test]
fn test_write_to_after_style_ref_mut() {
    use nu_ansi_term::{AnsiByteStrings, AnsiByteString};

    let mut styled: AnsiString = Style::new().paint("write me");


    styled.style_ref_mut().is_bold = true;
    styled.style_ref_mut().foreground = Some(Color::Cyan);


    let style = *styled.style_ref();
    let byte_strings_vec: Vec<AnsiByteString> = vec![style.paint(b"write me" as &[u8])];
    let ansi_byte_strings = AnsiByteStrings(&byte_strings_vec);

    let mut buffer: Vec<u8> = Vec::new();
    ansi_byte_strings.write_to(&mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();


    assert!(output.contains("write me"));

    assert!(output.contains("\x1b["));

    assert!(output.contains("\x1b[0m"));

    assert!(output.len() > "write me".len());

    let plain_strings_vec: Vec<AnsiString> = vec![Style::new().bold().fg(Color::Cyan).paint("write me")];
    let plain_ansi = AnsiStrings(&plain_strings_vec);
    assert_eq!(unstyle(&plain_ansi), "write me");
}

#[test]
fn test_style_ref_mut_prefix_with_reset() {
    let mut styled: AnsiString = Style::new().bold().paint("reset test");


    assert_eq!(styled.style_ref().prefix_with_reset, false);


    styled.style_ref_mut().prefix_with_reset = true;


    assert_eq!(styled.style_ref().prefix_with_reset, true);
    assert_eq!(styled.style_ref().is_bold, true);
    assert_eq!(styled.as_str(), "reset test");


    let output = format!("{}", styled);
    assert!(output.contains("reset test"));
    assert!(output.contains("\x1b["));

    assert!(output.starts_with("\x1b[0m"));
    assert!(output.len() > "reset test".len());
}