use combine::Parser;
use combine::error::StringStreamError;
use combine::many1;
use combine::parser::char::{digit, letter};
use combine::parser::combinator::{
    and_then, any_send_partial_state, any_send_sync_partial_state, factory, flat_map, lazy,
    look_ahead, map_input, no_partial,
};

fn to_digit_result(c: char) -> Result<u32, StringStreamError> {
    Ok(c.to_digit(10).unwrap())
}

fn to_upper_result(c: char) -> Result<char, StringStreamError> {
    Ok(c.to_ascii_uppercase())
}

fn parse_u32(s: String) -> Result<u32, StringStreamError> {
    Ok(s.parse::<u32>().unwrap())
}

#[test]
fn test_look_ahead_does_not_consume_input() {
    let mut p = look_ahead(digit());
    let (out, rest) = p.parse("7abc").unwrap();
    assert_eq!(out, '7');
    assert_eq!(rest, "7abc");

    let r_err = p.parse("abc");
    assert_eq!(r_err.is_err(), true);

    let r_err2 = p.parse("");
    assert_eq!(r_err2.is_err(), true);

    let mut p2 = look_ahead(many1::<String, _, _>(digit()));
    let (out2, rest2) = p2.parse("12345xyz").unwrap();
    assert_eq!(out2, "12345");
    assert_eq!(rest2, "12345xyz");
    assert_eq!(out2.len(), 5);

    let (out3, rest3) = p2.parse("9").unwrap();
    assert_eq!(out3, "9");
    assert_eq!(rest3, "9");
}

#[test]
fn test_flat_map_transforms_output() {
    let mut p = flat_map(digit(), to_digit_result);
    let (out, rest) = p.parse("3xyz").unwrap();
    assert_eq!(out, 3u32);
    assert_eq!(rest, "xyz");

    let (out2, rest2) = p.parse("0!").unwrap();
    assert_eq!(out2, 0u32);
    assert_eq!(rest2, "!");

    let r_err = p.parse("abc");
    assert_eq!(r_err.is_err(), true);

    let r_err2 = p.parse("");
    assert_eq!(r_err2.is_err(), true);

    let (out3, rest3) = p.parse("9").unwrap();
    assert_eq!(out3, 9u32);
    assert_eq!(rest3, "");
}

#[test]
fn test_and_then_transforms_output() {
    let mut p = and_then(letter(), to_upper_result);
    let (out, rest) = p.parse("abc").unwrap();
    assert_eq!(out, 'A');
    assert_eq!(rest, "bc");

    let (out2, rest2) = p.parse("zoo").unwrap();
    assert_eq!(out2, 'Z');
    assert_eq!(rest2, "oo");

    let r_err = p.parse("123");
    assert_eq!(r_err.is_err(), true);

    let mut p2 = and_then(many1::<String, _, _>(digit()), parse_u32);
    let (out3, rest3) = p2.parse("42xyz").unwrap();
    assert_eq!(out3, 42u32);
    assert_eq!(rest3, "xyz");

    let (out4, rest4) = p2.parse("100").unwrap();
    assert_eq!(out4, 100u32);
    assert_eq!(rest4, "");
}

#[test]
fn test_no_partial_wraps_parser() {
    let mut p = no_partial(many1::<String, _, _>(digit()));
    let (out, rest) = p.parse("12345abc").unwrap();
    assert_eq!(out, "12345");
    assert_eq!(rest, "abc");
    assert_eq!(out.len(), 5);

    let (out2, rest2) = p.parse("9").unwrap();
    assert_eq!(out2, "9");
    assert_eq!(rest2, "");

    let r_err = p.parse("abc");
    assert_eq!(r_err.is_err(), true);

    let mut p2 = no_partial((digit(), digit(), digit()));
    let ((a, b, c), rest3) = p2.parse("123xyz").unwrap();
    assert_eq!(a, '1');
    assert_eq!(b, '2');
    assert_eq!(c, '3');
    assert_eq!(rest3, "xyz");
}

#[test]
fn test_any_send_partial_state_wraps_parser() {
    let mut p = any_send_partial_state(many1::<String, _, _>(digit()));
    let (out, rest) = p.parse("12345abc").unwrap();
    assert_eq!(out, "12345");
    assert_eq!(rest, "abc");
    assert_eq!(out.len(), 5);

    let r_err = p.parse("xyz");
    assert_eq!(r_err.is_err(), true);

    let (out2, rest2) = p.parse("7").unwrap();
    assert_eq!(out2, "7");
    assert_eq!(rest2, "");

    let mut p2 = any_send_partial_state((digit(), letter()));
    let ((d, l), rest3) = p2.parse("1aXX").unwrap();
    assert_eq!(d, '1');
    assert_eq!(l, 'a');
    assert_eq!(rest3, "XX");
}

#[test]
fn test_any_send_sync_partial_state_wraps_parser() {
    let mut p = any_send_sync_partial_state(many1::<String, _, _>(letter()));
    let (out, rest) = p.parse("hello123").unwrap();
    assert_eq!(out, "hello");
    assert_eq!(rest, "123");
    assert_eq!(out.len(), 5);

    let (out2, rest2) = p.parse("z").unwrap();
    assert_eq!(out2, "z");
    assert_eq!(rest2, "");

    let r_err = p.parse("123");
    assert_eq!(r_err.is_err(), true);

    let mut p2 = any_send_sync_partial_state((letter(), digit(), letter()));
    let ((a, d, b), rest3) = p2.parse("a1b!").unwrap();
    assert_eq!(a, 'a');
    assert_eq!(d, '1');
    assert_eq!(b, 'b');
    assert_eq!(rest3, "!");
}

#[test]
fn test_map_input_transforms_with_input_access() {
    let mut p = map_input(digit(), |c: char, _input: &mut &str| {
        c.to_digit(10).unwrap() * 2
    });
    let (out, rest) = p.parse("4xyz").unwrap();
    assert_eq!(out, 8u32);
    assert_eq!(rest, "xyz");

    let (out2, rest2) = p.parse("9").unwrap();
    assert_eq!(out2, 18u32);
    assert_eq!(rest2, "");

    let r_err = p.parse("abc");
    assert_eq!(r_err.is_err(), true);

    let mut p2 = map_input(
        many1::<String, _, _>(digit()),
        |s: String, _i: &mut &str| s.len(),
    );
    let (out3, rest3) = p2.parse("12345abc").unwrap();
    assert_eq!(out3, 5usize);
    assert_eq!(rest3, "abc");

    let (out4, rest4) = p2.parse("9").unwrap();
    assert_eq!(out4, 1usize);
    assert_eq!(rest4, "");
}

#[test]
fn test_lazy_creates_parser_on_demand() {
    fn make() -> combine::parser::char::Digit<&'static str> {
        digit()
    }
    let mut p = lazy(make);
    let (out, rest) = p.parse("9xyz").unwrap();
    assert_eq!(out, '9');
    assert_eq!(rest, "xyz");

    let (out2, rest2) = p.parse("0").unwrap();
    assert_eq!(out2, '0');
    assert_eq!(rest2, "");

    let r_err = p.parse("abc");
    assert_eq!(r_err.is_err(), true);

    let r_err2 = p.parse("");
    assert_eq!(r_err2.is_err(), true);

    let (out3, rest3) = p.parse("5!").unwrap();
    assert_eq!(out3, '5');
    assert_eq!(rest3, "!");

    let (out4, rest4) = p.parse("7end").unwrap();
    assert_eq!(out4, '7');
    assert_eq!(rest4, "end");
}

#[test]
fn test_factory_creates_parser_per_call() {
    let mut p = factory(|_input: &mut &str| digit());
    let (out, rest) = p.parse("5xyz").unwrap();
    assert_eq!(out, '5');
    assert_eq!(rest, "xyz");

    let (out2, rest2) = p.parse("0").unwrap();
    assert_eq!(out2, '0');
    assert_eq!(rest2, "");

    let r_err = p.parse("abc");
    assert_eq!(r_err.is_err(), true);

    let r_err2 = p.parse("");
    assert_eq!(r_err2.is_err(), true);

    let (out3, rest3) = p.parse("3!").unwrap();
    assert_eq!(out3, '3');
    assert_eq!(rest3, "!");

    let (out4, rest4) = p.parse("9end").unwrap();
    assert_eq!(out4, '9');
    assert_eq!(rest4, "end");
}