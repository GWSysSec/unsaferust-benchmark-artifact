use winnow::Parser;
use winnow::ascii::{digit1, alpha1};
use winnow::error::{ContextError, InputError, ParseError};
use winnow::combinator::delimited;

#[test]
fn test_parse_error_input_and_inner_str() {
    // digit1 requires at least one digit; fails on "abc"
    let input = "abc";
    let res: Result<&str, ParseError<&str, ContextError>> =
        digit1::<&str, ContextError>.parse(input);
    let err = res.expect_err("must fail");

    // input() returns the original input
    assert_eq!(*err.input(), "abc");
    assert_eq!(err.input().len(), 3);

    // offset starts at 0 since nothing matched
    assert_eq!(err.offset(), 0);

    // char_span should be a valid range at offset 0
    let span = err.char_span();
    assert_eq!(span.start, 0);
    assert!(span.end >= span.start);
    assert!(span.end <= 3);

    // inner() returns a reference to the ContextError
    let _inner: &ContextError = err.inner();
    // Verify inner is same type and accessible repeatedly
    let _inner2: &ContextError = err.inner();
    assert_eq!(*err.input(), "abc");
}

#[test]
fn test_parse_error_offset_midway() {
    // delimited(digit1, alpha1, digit1) on "123abcXYZ":
    // - digit1 consumes "123"
    // - alpha1 consumes "abcXYZ" (all alpha chars)
    // - digit1 expects digits but gets EOF → fails at offset 9
    let mut parser = delimited(digit1::<&str, InputError<&str>>, alpha1, digit1);
    let input = "123abcXYZ";
    let res: Result<&str, ParseError<&str, InputError<&str>>> =
        parser.parse(input);
    let err = res.expect_err("must fail: trailing not digits");

    // original input preserved
    assert_eq!(*err.input(), "123abcXYZ");
    assert_eq!(err.input().len(), 9);

    // alpha1 is greedy and consumes all alpha chars "abcXYZ", so digit1 fails at offset 9 (EOF)
    assert_eq!(err.offset(), 9);

    // char_span at offset 9
    let span = err.char_span();
    assert_eq!(span.start, 9);
    assert!(span.end >= 9);
    assert!(span.end <= 9);

    // inner() accessible
    let inner: &InputError<&str> = err.inner();
    // The InputError's input field should reflect the remaining unconsumed portion (empty at EOF)
    assert_eq!(inner.input, "");
}

#[test]
fn test_parse_error_char_span_multibyte() {
    // Use a multibyte char at failure point
    let input = "αβγ";
    // digit1 should fail immediately at 'α'
    let res: Result<&str, ParseError<&str, ContextError>> =
        digit1::<&str, ContextError>.parse(input);
    let err = res.expect_err("must fail on non-digits");

    assert_eq!(*err.input(), "αβγ");
    assert_eq!(err.offset(), 0);

    // char_span should cover the first char 'α' which is 2 bytes in UTF-8
    let span = err.char_span();
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 2);
    assert_eq!(span.end - span.start, 'α'.len_utf8());

    // inner accessible
    let _inner: &ContextError = err.inner();
    // Confirm input bytes
    assert_eq!(err.input().as_bytes().len(), 6);
}

#[test]
fn test_parse_error_char_span_at_end_of_input() {
    // Parse "42" with digit1 succeeds; need fail AT end. Use digit1 requiring
    // more: combine with a trailing alpha1 that needs input but gets none.
    let mut parser = (digit1::<&str, ContextError>, alpha1::<&str, ContextError>);
    let input = "123";
    let res: Result<(&str, &str), ParseError<&str, ContextError>> =
        parser.parse(input);
    let err = res.expect_err("alpha1 after digits on empty tail");

    assert_eq!(*err.input(), "123");
    // Offset should be at end = 3
    assert_eq!(err.offset(), 3);

    let span = err.char_span();
    assert_eq!(span.start, 3);
    // At EOF, span end should equal start (no char to cover) or be clamped to input length
    assert!(span.end >= 3);
    assert!(span.end <= 3);

    let _inner: &ContextError = err.inner();
    assert_eq!(err.input().len(), 3);
}