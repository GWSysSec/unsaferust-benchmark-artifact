use cssparser::*;

#[test]
fn test_expect_square_bracket_block_basic() {
    let css = "[color: red]";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    let start_pos = parser.state().position().byte_index();
    assert_eq!(start_pos, 0);

    let result = parser.expect_square_bracket_block();
    assert!(result.is_ok());

    let after_pos = parser.state().position().byte_index();
    assert_ne!(after_pos, start_pos);
    assert!(after_pos >= 1);

    let nested_result = parser.parse_nested_block(|inner| -> Result<usize, ParseError<()>> {
        let mut count = 0;
        while inner.next().is_ok() {
            count += 1;
        }
        Ok(count)
    });
    assert!(nested_result.is_ok());
    let count = nested_result.unwrap();
    assert!(count >= 3);
    assert_ne!(count, 0);

    assert!(parser.is_exhausted());
}

#[test]
fn test_expect_square_bracket_block_wrong_token() {
    let css = "(not a bracket)";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    let pos_before = parser.state().position().byte_index();
    assert_eq!(pos_before, 0);

    let result = parser.expect_square_bracket_block();
    assert!(result.is_err());

    let err = result.err().unwrap();
    let cloned = err.clone();
    assert_eq!(format!("{:?}", err), format!("{:?}", cloned));

    let css2 = "";
    let mut input2 = ParserInput::new(css2);
    let mut parser2 = Parser::new(&mut input2);
    let result2 = parser2.expect_square_bracket_block();
    assert!(result2.is_err());

    let css3 = "  42";
    let mut input3 = ParserInput::new(css3);
    let mut parser3 = Parser::new(&mut input3);
    let result3 = parser3.expect_square_bracket_block();
    assert!(result3.is_err());

    let css4 = "ident";
    let mut input4 = ParserInput::new(css4);
    let mut parser4 = Parser::new(&mut input4);
    assert!(parser4.expect_square_bracket_block().is_err());
}

#[test]
fn test_expect_function_basic() {
    let css = "rgb(255, 0, 0)";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    let start = parser.state().position().byte_index();
    assert_eq!(start, 0);

    let name_result = parser.expect_function();
    assert!(name_result.is_ok());
    let name = name_result.unwrap().clone();
    assert_eq!(&*name, "rgb");
    assert_eq!(name.len(), 3);
    assert_ne!(name.len(), 0);

    let after = parser.state().position().byte_index();
    assert!(after > start);
    assert_eq!(after, 4);

    let args: Result<Vec<i32>, ParseError<()>> = parser.parse_nested_block(|inner| {
        let mut nums = Vec::new();
        nums.push(inner.expect_integer()?);
        inner.expect_comma()?;
        nums.push(inner.expect_integer()?);
        inner.expect_comma()?;
        nums.push(inner.expect_integer()?);
        Ok(nums)
    });
    assert!(args.is_ok());
    let nums = args.unwrap();
    assert_eq!(nums.len(), 3);
    assert_eq!(nums[0], 255);
    assert_eq!(nums[1], 0);
    assert_eq!(nums[2], 0);
}

#[test]
fn test_expect_function_errors_and_variants() {
    let css = "calc(1 + 2)";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    let n = parser.expect_function().unwrap().clone();
    assert_eq!(&*n, "calc");
    assert_eq!(n.len(), 4);

    let css2 = "ident";
    let mut input2 = ParserInput::new(css2);
    let mut parser2 = Parser::new(&mut input2);
    let pos2 = parser2.state().position().byte_index();
    assert_eq!(pos2, 0);
    let r = parser2.expect_function();
    assert!(r.is_err());

    let css3 = "123";
    let mut input3 = ParserInput::new(css3);
    let mut parser3 = Parser::new(&mut input3);
    assert!(parser3.expect_function().is_err());

    let css4 = "(not-a-function)";
    let mut input4 = ParserInput::new(css4);
    let mut parser4 = Parser::new(&mut input4);
    assert!(parser4.expect_function().is_err());

    let css5 = "URL(http://x)";
    let mut input5 = ParserInput::new(css5);
    let mut parser5 = Parser::new(&mut input5);
    let name5 = parser5.expect_function();


    if let Ok(n) = name5 {
        assert_ne!(n.len(), 0);
        assert!(n.len() >= 3);
    } else {
        assert!(name5.is_err());
    }
}