use combine::Parser;
use combine::parser::byte::{byte, crlf, hex_digit, lower, newline, oct_digit, tab, upper};

#[test]
fn test_byte_matches_specific_byte() {
    let mut p = byte(b'a');
    let (out, rest) = p.parse(&b"abc"[..]).unwrap();
    assert_eq!(out, b'a');
    assert_eq!(rest, &b"bc"[..]);

    let r = p.parse(&b"xyz"[..]);
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(&b""[..]);
    assert_eq!(r2.is_err(), true);

    let (out2, rest2) = p.parse(&b"a"[..]).unwrap();
    assert_eq!(out2, b'a');
    assert_eq!(rest2, &b""[..]);

    let mut seq = (byte(b'h'), byte(b'i'), byte(b'!'));
    let ((a, b, c), rest3) = seq.parse(&b"hi!end"[..]).unwrap();
    assert_eq!(a, b'h');
    assert_eq!(b, b'i');
    assert_eq!(c, b'!');
    assert_eq!(rest3, &b"end"[..]);

    let r3 = seq.parse(&b"hi?"[..]);
    assert_eq!(r3.is_err(), true);
}

#[test]
fn test_newline_byte_parser() {
    let mut p = newline();
    let (out, rest) = p.parse(&b"\nabc"[..]).unwrap();
    assert_eq!(out, b'\n');
    assert_eq!(rest, &b"abc"[..]);

    let r = p.parse(&b"abc"[..]);
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(&b"\rabc"[..]);
    assert_eq!(r2.is_err(), true);

    let (out2, rest2) = p.parse(&b"\n"[..]).unwrap();
    assert_eq!(out2, b'\n');
    assert_eq!(rest2, &b""[..]);

    let r3 = p.parse(&b" \n"[..]);
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse(&b""[..]);
    assert_eq!(r4.is_err(), true);

    let r5 = p.parse(&b"\t\n"[..]);
    assert_eq!(r5.is_err(), true);
}

#[test]
fn test_crlf_byte_parser() {
    let mut p = crlf();
    let (out, rest) = p.parse(&b"\r\nabc"[..]).unwrap();
    assert_eq!(out, b'\n');
    assert_eq!(rest, &b"abc"[..]);

    let r = p.parse(&b"\nabc"[..]);
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(&b"abc"[..]);
    assert_eq!(r2.is_err(), true);

    let (out2, rest2) = p.parse(&b"\r\n"[..]).unwrap();
    assert_eq!(out2, b'\n');
    assert_eq!(rest2, &b""[..]);

    let r4 = p.parse(&b""[..]);
    assert_eq!(r4.is_err(), true);

    let r5 = p.parse(&b"\r"[..]);
    assert_eq!(r5.is_err(), true);

    let (out3, rest3) = p.parse(&b"\r\n\r\n"[..]).unwrap();
    assert_eq!(out3, b'\n');
    assert_eq!(rest3, &b"\r\n"[..]);
}

#[test]
fn test_tab_byte_parser() {
    let mut p = tab();
    let (out, rest) = p.parse(&b"\tabc"[..]).unwrap();
    assert_eq!(out, b'\t');
    assert_eq!(rest, &b"abc"[..]);

    let r = p.parse(&b"abc"[..]);
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(&b" \t"[..]);
    assert_eq!(r2.is_err(), true);

    let r3 = p.parse(&b"\n"[..]);
    assert_eq!(r3.is_err(), true);

    let (out2, rest2) = p.parse(&b"\t"[..]).unwrap();
    assert_eq!(out2, b'\t');
    assert_eq!(rest2, &b""[..]);

    let r4 = p.parse(&b""[..]);
    assert_eq!(r4.is_err(), true);

    let (out3, rest3) = p.parse(&b"\t\t"[..]).unwrap();
    assert_eq!(out3, b'\t');
    assert_eq!(rest3, &b"\t"[..]);
}

#[test]
fn test_upper_byte_parser() {
    let mut p = upper();
    let (out, rest) = p.parse(&b"Abc"[..]).unwrap();
    assert_eq!(out, b'A');
    assert_eq!(rest, &b"bc"[..]);

    let (out2, rest2) = p.parse(&b"Z!"[..]).unwrap();
    assert_eq!(out2, b'Z');
    assert_eq!(rest2, &b"!"[..]);

    let r = p.parse(&b"abc"[..]);
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(&b"123"[..]);
    assert_eq!(r2.is_err(), true);

    let (out3, rest3) = p.parse(&b"M"[..]).unwrap();
    assert_eq!(out3, b'M');
    assert_eq!(rest3, &b""[..]);

    let r3 = p.parse(&b""[..]);
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse(&b"!ABC"[..]);
    assert_eq!(r4.is_err(), true);
}

#[test]
fn test_lower_byte_parser() {
    let mut p = lower();
    let (out, rest) = p.parse(&b"abc"[..]).unwrap();
    assert_eq!(out, b'a');
    assert_eq!(rest, &b"bc"[..]);

    let (out2, rest2) = p.parse(&b"z!"[..]).unwrap();
    assert_eq!(out2, b'z');
    assert_eq!(rest2, &b"!"[..]);

    let r = p.parse(&b"ABC"[..]);
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(&b"123"[..]);
    assert_eq!(r2.is_err(), true);

    let (out3, rest3) = p.parse(&b"m"[..]).unwrap();
    assert_eq!(out3, b'm');
    assert_eq!(rest3, &b""[..]);

    let r3 = p.parse(&b""[..]);
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse(&b"!abc"[..]);
    assert_eq!(r4.is_err(), true);
}

#[test]
fn test_oct_digit_byte_parser() {
    let mut p = oct_digit();
    let (out, rest) = p.parse(&b"7abc"[..]).unwrap();
    assert_eq!(out, b'7');
    assert_eq!(rest, &b"abc"[..]);

    let (out2, rest2) = p.parse(&b"0!"[..]).unwrap();
    assert_eq!(out2, b'0');
    assert_eq!(rest2, &b"!"[..]);

    let r = p.parse(&b"8"[..]);
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(&b"9"[..]);
    assert_eq!(r2.is_err(), true);

    let r3 = p.parse(&b"a"[..]);
    assert_eq!(r3.is_err(), true);

    let (out3, rest3) = p.parse(&b"3"[..]).unwrap();
    assert_eq!(out3, b'3');
    assert_eq!(rest3, &b""[..]);

    let r4 = p.parse(&b""[..]);
    assert_eq!(r4.is_err(), true);

    let (out4, rest4) = p.parse(&b"5xx"[..]).unwrap();
    assert_eq!(out4, b'5');
    assert_eq!(rest4, &b"xx"[..]);
}

#[test]
fn test_hex_digit_byte_parser() {
    let mut p = hex_digit();
    let (out, rest) = p.parse(&b"Fxy"[..]).unwrap();
    assert_eq!(out, b'F');
    assert_eq!(rest, &b"xy"[..]);

    let (out2, rest2) = p.parse(&b"a!"[..]).unwrap();
    assert_eq!(out2, b'a');
    assert_eq!(rest2, &b"!"[..]);

    let (out3, rest3) = p.parse(&b"9z"[..]).unwrap();
    assert_eq!(out3, b'9');
    assert_eq!(rest3, &b"z"[..]);

    let r = p.parse(&b"g"[..]);
    assert_eq!(r.is_err(), true);

    let r2 = p.parse(&b"G"[..]);
    assert_eq!(r2.is_err(), true);

    let r3 = p.parse(&b"!"[..]);
    assert_eq!(r3.is_err(), true);

    let r4 = p.parse(&b""[..]);
    assert_eq!(r4.is_err(), true);

    let (out4, rest4) = p.parse(&b"0abc"[..]).unwrap();
    assert_eq!(out4, b'0');
    assert_eq!(rest4, &b"abc"[..]);
}