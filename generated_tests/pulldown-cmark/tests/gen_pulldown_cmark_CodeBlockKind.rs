
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};

#[test]
fn test_code_block_kind_is_indented_true() {
    let md = "    indented code block\n    second line\n\nparagraph after\n";
    let parser = Parser::new(md);
    let mut found_indented = false;
    let mut found_code_start = false;

    for event in parser {
        if let Event::Start(Tag::CodeBlock(ref kind)) = event {
            found_code_start = true;
            if kind.is_indented() {
                found_indented = true;
                assert_eq!(kind.is_indented(), true);
                assert_eq!(kind.is_fenced(), false);
            }
        }
    }

    assert!(found_code_start, "Should have found a code block start");
    assert!(found_indented, "Should have found an indented code block");


    let indented = CodeBlockKind::Indented;
    assert_eq!(indented.is_indented(), true);
    assert_eq!(indented.is_fenced(), false);
    assert_ne!(indented.is_indented(), false);
    assert_ne!(indented.is_fenced(), true);
}

#[test]
fn test_code_block_kind_is_fenced_true() {
    let md = "```rust\nfn main() {}\n```\n";
    let parser = Parser::new(md);
    let mut found_fenced = false;
    let mut found_code_start = false;

    for event in parser {
        if let Event::Start(Tag::CodeBlock(ref kind)) = event {
            found_code_start = true;
            if kind.is_fenced() {
                found_fenced = true;
                assert_eq!(kind.is_fenced(), true);
                assert_eq!(kind.is_indented(), false);
            }
        }
    }

    assert!(found_code_start, "Should have found a code block start");
    assert!(found_fenced, "Should have found a fenced code block");


    let fenced: CodeBlockKind<'_> = CodeBlockKind::Fenced(CowStr::Borrowed("rust"));
    assert_eq!(fenced.is_fenced(), true);
    assert_eq!(fenced.is_indented(), false);
    assert_ne!(fenced.is_fenced(), false);
    assert_ne!(fenced.is_indented(), true);
}

#[test]
fn test_code_block_kind_fenced_without_language() {
    let md = "```\nplain code\n```\n";
    let parser = Parser::new(md);
    let mut found_fenced_empty = false;

    for event in parser {
        if let Event::Start(Tag::CodeBlock(ref kind)) = event {
            assert_eq!(kind.is_fenced(), true);
            assert_eq!(kind.is_indented(), false);
            if let CodeBlockKind::Fenced(info) = kind {
                assert_eq!(info.as_ref(), "");
                found_fenced_empty = true;
            }
        }
    }

    assert!(found_fenced_empty, "Should have found a fenced code block with empty info string");


    let fenced_empty: CodeBlockKind<'_> = CodeBlockKind::Fenced(CowStr::Borrowed(""));
    assert_eq!(fenced_empty.is_fenced(), true);
    assert_eq!(fenced_empty.is_indented(), false);
}

#[test]
fn test_code_block_kind_into_static_indented() {
    let indented = CodeBlockKind::Indented;
    let static_indented = indented.into_static();

    assert_eq!(static_indented.is_indented(), true);
    assert_eq!(static_indented.is_fenced(), false);


    let boxed: Box<CodeBlockKind<'static>> = Box::new(static_indented);
    assert_eq!(boxed.is_indented(), true);
    assert_eq!(boxed.is_fenced(), false);
    assert_ne!(boxed.is_indented(), false);
    assert_ne!(boxed.is_fenced(), true);
}

#[test]
fn test_code_block_kind_into_static_fenced() {
    let info_string = String::from("python");
    let fenced: CodeBlockKind<'_> = CodeBlockKind::Fenced(CowStr::from(info_string.as_str()));
    let static_fenced: CodeBlockKind<'static> = fenced.into_static();

    assert_eq!(static_fenced.is_fenced(), true);
    assert_eq!(static_fenced.is_indented(), false);

    if let CodeBlockKind::Fenced(ref info) = static_fenced {
        assert_eq!(info.as_ref(), "python");
    } else {
        panic!("Expected Fenced variant after into_static");
    }


    let boxed: Box<CodeBlockKind<'static>> = Box::new(static_fenced);
    assert_eq!(boxed.is_fenced(), true);
    assert_eq!(boxed.is_indented(), false);
    assert_ne!(boxed.is_fenced(), false);
}

#[test]
fn test_code_block_kind_into_static_fenced_complex_info() {

    let md = "```rust,no_run\nlet x = 1;\n```\n";
    let parser = Parser::new(md);
    let mut collected_kind: Option<CodeBlockKind<'static>> = None;

    for event in parser {
        if let Event::Start(Tag::CodeBlock(kind)) = event {
            let static_kind = kind.into_static();
            assert_eq!(static_kind.is_fenced(), true);
            assert_eq!(static_kind.is_indented(), false);
            collected_kind = Some(static_kind);
        }
    }

    let kind = collected_kind.expect("Should have collected a code block kind");
    assert_eq!(kind.is_fenced(), true);
    assert_eq!(kind.is_indented(), false);

    if let CodeBlockKind::Fenced(ref info) = kind {
        assert_eq!(info.as_ref(), "rust,no_run");
    } else {
        panic!("Expected Fenced variant");
    }
}

#[test]
fn test_code_block_kind_multiple_blocks_mixed() {
    let md = concat!(
        "    indented line 1\n",
        "    indented line 2\n",
        "\n",
        "```javascript\n",
        "console.log('hello');\n",
        "```\n",
        "\n",
        "    another indented\n",
        "\n",
        "~~~python\n",
        "print('world')\n",
        "~~~\n",
    );

    let parser = Parser::new(md);
    let mut indented_count = 0;
    let mut fenced_count = 0;
    let mut static_kinds: Vec<CodeBlockKind<'static>> = Vec::new();

    for event in parser {
        if let Event::Start(Tag::CodeBlock(kind)) = event {
            if kind.is_indented() {
                indented_count += 1;
                assert_eq!(kind.is_fenced(), false);
            } else if kind.is_fenced() {
                fenced_count += 1;
                assert_eq!(kind.is_indented(), false);
            }
            static_kinds.push(kind.into_static());
        }
    }

    assert_eq!(indented_count, 2);
    assert_eq!(fenced_count, 2);
    assert_eq!(static_kinds.len(), 4);


    assert_eq!(static_kinds[0].is_indented(), true);
    assert_eq!(static_kinds[1].is_fenced(), true);
    assert_eq!(static_kinds[2].is_indented(), true);
    assert_eq!(static_kinds[3].is_fenced(), true);

    if let CodeBlockKind::Fenced(ref info) = static_kinds[1] {
        assert_eq!(info.as_ref(), "javascript");
    } else {
        panic!("Expected fenced with javascript");
    }

    if let CodeBlockKind::Fenced(ref info) = static_kinds[3] {
        assert_eq!(info.as_ref(), "python");
    } else {
        panic!("Expected fenced with python");
    }
}

#[test]
fn test_code_block_kind_tilde_fenced() {
    let md = "~~~\ntilde fenced\n~~~\n";
    let parser = Parser::new(md);
    let mut found = false;

    for event in parser {
        if let Event::Start(Tag::CodeBlock(ref kind)) = event {
            found = true;
            assert_eq!(kind.is_fenced(), true);
            assert_eq!(kind.is_indented(), false);
            if let CodeBlockKind::Fenced(info) = kind {
                assert_eq!(info.as_ref(), "");
            }
        }
    }

    assert!(found, "Should have found a tilde-fenced code block");


    let tilde_kind: CodeBlockKind<'_> = CodeBlockKind::Fenced(CowStr::Borrowed(""));
    let static_tilde = tilde_kind.into_static();
    assert_eq!(static_tilde.is_fenced(), true);
    assert_eq!(static_tilde.is_indented(), false);
}

#[test]
fn test_code_block_kind_into_static_preserves_long_info_string() {

    let long_info = "a".repeat(100);
    let fenced: CodeBlockKind<'_> = CodeBlockKind::Fenced(CowStr::from(long_info.clone()));

    assert_eq!(fenced.is_fenced(), true);
    assert_eq!(fenced.is_indented(), false);

    let static_fenced = fenced.into_static();
    assert_eq!(static_fenced.is_fenced(), true);
    assert_eq!(static_fenced.is_indented(), false);

    if let CodeBlockKind::Fenced(ref info) = static_fenced {
        assert_eq!(info.as_ref(), long_info.as_str());
        assert_eq!(info.len(), 100);
    } else {
        panic!("Expected Fenced variant after into_static with long info");
    }


    let indented_static = CodeBlockKind::Indented.into_static();
    assert_eq!(indented_static.is_indented(), true);
}

#[test]
fn test_code_block_kind_equality_and_behavior() {

    let variants: Vec<CodeBlockKind<'_>> = vec![
        CodeBlockKind::Indented,
        CodeBlockKind::Fenced(CowStr::Borrowed("")),
        CodeBlockKind::Fenced(CowStr::Borrowed("rust")),
        CodeBlockKind::Fenced(CowStr::Borrowed("c++")),
    ];

    for kind in &variants {

        assert_ne!(kind.is_indented(), kind.is_fenced());
        assert_eq!(kind.is_indented() || kind.is_fenced(), true);
        assert_eq!(kind.is_indented() && kind.is_fenced(), false);
    }

    assert_eq!(variants[0].is_indented(), true);
    assert_eq!(variants[1].is_fenced(), true);
    assert_eq!(variants[2].is_fenced(), true);
    assert_eq!(variants[3].is_fenced(), true);
}