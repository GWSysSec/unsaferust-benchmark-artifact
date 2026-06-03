use winnow::Parser;
use winnow::ascii::{digit1, alpha1};
use winnow::error::{ContextError, InputError, ParseError};
use winnow::combinator::delimited;

#[test]
fn test_parse_error_input_and_inner_str() {

    let input = "abc";
    let res: Result<&str, ParseError<&str, ContextError>> =
        digit1::<&str, ContextError>.parse(input);
    let err = res.expect_err("must fail");


    assert_eq!(*err.input(), "abc");
    assert_eq!(err.input().len(), 3);


    assert_eq!(err.offset(), 0);


    let span = err.char_span();
    assert_eq!(span.start, 0);
    assert!(span.end >= span.start);
    assert!(span.end <= 3);


    let _inner: &ContextError = err.inner();

    let _inner2: &ContextError = err.inner();
    assert_eq!(*err.input(), "abc");
}

#[test]
fn test_parse_error_offset_midway() {




    let mut parser = delimited(digit1::<&str, InputError<&str>>, alpha1, digit1);
    let input = "123abcXYZ";
    let res: Result<&str, ParseError<&str, InputError<&str>>> =
        parser.parse(input);
    let err = res.expect_err("must fail: trailing not digits");


    assert_eq!(*err.input(), "123abcXYZ");
    assert_eq!(err.input().len(), 9);


    assert_eq!(err.offset(), 9);


    let span = err.char_span();
    assert_eq!(span.start, 9);
    assert!(span.end >= 9);
    assert!(span.end <= 9);


    let inner: &InputError<&str> = err.inner();

    assert_eq!(inner.input, "");
}

#[test]
fn test_parse_error_char_span_multibyte() {

    let input = "αβγ";

    let res: Result<&str, ParseError<&str, ContextError>> =
        digit1::<&str, ContextError>.parse(input);
    let err = res.expect_err("must fail on non-digits");

    assert_eq!(*err.input(), "αβγ");
    assert_eq!(err.offset(), 0);


    let span = err.char_span();
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 2);
    assert_eq!(span.end - span.start, 'α'.len_utf8());


    let _inner: &ContextError = err.inner();

    assert_eq!(err.input().as_bytes().len(), 6);
}

#[test]
fn test_parse_error_char_span_at_end_of_input() {


    let mut parser = (digit1::<&str, ContextError>, alpha1::<&str, ContextError>);
    let input = "123";
    let res: Result<(&str, &str), ParseError<&str, ContextError>> =
        parser.parse(input);
    let err = res.expect_err("alpha1 after digits on empty tail");

    assert_eq!(*err.input(), "123");

    assert_eq!(err.offset(), 3);

    let span = err.char_span();
    assert_eq!(span.start, 3);

    assert!(span.end >= 3);
    assert!(span.end <= 3);

    let _inner: &ContextError = err.inner();
    assert_eq!(err.input().len(), 3);
}