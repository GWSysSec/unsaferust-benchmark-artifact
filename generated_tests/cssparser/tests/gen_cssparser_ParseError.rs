use cssparser::*;

#[derive(Debug, PartialEq, Clone)]
struct InnerErr(u32);

#[derive(Debug, PartialEq)]
struct OuterErr(u32);

impl From<InnerErr> for OuterErr {
    fn from(i: InnerErr) -> Self {
        OuterErr(i.0 + 1000)
    }
}

#[test]
fn test_parse_error_into_custom_kind() {
    let css = "foo bar";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);


    let err: ParseError<InnerErr> = parser.new_custom_error(InnerErr(42));


    match &err.kind {
        ParseErrorKind::Custom(c) => {
            assert_eq!(c.0, 42u32);
            assert_ne!(c.0, 0u32);
        }
        _ => panic!("expected Custom kind"),
    }


    let loc_before = err.location;
    assert_eq!(loc_before.line, 0);
    assert_eq!(loc_before.column, 1);


    let converted: ParseError<OuterErr> = err.into::<OuterErr>();


    assert_eq!(converted.location.line, loc_before.line);
    assert_eq!(converted.location.column, loc_before.column);


    match converted.kind {
        ParseErrorKind::Custom(o) => {
            assert_eq!(o.0, 1042u32);
            assert_ne!(o.0, 42u32);
        }
        _ => panic!("expected Custom kind after conversion"),
    }
}

#[test]
fn test_parse_error_into_basic_kind_preserved() {
    let css = "(unbalanced";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);


    let r: Result<(), ParseError<InnerErr>> = parser
        .expect_ident()
        .map(|_| ())
        .map_err(|e| e.into());

    assert!(r.is_err());
    let err = r.err().unwrap();

    let was_basic = matches!(err.kind, ParseErrorKind::Basic(_));
    assert!(was_basic, "expected basic kind from expect_ident on '('");
    let loc_before = err.location;
    assert_eq!(loc_before.line, 0);
    assert!(loc_before.column >= 1);


    let converted: ParseError<OuterErr> = err.into::<OuterErr>();
    assert_eq!(converted.location.line, loc_before.line);
    assert_eq!(converted.location.column, loc_before.column);

    match converted.kind {
        ParseErrorKind::Basic(_) => {}
        ParseErrorKind::Custom(_) => panic!("basic kind should not become custom"),
    }


    let basic: BasicParseError = converted.basic();
    assert_eq!(basic.location.column, loc_before.column);
    assert_eq!(basic.location.line, 0);
}

#[test]
fn test_parse_error_into_chain_conversions() {
    let css = "x";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);


    let err_u8: ParseError<u8> = parser.new_custom_error(7u8);
    match &err_u8.kind {
        ParseErrorKind::Custom(v) => assert_eq!(*v, 7u8),
        _ => panic!(),
    }
    let loc = err_u8.location;
    assert_eq!(loc.line, 0);

    let err_u16: ParseError<u16> = err_u8.into::<u16>();
    match &err_u16.kind {
        ParseErrorKind::Custom(v) => {
            assert_eq!(*v, 7u16);
            assert_ne!(*v, 0u16);
        }
        _ => panic!(),
    }
    assert_eq!(err_u16.location.line, loc.line);
    assert_eq!(err_u16.location.column, loc.column);

    let err_u32: ParseError<u32> = err_u16.into::<u32>();
    match err_u32.kind {
        ParseErrorKind::Custom(v) => {
            assert_eq!(v, 7u32);
            assert!(v < 100u32);
        }
        _ => panic!(),
    }
    assert_eq!(err_u32.location.line, loc.line);
    assert_eq!(err_u32.location.column, loc.column);
}