use combine::Parser;
use combine::parser::repeat::count;
use combine::parser::char::{digit, letter};
use combine::parser::sequence::{then, then_partial, then_ref};

#[test]
fn test_then_chains_parser_based_on_consumed_output() {
    let mut p = then(digit(), |c: char| {
        let n = c.to_digit(10).unwrap() as usize;
        count::<String, _, _>(n, letter())
    });

    let (out, rest) = p.parse("3abcdef").unwrap();
    assert_eq!(out, "abc");
    assert_eq!(rest, "def");
    assert_eq!(out.len(), 3);

    let (out2, rest2) = p.parse("0xyz").unwrap();
    assert_eq!(out2, "");
    assert_eq!(rest2, "xyz");
    assert_eq!(out2.len(), 0);

    let (out3, rest3) = p.parse("5helloworld").unwrap();
    assert_eq!(out3, "hello");
    assert_eq!(rest3, "world");
    assert_eq!(out3.len(), 5);

    let r_err = p.parse("abc");
    assert!(r_err.is_err());






    let r2 = p.parse("3ab");
    assert!(r2.is_ok());
    let (out4, rest4) = r2.unwrap();
    assert_eq!(out4, "ab");
    assert_eq!(rest4, "");
    assert_eq!(out4.len(), 2);

    let r_err3 = p.parse("");
    assert!(r_err3.is_err());
}

#[test]
fn test_then_ref_uses_borrowed_first_output() {
    let mut p = then_ref(digit(), |c: &char| {
        let n = c.to_digit(10).unwrap() as usize;
        count::<String, _, _>(n, letter())
    });

    let ((_, out), rest) = p.parse("2abxyz").unwrap();
    assert_eq!(out, "ab");
    assert_eq!(rest, "xyz");
    assert_eq!(out.len(), 2);

    let ((_, out2), rest2) = p.parse("4abcdEF").unwrap();
    assert_eq!(out2, "abcd");
    assert_eq!(rest2, "EF");
    assert_eq!(out2.len(), 4);

    let ((_, out3), rest3) = p.parse("0!").unwrap();
    assert_eq!(out3, "");
    assert_eq!(rest3, "!");

    let r_err = p.parse("xxx");
    assert!(r_err.is_err());

    let r_err2 = p.parse("");
    assert!(r_err2.is_err());
}

#[test]
fn test_then_partial_with_mutable_ref() {
    let mut p = then_partial(digit(), |c: &mut char| {
        let n = c.to_digit(10).unwrap() as usize;
        count::<String, _, _>(n, letter())
    });

    let (out, rest) = p.parse("3abcdef").unwrap();
    assert_eq!(out, "abc");
    assert_eq!(rest, "def");
    assert_eq!(out.len(), 3);

    let (out2, rest2) = p.parse("1z!").unwrap();
    assert_eq!(out2, "z");
    assert_eq!(rest2, "!");
    assert_eq!(out2.len(), 1);

    let (out3, rest3) = p.parse("0end").unwrap();
    assert_eq!(out3, "");
    assert_eq!(rest3, "end");
    assert_eq!(out3.len(), 0);




    let r = p.parse("3a");
    assert!(r.is_ok());
    let (out4, rest4) = r.unwrap();
    assert_eq!(out4, "a");
    assert_eq!(rest4, "");
    assert_eq!(out4.len(), 1);

    let r_err2 = p.parse("nope");
    assert!(r_err2.is_err());
}