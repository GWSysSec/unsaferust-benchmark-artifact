use cssparser::*;

#[test]
fn test_token_serialization_type_nothing_equality() {
    let a = TokenSerializationType::nothing();
    let b = TokenSerializationType::nothing();
    let c = TokenSerializationType::nothing();


    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(a, c);
    assert_eq!(a, TokenSerializationType::nothing());


    let d = a;
    let e = b;
    assert_eq!(d, a);
    assert_eq!(e, b);
    assert_eq!(d, e);
    assert_eq!(c, d);
}

#[test]
fn test_token_serialization_type_set_if_nothing_idempotent() {
    let baseline = TokenSerializationType::nothing();

    let mut x = TokenSerializationType::nothing();
    assert_eq!(x, baseline);


    x.set_if_nothing(TokenSerializationType::nothing());
    assert_eq!(x, baseline);


    x.set_if_nothing(TokenSerializationType::nothing());
    assert_eq!(x, baseline);


    for _ in 0..50 {
        x.set_if_nothing(TokenSerializationType::nothing());
    }
    assert_eq!(x, baseline);


    let mut y = TokenSerializationType::nothing();
    y.set_if_nothing(TokenSerializationType::nothing());
    assert_eq!(y, x);
    assert_eq!(y, baseline);


    let instances: Vec<TokenSerializationType> = (0..20)
        .map(|_| {
            let mut t = TokenSerializationType::nothing();
            t.set_if_nothing(TokenSerializationType::nothing());
            t
        })
        .collect();
    assert_eq!(instances.len(), 20);
    for inst in &instances {
        assert_eq!(*inst, baseline);
    }
}

#[test]
fn test_token_serialization_type_alongside_parser() {


    let css = "color: red !important";
    let mut input = ParserInput::new(css);
    let _parser = Parser::new(&mut input);

    let nothing1 = TokenSerializationType::nothing();
    let nothing2 = TokenSerializationType::nothing();
    assert_eq!(nothing1, nothing2);

    let nothing3 = TokenSerializationType::nothing();
    assert_eq!(nothing1, nothing3);
    assert_eq!(nothing2, nothing3);

    let mut tst = TokenSerializationType::nothing();
    assert_eq!(tst, nothing1);

    tst.set_if_nothing(TokenSerializationType::nothing());
    assert_eq!(tst, nothing1);
    assert_eq!(tst, nothing2);
    assert_eq!(tst, nothing3);

    let nothing4 = TokenSerializationType::nothing();
    assert_eq!(tst, nothing4);
    assert_eq!(nothing4, nothing1);
}

#[test]
fn test_parse_nth_multi_case_workflow() {


    let cases: Vec<(&str, i32, i32)> = vec![
        ("2n+1", 2, 1),
        ("3n-2", 3, -2),
        ("odd", 2, 1),
        ("even", 2, 0),
        ("5", 0, 5),
        ("-n+3", -1, 3),
        ("n", 1, 0),
    ];

    let baseline = TokenSerializationType::nothing();
    let mut tracker = TokenSerializationType::nothing();

    for (src, expected_a, expected_b) in &cases {
        let mut input = ParserInput::new(src);
        let mut parser = Parser::new(&mut input);
        let (a, b) = parse_nth(&mut parser).expect("should parse nth");
        assert_eq!(a, *expected_a, "a mismatch for {}", src);
        assert_eq!(b, *expected_b, "b mismatch for {}", src);


        tracker.set_if_nothing(TokenSerializationType::nothing());
        assert_eq!(tracker, baseline);
    }


    let mut bad = ParserInput::new("foo");
    let mut bad_parser = Parser::new(&mut bad);
    assert!(parse_nth(&mut bad_parser).is_err());


    assert_eq!(tracker, baseline);
    assert_eq!(tracker, TokenSerializationType::nothing());
}

#[test]
fn test_parse_important_workflow() {

    let mut input = ParserInput::new("!important");
    let mut parser = Parser::new(&mut input);
    assert!(parse_important(&mut parser).is_ok());


    let mut input2 = ParserInput::new("!IMPORTANT");
    let mut parser2 = Parser::new(&mut input2);
    assert!(parse_important(&mut parser2).is_ok());

    let mut input3 = ParserInput::new("!Important");
    let mut parser3 = Parser::new(&mut input3);
    assert!(parse_important(&mut parser3).is_ok());


    let mut bad = ParserInput::new("!bogus");
    let mut bad_parser = Parser::new(&mut bad);
    assert!(parse_important(&mut bad_parser).is_err());


    let mut bad2 = ParserInput::new("important");
    let mut bad_parser2 = Parser::new(&mut bad2);
    assert!(parse_important(&mut bad_parser2).is_err());


    let mut bad3 = ParserInput::new("");
    let mut bad_parser3 = Parser::new(&mut bad3);
    assert!(parse_important(&mut bad_parser3).is_err());


    let nothing_before = TokenSerializationType::nothing();
    let mut mutable_tst = TokenSerializationType::nothing();
    mutable_tst.set_if_nothing(TokenSerializationType::nothing());
    let nothing_after = TokenSerializationType::nothing();
    assert_eq!(nothing_before, nothing_after);
    assert_eq!(mutable_tst, nothing_before);
    assert_eq!(mutable_tst, nothing_after);
}