use pulldown_cmark::{html, BrokenLink, CowStr, LinkType, Options, Parser};

#[test]
fn broken_link_into_static_basic() {
    let owned_ref: String = String::from("my-reference");
    let bl = BrokenLink {
        span: 0..12,
        link_type: LinkType::Reference,
        reference: CowStr::Borrowed(&owned_ref),
    };

    assert_eq!(bl.span.start, 0);
    assert_eq!(bl.span.end, 12);
    assert_eq!(&*bl.reference, "my-reference");
    assert_eq!(bl.link_type, LinkType::Reference);

    let static_bl: BrokenLink<'static> = bl.into_static();

    assert_eq!(static_bl.span.start, 0);
    assert_eq!(static_bl.span.end, 12);
    assert_eq!(&*static_bl.reference, "my-reference");
    assert_eq!(static_bl.link_type, LinkType::Reference);
    assert_ne!(static_bl.span.end, 0);
}

#[test]
fn broken_link_into_static_outlives_source() {
    let captured: BrokenLink<'static> = {
        let temp_source: String = String::from("ephemeral-ref");
        let bl = BrokenLink {
            span: 5..18,
            link_type: LinkType::ReferenceUnknown,
            reference: CowStr::Borrowed(temp_source.as_str()),
        };
        assert_eq!(&*bl.reference, "ephemeral-ref");
        assert_eq!(bl.span.start, 5);
        assert_eq!(bl.span.end, 18);
        bl.into_static()

    };

    assert_eq!(&*captured.reference, "ephemeral-ref");
    assert_eq!(captured.span.start, 5);
    assert_eq!(captured.span.end, 18);
    assert_eq!(captured.link_type, LinkType::ReferenceUnknown);
    assert_ne!(captured.link_type, LinkType::Reference);
    assert_eq!(captured.reference.len(), 13);
}

#[test]
fn broken_link_into_static_from_callback() {
    let md = r##"See [foo] and [bar] here.

[unused]: https://example.org
"##;

    let mut collected: Vec<BrokenLink<'static>> = Vec::new();
    assert_eq!(collected.len(), 0);

    let mut html_out = String::new();
    {
        let mut callback = |bl: BrokenLink<'_>| {
            let owned = bl.into_static();
            collected.push(owned);
            None
        };
        let parser =
            Parser::new_with_broken_link_callback(md, Options::empty(), Some(&mut callback));
        html::push_html(&mut html_out, parser);
    }

    assert!(!html_out.is_empty());
    assert!(html_out.contains("[foo]"));
    assert!(html_out.contains("[bar]"));
    assert_eq!(collected.len(), 2);

    let refs: Vec<String> = collected.iter().map(|b| b.reference.to_string()).collect();
    assert!(refs.contains(&"foo".to_string()));
    assert!(refs.contains(&"bar".to_string()));
    assert_ne!(refs[0], "unused");
    assert!(collected[0].span.end > collected[0].span.start);
    assert!(collected[1].span.end > collected[1].span.start);
}

#[test]
fn broken_link_into_static_with_owned_cowstr() {
    let bl = BrokenLink {
        span: 100..110,
        link_type: LinkType::Shortcut,
        reference: CowStr::Boxed("dynamic-ref".to_string().into_boxed_str()),
    };

    assert_eq!(&*bl.reference, "dynamic-ref");
    assert_eq!(bl.span.start, 100);
    assert_eq!(bl.span.end, 110);
    assert_eq!(bl.link_type, LinkType::Shortcut);

    let s: BrokenLink<'static> = bl.into_static();

    assert_eq!(&*s.reference, "dynamic-ref");
    assert_eq!(s.span.start, 100);
    assert_eq!(s.span.end, 110);
    assert_eq!(s.span.len(), 10);
    assert_eq!(s.link_type, LinkType::Shortcut);
    assert_ne!(s.link_type, LinkType::Collapsed);
    assert_eq!(s.reference.len(), 11);
}