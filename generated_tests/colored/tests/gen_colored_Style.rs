#![cfg(not(feature = "no-color"))]

use colored::*;

#[test]
fn style_remove_basic() {
    let mut style: Style = Styles::Bold | Styles::Italic | Styles::Underline;

    assert_eq!(style.contains(Styles::Bold), true);
    assert_eq!(style.contains(Styles::Italic), true);
    assert_eq!(style.contains(Styles::Underline), true);
    assert_eq!(style.contains(Styles::Blink), false);
    assert_eq!(style.contains(Styles::Dimmed), false);

    style.remove(Styles::Italic);

    assert_eq!(style.contains(Styles::Bold), true);
    assert_eq!(style.contains(Styles::Italic), false);
    assert_eq!(style.contains(Styles::Underline), true);

    style.remove(Styles::Bold);
    assert_eq!(style.contains(Styles::Bold), false);
    assert_eq!(style.contains(Styles::Underline), true);


    style.remove(Styles::Blink);
    assert_eq!(style.contains(Styles::Underline), true);
    assert_eq!(style.contains(Styles::Blink), false);

    style.remove(Styles::Underline);
    assert_eq!(style.contains(Styles::Underline), false);
}

#[test]
fn style_remove_all_then_verify_absence() {
    let mut style: Style =
        Styles::Bold | Styles::Dimmed | Styles::Italic | Styles::Underline | Styles::Blink;

    let all = [
        Styles::Bold,
        Styles::Dimmed,
        Styles::Italic,
        Styles::Underline,
        Styles::Blink,
    ];

    for s in all.iter() {
        assert_eq!(style.contains(*s), true);
    }

    assert_eq!(style.contains(Styles::Reversed), false);
    assert_eq!(style.contains(Styles::Hidden), false);
    assert_eq!(style.contains(Styles::Strikethrough), false);


    style.remove(Styles::Bold);
    assert_eq!(style.contains(Styles::Bold), false);
    assert_eq!(style.contains(Styles::Dimmed), true);

    style.remove(Styles::Dimmed);
    assert_eq!(style.contains(Styles::Dimmed), false);
    assert_eq!(style.contains(Styles::Italic), true);

    style.remove(Styles::Italic);
    style.remove(Styles::Underline);
    style.remove(Styles::Blink);

    for s in all.iter() {
        assert_eq!(style.contains(*s), false);
    }
}

#[test]
fn style_remove_independence_from_other_flags() {
    let mut a: Style = Styles::Bold | Styles::Italic;
    let b: Style = Styles::Bold | Styles::Italic;

    assert_eq!(a.contains(Styles::Bold), true);
    assert_eq!(b.contains(Styles::Bold), true);
    assert_eq!(a.contains(Styles::Italic), true);
    assert_eq!(b.contains(Styles::Italic), true);

    a.remove(Styles::Bold);

    assert_eq!(a.contains(Styles::Bold), false);
    assert_eq!(a.contains(Styles::Italic), true);

    assert_eq!(b.contains(Styles::Bold), true);
    assert_eq!(b.contains(Styles::Italic), true);
}