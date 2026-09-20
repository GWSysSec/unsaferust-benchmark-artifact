use cssparser::{parse_important, ParseError, ParseErrorKind, Parser, ParserInput};

#[derive(Debug, PartialEq)]
struct ParserLayerError {
    code: u16,
    label: &'static str,
}

#[derive(Debug, PartialEq)]
struct ConsumerLayerError {
    code: u32,
    label: String,
    recoverable: bool,
}

impl From<ParserLayerError> for ConsumerLayerError {
    fn from(value: ParserLayerError) -> Self {
        Self {
            code: u32::from(value.code),
            label: value.label.to_owned(),
            recoverable: value.code < 500,
        }
    }
}

#[test]
fn parse_error_kind_into_converts_custom_payload_without_losing_data() {
    let original: ParseErrorKind<'static, ParserLayerError> =
        ParseErrorKind::Custom(ParserLayerError {
            code: 214,
            label: "unsupported-value",
        });

    let converted: ParseErrorKind<'static, ConsumerLayerError> =
        cssparser::ParseErrorKind::into(original);

    match converted {
        ParseErrorKind::Custom(error) => {
            assert_eq!(error.code, 214);
            assert_eq!(error.label, "unsupported-value");
            assert!(error.recoverable);
        }
        ParseErrorKind::Basic(_) => panic!("custom error kind should remain custom after into"),
    }
}

#[test]
fn parse_error_kind_into_fits_a_real_parser_workflow() {
    let mut input = ParserInput::new("margin: 10 !important");
    let mut parser = Parser::new(&mut input);

    let property = parser
        .expect_ident_cloned()
        .expect("declaration should start with a property name");
    assert_eq!(&*property, "margin");

    parser
        .expect_colon()
        .expect("property name should be followed by a colon");

    let error_location = parser.current_source_location();
    let error: ParseError<'_, ParserLayerError> =
        parser.new_custom_error(ParserLayerError {
            code: 422,
            label: "semantic-check-before-value",
        });

    assert_eq!(error.location, error_location);

    let converted_kind: ParseErrorKind<'_, ConsumerLayerError> =
        cssparser::ParseErrorKind::into(error.kind);

    match converted_kind {
        ParseErrorKind::Custom(error) => {
            assert_eq!(error.code, 422);
            assert_eq!(error.label, "semantic-check-before-value");
            assert!(error.recoverable);
        }
        ParseErrorKind::Basic(_) => panic!("custom parser error should convert to custom consumer error"),
    }

    let value = parser
        .expect_number()
        .expect("numeric margin value should still be parseable after creating an error value");
    assert_eq!(value, 10.0);

    parse_important(&mut parser).expect("remaining input should parse as !important");
    assert!(parser.is_exhausted());
}

#[test]
fn parse_error_kind_into_preserves_basic_errors_when_changing_custom_type() {
    let mut input = ParserInput::new("");
    let mut parser = Parser::new(&mut input);

    let error: ParseError<'_, ParserLayerError> = parser.new_error_for_next_token();
    let converted_kind: ParseErrorKind<'_, ConsumerLayerError> =
        cssparser::ParseErrorKind::into(error.kind);

    assert!(
        matches!(converted_kind, ParseErrorKind::Basic(_)),
        "basic parse errors should remain basic when the custom error type changes"
    );
}