use cssparser::*;

#[test]
fn byte_index_at_start_is_zero() {

    let mut input_a = ParserInput::new("hello");
    let parser_a = Parser::new(&mut input_a);
    let pos_a = parser_a.position();
    assert_eq!(pos_a.byte_index(), 0);


    let mut input_b = ParserInput::new("");
    let parser_b = Parser::new(&mut input_b);
    assert_eq!(parser_b.position().byte_index(), 0);


    let mut input_c = ParserInput::new("   \t\n");
    let parser_c = Parser::new(&mut input_c);
    assert_eq!(parser_c.position().byte_index(), 0);


    let mut input_d = ParserInput::new("/* comment */ foo");
    let parser_d = Parser::new(&mut input_d);
    assert_eq!(parser_d.position().byte_index(), 0);


    let css_e = "αβγ rest";
    let mut input_e = ParserInput::new(css_e);
    let parser_e = Parser::new(&mut input_e);
    assert_eq!(parser_e.position().byte_index(), 0);


    let pos_a2 = parser_a.position();
    assert_eq!(pos_a.byte_index(), pos_a2.byte_index());
    assert_eq!(pos_a2.byte_index(), 0usize);
    assert_ne!(css_e.len(), 0);
}

#[test]
fn byte_index_advances_monotonically_through_tokens() {
    let css = "red green blue orange";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    let idx0 = parser.position().byte_index();
    assert_eq!(idx0, 0);

    let _ = parser.next().unwrap();
    let idx1 = parser.position().byte_index();
    assert!(idx1 >= 3, "expected at least 3 after 'red', got {}", idx1);
    assert!(idx1 > idx0);

    let _ = parser.next().unwrap();
    let idx2 = parser.position().byte_index();
    assert!(idx2 >= 9, "expected at least 9 after 'green', got {}", idx2);
    assert!(idx2 > idx1);

    let _ = parser.next().unwrap();
    let idx3 = parser.position().byte_index();
    assert!(idx3 >= 14, "expected at least 14 after 'blue', got {}", idx3);
    assert!(idx3 > idx2);

    let _ = parser.next().unwrap();
    let idx4 = parser.position().byte_index();
    assert_eq!(idx4, css.len());
    assert!(idx4 > idx3);


    assert!(parser.is_exhausted());
    assert!(parser.position().byte_index() <= css.len());
    assert_eq!(parser.position().byte_index(), idx4);
}

#[test]
fn byte_index_with_state_save_and_reset() {
    let css = "alpha beta gamma";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);


    let state_start = parser.state();
    assert_eq!(state_start.position().byte_index(), 0);
    assert_eq!(parser.position().byte_index(), 0);


    let _ = parser.next().unwrap();
    let state_after_alpha = parser.state();
    let idx_after_alpha = state_after_alpha.position().byte_index();
    assert!(idx_after_alpha >= 5);
    assert_eq!(parser.position().byte_index(), idx_after_alpha);


    assert_eq!(state_start.position().byte_index(), 0);


    let _ = parser.next().unwrap();
    let idx_after_beta = parser.position().byte_index();
    assert!(idx_after_beta > idx_after_alpha);
    assert!(idx_after_beta >= 10);


    assert_eq!(state_after_alpha.position().byte_index(), idx_after_alpha);
    assert_eq!(state_start.position().byte_index(), 0);


    parser.reset(&state_after_alpha);
    assert_eq!(parser.position().byte_index(), idx_after_alpha);


    parser.reset(&state_start);
    assert_eq!(parser.position().byte_index(), 0);
    assert_eq!(state_start.position().byte_index(), 0);
}

#[test]
fn byte_index_bounds_and_equality_semantics() {
    let css = "foo \"a string\" 42px";
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);


    let p_start_1 = parser.position();
    let p_start_2 = parser.position();
    assert_eq!(p_start_1.byte_index(), p_start_2.byte_index());
    assert_eq!(p_start_1.byte_index(), 0);



    let mut last = 0usize;
    let mut token_count = 0usize;
    while !parser.is_exhausted() {
        let prev = parser.position().byte_index();
        if parser.next().is_err() {
            break;
        }
        let now = parser.position().byte_index();
        assert!(now >= prev, "byte_index regressed: {} -> {}", prev, now);
        assert!(now <= css.len(), "byte_index overran input length");
        assert!(now > last || token_count == 0);
        last = now;
        token_count += 1;
        if token_count > 32 {
            break;
        }
    }

    assert!(token_count >= 3, "expected at least 3 tokens, got {}", token_count);
    assert_eq!(parser.position().byte_index(), css.len());
    assert!(parser.is_exhausted());
    assert!(last <= css.len());
}