use cssparser::{parse_important, Parser, ParserInput};

#[test]
fn source_position_byte_index_tracks_parser_progress_and_slicing() {
    let css = "  color: red;\nmargin: 10 px";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    let initial = parser.position();
    assert_eq!(initial.byte_index(), 0);
    assert_eq!(parser.current_line(), "  color: red;");

    parser.skip_whitespace();
    let after_leading_whitespace = parser.position();
    assert_eq!(after_leading_whitespace.byte_index(), 2);

    let ident = parser.expect_ident_cloned().unwrap();
    assert_eq!(&*ident, "color");

    let after_property_name = parser.position();
    assert_eq!(after_property_name.byte_index(), 7);
    assert_eq!(parser.slice(initial..after_property_name), "  color");

    parser.expect_colon().unwrap();
    assert_eq!(parser.position().byte_index(), 8);

    parser.skip_whitespace();
    let value_start = parser.position();
    assert_eq!(value_start.byte_index(), 9);

    let value = parser.expect_ident_cloned().unwrap();
    assert_eq!(&*value, "red");

    let value_end = parser.position();
    assert_eq!(value_end.byte_index(), 12);
    assert_eq!(parser.slice(value_start..value_end), "red");

    parser.expect_semicolon().unwrap();
    assert_eq!(parser.position().byte_index(), 13);

    parser.skip_whitespace();
    let second_declaration_start = parser.position();
    assert_eq!(second_declaration_start.byte_index(), 14);
    assert_eq!(parser.current_line(), "margin: 10 px");

    let second_property = parser.expect_ident_cloned().unwrap();
    assert_eq!(&*second_property, "margin");
    assert_eq!(parser.position().byte_index(), 20);
    assert_eq!(parser.slice_from(second_declaration_start), "margin");

    parser.expect_colon().unwrap();
    assert_eq!(parser.position().byte_index(), 21);

    parser.skip_whitespace();
    let second_value_start = parser.position();
    assert_eq!(second_value_start.byte_index(), 22);

    let number = parser.expect_number().unwrap();
    assert_eq!(number, 10.0);
    assert_eq!(parser.position().byte_index(), 24);

    parser.skip_whitespace();
    assert_eq!(parser.position().byte_index(), 25);

    let unit = parser.expect_ident_cloned().unwrap();
    assert_eq!(&*unit, "px");

    let second_declaration_end = parser.position();
    assert_eq!(second_declaration_end.byte_index(), css.len());
    assert_eq!(
        parser.slice(second_declaration_start..second_declaration_end),
        "margin: 10 px"
    );
    assert_eq!(parser.slice_from(second_value_start), "10 px");
    assert!(parser.is_exhausted());
}

#[test]
fn source_position_byte_index_is_restored_after_try_parse_failure_and_reset() {
    let css = "margin: 10 px !important";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    let start = parser.position();
    assert_eq!(start.byte_index(), 0);

    let failed_number = parser.try_parse(|input| input.expect_number());
    assert!(failed_number.is_err());
    assert_eq!(parser.position().byte_index(), start.byte_index());

    let saved_state = parser.state();
    assert_eq!(saved_state.position().byte_index(), 0);

    let property = parser.expect_ident_cloned().unwrap();
    assert_eq!(&*property, "margin");
    assert_eq!(parser.position().byte_index(), 6);

    parser.reset(&saved_state);
    assert_eq!(parser.position().byte_index(), 0);

    parser.expect_ident_matching("margin").unwrap();
    parser.expect_colon().unwrap();
    parser.skip_whitespace();

    let number_start = parser.position();
    assert_eq!(number_start.byte_index(), 8);

    let number = parser.expect_number().unwrap();
    assert_eq!(number, 10.0);
    assert_eq!(parser.position().byte_index(), 10);

    parser.skip_whitespace();
    assert_eq!(parser.position().byte_index(), 11);

    let unit = parser.expect_ident_cloned().unwrap();
    assert_eq!(&*unit, "px");
    assert_eq!(parser.position().byte_index(), 13);

    parser.skip_whitespace();
    assert_eq!(parser.position().byte_index(), 14);

    parse_important(&mut parser).unwrap();
    assert_eq!(parser.position().byte_index(), css.len());
    assert!(parser.is_exhausted());
}