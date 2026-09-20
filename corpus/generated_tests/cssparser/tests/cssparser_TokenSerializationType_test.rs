use cssparser::{Parser, ParserInput, TokenSerializationType};

fn token_serialization_types(css: &str) -> Vec<TokenSerializationType> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    let mut types = Vec::new();

    while !parser.is_exhausted() {
        let token = parser.next().expect("expected a valid CSS token");
        types.push(token.serialization_type());
    }

    types
}

#[test]
fn nothing_never_requires_a_separator() {
    let types = token_serialization_types("alpha beta");
    assert_eq!(types.len(), 2);

    let nothing = TokenSerializationType::nothing();
    let ident = types[0];

    assert!(!nothing.needs_separator_when_before(nothing));
    assert!(!nothing.needs_separator_when_before(ident));
    assert!(!ident.needs_separator_when_before(nothing));
    assert!(ident.needs_separator_when_before(types[1]));
}

#[test]
fn set_if_nothing_initializes_only_empty_serialization_type() {
    let types = token_serialization_types("alpha,beta");
    assert_eq!(types.len(), 3);

    let first_ident = types[0];
    let comma = types[1];
    let second_ident = types[2];

    assert!(first_ident.needs_separator_when_before(second_ident));
    assert!(!comma.needs_separator_when_before(second_ident));

    let mut initialized_from_nothing = TokenSerializationType::nothing();
    initialized_from_nothing.set_if_nothing(first_ident);
    assert!(initialized_from_nothing.needs_separator_when_before(second_ident));

    let mut already_initialized = first_ident;
    already_initialized.set_if_nothing(comma);
    assert!(already_initialized.needs_separator_when_before(second_ident));

    let mut still_nothing = TokenSerializationType::nothing();
    still_nothing.set_if_nothing(TokenSerializationType::nothing());
    assert!(!still_nothing.needs_separator_when_before(first_ident));
    assert!(!second_ident.needs_separator_when_before(still_nothing));
}