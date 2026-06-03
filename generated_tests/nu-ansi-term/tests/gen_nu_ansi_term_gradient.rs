use nu_ansi_term::gradient::{build_all_gradient_text, Gradient};
use nu_ansi_term::Rgb;

#[test]
fn gradient_text_basic_properties() {
    let start = Rgb::new(255, 0, 0);
    let end = Rgb::new(0, 0, 255);
    let fg = Gradient::new(start, end);
    let bg = Gradient::new(Rgb::new(0, 255, 0), Rgb::new(255, 255, 0));

    let text = "Hello, World!";
    let out = build_all_gradient_text(text, fg, bg);


    assert_ne!(out.len(), 0);

    assert!(out.len() > text.len());

    assert!(out.contains('\x1b'));

    for ch in text.chars() {
        assert!(out.contains(ch), "missing char {:?}", ch);
    }

    assert!(out.contains("\x1b[0m"));

    assert!(out.contains("38;2;"));

    assert!(out.contains("48;2;"));

    assert!(out.contains("255;0;0"));
}

#[test]
fn gradient_text_empty_string() {
    let fg = Gradient::new(Rgb::new(10, 20, 30), Rgb::new(200, 100, 50));
    let bg = Gradient::new(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
    let out = build_all_gradient_text("", fg, bg);

    assert!(!out.contains('H'));
    assert!(!out.contains('a'));
    assert!(out.len() < 32);

    let out2 = build_all_gradient_text("ABCDEFG", fg, bg);
    assert_ne!(out, out2);
    assert!(out2.len() > out.len());
    assert!(out2.contains('A'));
    assert!(out2.contains('G'));
    assert!(out2.contains('\x1b'));
    assert!(out2.contains("38;2;"));
}

#[test]
fn gradient_text_single_char_vs_multi() {
    let fg = Gradient::new(Rgb::new(255, 0, 0), Rgb::new(0, 0, 255));
    let bg = Gradient::new(Rgb::new(0, 255, 0), Rgb::new(255, 255, 0));

    let single = build_all_gradient_text("X", fg, bg);
    let multi = build_all_gradient_text("XXXXXXXX", fg, bg);

    assert!(single.contains('X'));
    assert_eq!(single.matches('X').count(), 1);
    assert_eq!(multi.matches('X').count(), 8);
    assert!(multi.len() > single.len());
    assert!(single.contains('\x1b'));
    assert!(multi.contains('\x1b'));

    assert!(multi.matches('\x1b').count() >= single.matches('\x1b').count());

    assert!(single.contains("38;2;"));
    assert!(multi.contains("48;2;"));

    assert!(single.contains("255;0;0"));
    assert!(multi.contains("255;0;0"));
}