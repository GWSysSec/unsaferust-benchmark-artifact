use cssparser::{
    BasicParseErrorKind, ParseError, ParseErrorKind, Parser, ParserInput, SourceLocation,
};

#[derive(Debug, PartialEq)]
struct SmallError {
    code: u16,
    message: &'static str,
}

#[derive(Debug, PartialEq)]
struct LargeError {
    code: u16,
    message: String,
}

impl From<SmallError> for LargeError {
    fn from(value: SmallError) -> Self {
        Self {
            code: value.code,
            message: value.message.to_owned(),
        }
    }
}

#[test]
fn parse_error_into_converts_custom_error_while_preserving_location() {
    let mut input = ParserInput::new("body: broken-value");
    let mut parser = Parser::new(&mut input);

    let ident = parser.expect_ident().expect("first token should be an ident");
    assert_eq!(&**ident, "body");

    parser.expect_colon().expect("ident should be followed by a colon");

    let location_before_error = parser.current_source_location();
    let original: ParseError<'_, SmallError> = parser.new_custom_error(SmallError {
        code: 404,
        message: "unknown property value",
    });

    let converted: ParseError<'_, LargeError> = cssparser::ParseError::into(original);

    assert_eq!(converted.location.line, location_before_error.line);
    assert_eq!(converted.location.column, location_before_error.column);

    match converted.kind {
        ParseErrorKind::Custom(error) => {
            assert_eq!(error.code, 404);
            assert_eq!(error.message, "unknown property value");
        }
        other => panic!("expected converted custom error, got {other:?}"),
    }

    let remaining = parser.expect_ident().expect("parser should still be usable");
    assert_eq!(&**remaining, "broken-value");
    assert!(parser.is_exhausted());
}

#[test]
fn parse_error_into_preserves_basic_error_kind_and_location() {
    let mut input = ParserInput::new("   ");
    let mut parser = Parser::new(&mut input);

    parser.skip_whitespace();
    assert!(parser.is_exhausted());

    let location_before_error: SourceLocation = parser.current_source_location();
    let original: ParseError<'_, SmallError> =
        parser.new_error(BasicParseErrorKind::EndOfInput);

    let converted: ParseError<'_, LargeError> = cssparser::ParseError::into(original);

    assert_eq!(converted.location.line, location_before_error.line);
    assert_eq!(converted.location.column, location_before_error.column);

    match converted.kind {
        ParseErrorKind::Basic(BasicParseErrorKind::EndOfInput) => {}
        other => panic!("expected EndOfInput basic error, got {other:?}"),
    }
}