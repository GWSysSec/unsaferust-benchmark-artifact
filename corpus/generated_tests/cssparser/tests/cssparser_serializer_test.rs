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

fn nothing() -> TokenSerializationType {
    TokenSerializationType::Nothing
}

fn set_if_nothing(accumulated: &mut TokenSerializationType, value: TokenSerializationType) {
    if *accumulated == TokenSerializationType::Nothing {
        *accumulated = value;
    }
}

#[test]
fn serializer_nothing_behaves_as_neutral_boundary_marker() {
    let types = token_serialization_types("alpha beta 12px");
    assert_eq!(types.len(), 3);

    let nothing = nothing();
    let first_ident = types[0];
    let second_ident = types[1];
    let dimension = types[2];

    assert!(!nothing.needs_separator_when_before(nothing));
    assert!(!nothing.needs_separator_when_before(first_ident));
    assert!(!first_ident.needs_separator_when_before(nothing));
    assert!(first_ident.needs_separator_when_before(second_ident));
    assert!(second_ident.needs_separator_when_before(dimension));
}

#[test]
fn serializer_set_if_nothing_only_initializes_empty_state() {
    let types = token_serialization_types("alpha, beta");
    assert_eq!(types.len(), 3);

    let ident = types[0];
    let comma = types[1];
    let second_ident = types[2];

    let mut accumulated = nothing();
    assert!(!accumulated.needs_separator_when_before(ident));

    set_if_nothing(&mut accumulated, ident);
    assert!(accumulated.needs_separator_when_before(second_ident));

    set_if_nothing(&mut accumulated, comma);
    assert!(accumulated.needs_separator_when_before(second_ident));
    assert!(accumulated.needs_separator_when_before(ident));

    let mut punctuation_state = nothing();
    set_if_nothing(&mut punctuation_state, comma);
    assert!(!punctuation_state.needs_separator_when_before(second_ident));
}

#[test]
fn serializer_state_can_be_built_from_a_real_token_stream() {
    let types = token_serialization_types("url(example.png)#id");
    assert_eq!(types.len(), 2);

    let url = types[0];
    let id_hash = types[1];

    let mut previous = nothing();
    assert!(!previous.needs_separator_when_before(url));

    set_if_nothing(&mut previous, url);
    assert!(!previous.needs_separator_when_before(id_hash));

    let before_second_token = previous;
    set_if_nothing(&mut previous, id_hash);
    assert_eq!(
        previous.needs_separator_when_before(id_hash),
        before_second_token.needs_separator_when_before(id_hash)
    );
}