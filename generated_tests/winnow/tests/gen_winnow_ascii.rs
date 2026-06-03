use winnow::Parser;
use winnow::ascii::{
    alphanumeric0, alphanumeric1, escaped_transform, hex_digit0, newline, oct_digit0, tab, take_escaped,
};
use winnow::error::ContextError;
use winnow::token::one_of;

#[test]
fn test_newline_and_tab() {
    let mut input = "\nrest";
    let r: Result<char, ContextError> = newline(&mut input);
    assert_eq!(r.unwrap(), '\n');
    assert_eq!(input, "rest");

    let mut input2 = "\tafter";
    let r2: Result<char, ContextError> = tab(&mut input2);
    assert_eq!(r2.unwrap(), '\t');
    assert_eq!(input2, "after");

    let mut bad = "x\n";
    let r3: Result<char, ContextError> = newline(&mut bad);
    assert!(r3.is_err());
    assert_eq!(bad, "x\n");

    let mut bad2 = "x\t";
    let r4: Result<char, ContextError> = tab(&mut bad2);
    assert!(r4.is_err());
    assert_eq!(bad2, "x\t");
}

#[test]
fn test_hex_oct_alphanumeric_zero_variants() {

    let mut empty = "";
    let r: Result<&str, ContextError> = hex_digit0(&mut empty);
    assert_eq!(r.unwrap(), "");
    assert_eq!(empty, "");

    let mut hex = "DEADbeef-rest";
    let r2: Result<&str, ContextError> = hex_digit0(&mut hex);
    assert_eq!(r2.unwrap(), "DEADbeef");
    assert_eq!(hex, "-rest");

    let mut nothex = "zzz";
    let r3: Result<&str, ContextError> = hex_digit0(&mut nothex);
    assert_eq!(r3.unwrap(), "");
    assert_eq!(nothex, "zzz");


    let mut oct = "01234567xyz";
    let r4: Result<&str, ContextError> = oct_digit0(&mut oct);
    assert_eq!(r4.unwrap(), "01234567");
    assert_eq!(oct, "xyz");

    let mut nooct = "89";
    let r5: Result<&str, ContextError> = oct_digit0(&mut nooct);
    assert_eq!(r5.unwrap(), "");
    assert_eq!(nooct, "89");


    let mut alnum = "abc123!!";
    let r6: Result<&str, ContextError> = alphanumeric0(&mut alnum);
    assert_eq!(r6.unwrap(), "abc123");
    assert_eq!(alnum, "!!");

    let mut none = "...";
    let r7: Result<&str, ContextError> = alphanumeric0(&mut none);
    assert_eq!(r7.unwrap(), "");
    assert_eq!(none, "...");
}

#[test]
fn test_take_escaped_basic() {
    let mut parser = take_escaped(
        alphanumeric1::<&str, ContextError>,
        '\\',
        one_of(['n', 't', '\\', '"']),
    );
    let mut input = "abc\\ndef\"tail";
    let res: Result<&str, ContextError> = parser.parse_next(&mut input);
    assert_eq!(res.unwrap(), "abc\\ndef");
    assert_eq!(input, "\"tail");

    let mut input2 = "plain rest";
    let res2: Result<&str, ContextError> = parser.parse_next(&mut input2);
    assert_eq!(res2.unwrap(), "plain");
    assert_eq!(input2, " rest");

    let mut input3 = "xyz";
    let res3: Result<&str, ContextError> = parser.parse_next(&mut input3);
    assert_eq!(res3.unwrap(), "xyz");
    assert_eq!(input3, "");
}

#[test]
fn test_escaped_transform_full() {
    use winnow::combinator::alt;

    let mut parser = escaped_transform::<_, ContextError, _, _, _, _, String>(
        alphanumeric1,
        '\\',
        alt((
            "n".value("\n"),
            "t".value("\t"),
            "\\".value("\\"),
            "\"".value("\""),
        )),
    );

    let mut input = "hello\\nworld\\t!!!";
    let pre_len = input.len();
    assert_eq!(pre_len, 17);
    let r: Result<String, ContextError> = parser.parse_next(&mut input);
    let out = r.unwrap();
    assert_eq!(out, "hello\nworld\t");
    assert_eq!(input, "!!!");
    assert_ne!(out.len(), pre_len);
    assert_eq!(out.chars().filter(|c| *c == '\n').count(), 1);
    assert_eq!(out.chars().filter(|c| *c == '\t').count(), 1);

    let mut input2 = "abc\\\\def";
    let r2: Result<String, ContextError> = parser.parse_next(&mut input2);
    assert_eq!(r2.unwrap(), "abc\\def");
    assert_eq!(input2, "");

    let mut input3 = "noescapes!";
    let r3: Result<String, ContextError> = parser.parse_next(&mut input3);
    assert_eq!(r3.unwrap(), "noescapes");
    assert_eq!(input3, "!");
}