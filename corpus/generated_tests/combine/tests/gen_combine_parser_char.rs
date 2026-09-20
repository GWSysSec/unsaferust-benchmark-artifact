use combine::Parser;
use combine::parser::char::{crlf, hex_digit, lower, newline, oct_digit, string_cmp, tab, upper};

#[test]
fn test_char_newline() {
    let mut p = newline();
    let (out, rest) = p.parse("\nabc").unwrap();
    assert_eq!(out, '\n');
    assert_eq!(rest, "abc");

    let r = p.parse("abc");
    assert_eq!(r.is_err(), true);

    let r2 = p.parse("\rabc");
    assert_eq!(r2.is_err(), true);

    let (out2, rest2) = p.parse("\n").unwrap();
    assert_eq!(out2, '\n');
    assert_eq!(rest2, "");

    let r3 = p.parse(" \n");
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse("");
    assert_eq!(r4.is_err(), true);

    let (out3, rest3) = p.parse("\n\n").unwrap();
    assert_eq!(out3, '\n');
    assert_eq!(rest3, "\n");
}

#[test]
fn test_char_crlf() {
    let mut p = crlf();
    let (out, rest) = p.parse("\r\nabc").unwrap();
    assert_eq!(out, '\n');
    assert_eq!(rest, "abc");

    let r = p.parse("\nabc");
    assert_eq!(r.is_err(), true);

    let r2 = p.parse("abc");
    assert_eq!(r2.is_err(), true);

    let (out2, rest2) = p.parse("\r\n").unwrap();
    assert_eq!(out2, '\n');
    assert_eq!(rest2, "");

    let r3 = p.parse("");
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse("\r");
    assert_eq!(r4.is_err(), true);

    let (out3, rest3) = p.parse("\r\n\r\n").unwrap();
    assert_eq!(out3, '\n');
    assert_eq!(rest3, "\r\n");
}

#[test]
fn test_char_tab() {
    let mut p = tab();
    let (out, rest) = p.parse("\tabc").unwrap();
    assert_eq!(out, '\t');
    assert_eq!(rest, "abc");

    let r = p.parse("abc");
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(" \t");
    assert_eq!(r2.is_err(), true);

    let r3 = p.parse("\n");
    assert_eq!(r3.is_err(), true);

    let (out2, rest2) = p.parse("\t").unwrap();
    assert_eq!(out2, '\t');
    assert_eq!(rest2, "");

    let r4 = p.parse("");
    assert_eq!(r4.is_err(), true);

    let (out3, rest3) = p.parse("\t\t").unwrap();
    assert_eq!(out3, '\t');
    assert_eq!(rest3, "\t");
}

#[test]
fn test_char_upper() {
    let mut p = upper();
    let (out, rest) = p.parse("Abc").unwrap();
    assert_eq!(out, 'A');
    assert_eq!(rest, "bc");

    let (out2, rest2) = p.parse("Z!").unwrap();
    assert_eq!(out2, 'Z');
    assert_eq!(rest2, "!");

    let r = p.parse("abc");
    assert_eq!(r.is_err(), true);

    let r2 = p.parse("123");
    assert_eq!(r2.is_err(), true);

    let (out3, rest3) = p.parse("M").unwrap();
    assert_eq!(out3, 'M');
    assert_eq!(rest3, "");

    let r3 = p.parse("");
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse("!ABC");
    assert_eq!(r4.is_err(), true);
}

#[test]
fn test_char_lower() {
    let mut p = lower();
    let (out, rest) = p.parse("abc").unwrap();
    assert_eq!(out, 'a');
    assert_eq!(rest, "bc");

    let (out2, rest2) = p.parse("z!").unwrap();
    assert_eq!(out2, 'z');
    assert_eq!(rest2, "!");

    let r = p.parse("ABC");
    assert_eq!(r.is_err(), true);

    let r2 = p.parse("123");
    assert_eq!(r2.is_err(), true);

    let (out3, rest3) = p.parse("m").unwrap();
    assert_eq!(out3, 'm');
    assert_eq!(rest3, "");

    let r3 = p.parse("");
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse("!abc");
    assert_eq!(r4.is_err(), true);
}

#[test]
fn test_char_oct_digit() {
    let mut p = oct_digit();
    let (out, rest) = p.parse("7abc").unwrap();
    assert_eq!(out, '7');
    assert_eq!(rest, "abc");

    let (out2, rest2) = p.parse("0!").unwrap();
    assert_eq!(out2, '0');
    assert_eq!(rest2, "!");

    let r = p.parse("8");
    assert_eq!(r.is_err(), true);

    let r2 = p.parse("9");
    assert_eq!(r2.is_err(), true);

    let r3 = p.parse("a");
    assert_eq!(r3.is_err(), true);

    let (out3, rest3) = p.parse("3").unwrap();
    assert_eq!(out3, '3');
    assert_eq!(rest3, "");

    let r4 = p.parse("");
    assert_eq!(r4.is_err(), true);

    let (out4, rest4) = p.parse("5xx").unwrap();
    assert_eq!(out4, '5');
    assert_eq!(rest4, "xx");
}

#[test]
fn test_char_hex_digit() {
    let mut p = hex_digit();
    let (out, rest) = p.parse("Fxy").unwrap();
    assert_eq!(out, 'F');
    assert_eq!(rest, "xy");

    let (out2, rest2) = p.parse("a!").unwrap();
    assert_eq!(out2, 'a');
    assert_eq!(rest2, "!");

    let (out3, rest3) = p.parse("9z").unwrap();
    assert_eq!(out3, '9');
    assert_eq!(rest3, "z");

    let r = p.parse("g");
    assert_eq!(r.is_err(), true);

    let r2 = p.parse("G");
    assert_eq!(r2.is_err(), true);

    let r3 = p.parse("!");
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse("");
    assert_eq!(r4.is_err(), true);

    let (out4, rest4) = p.parse("0abc").unwrap();
    assert_eq!(out4, '0');
    assert_eq!(rest4, "abc");
}

#[test]
fn test_string_cmp_case_insensitive() {
    let mut p = string_cmp("hello", |a: char, b: char| a.eq_ignore_ascii_case(&b));
    let (out, rest) = p.parse("HELLO world").unwrap();
    assert_eq!(out, "hello");
    assert_eq!(rest, " world");

    let (out2, rest2) = p.parse("Hello!").unwrap();
    assert_eq!(out2, "hello");
    assert_eq!(rest2, "!");

    let (out3, rest3) = p.parse("hello").unwrap();
    assert_eq!(out3, "hello");
    assert_eq!(rest3, "");

    let r = p.parse("help me");
    assert_eq!(r.is_err(), true);

    let r2 = p.parse("");
    assert_eq!(r2.is_err(), true);

    let r3 = p.parse("hell");
    assert_eq!(r3.is_err(), true);

    let mut p2 = string_cmp("abc", |a: char, b: char| a == b);
    let (out4, rest4) = p2.parse("abcdef").unwrap();
    assert_eq!(out4, "abc");
    assert_eq!(rest4, "def");

    let r4 = p2.parse("ABC");
    assert_eq!(r4.is_err(), true);
}