use pulldown_cmark::{Event, HeadingLevel, LinkType, Parser, Tag, TagEnd};

#[test]
fn test_tag_to_end_for_various_tags() {
    let md = "# Heading\n\nA *paragraph* with **bold** and `code`.\n\n- item1\n- item2\n\n> quote\n";
    let parser = Parser::new(md);

    let mut start_count = 0;
    let mut end_count = 0;
    let mut stack: Vec<TagEnd> = Vec::new();

    for ev in parser {
        match ev {
            Event::Start(tag) => {
                let end = tag.to_end();
                stack.push(end);
                start_count += 1;
            }
            Event::End(actual_end) => {
                let expected = stack.pop().expect("Unbalanced end tag");
                assert_eq!(expected, actual_end, "Tag::to_end mismatch");
                end_count += 1;
            }
            _ => {}
        }
    }

    assert_eq!(start_count, end_count);
    assert!(start_count >= 7);
    assert!(stack.is_empty());
}

#[test]
fn test_tag_to_end_specific_values() {
    let h = Tag::Heading {
        level: HeadingLevel::H2,
        id: None,
        classes: Vec::new(),
        attrs: Vec::new(),
    };
    assert_eq!(h.to_end(), TagEnd::Heading(HeadingLevel::H2));

    let p = Tag::Paragraph;
    assert_eq!(p.to_end(), TagEnd::Paragraph);

    let e = Tag::Emphasis;
    assert_eq!(e.to_end(), TagEnd::Emphasis);

    let s = Tag::Strong;
    assert_eq!(s.to_end(), TagEnd::Strong);

    let bq = Tag::BlockQuote(None);
    assert_eq!(bq.to_end(), TagEnd::BlockQuote(None));

    let item = Tag::Item;
    assert_eq!(item.to_end(), TagEnd::Item);

    assert_ne!(
        Tag::Heading {
            level: HeadingLevel::H1,
            id: None,
            classes: Vec::new(),
            attrs: Vec::new(),
        }
        .to_end(),
        TagEnd::Heading(HeadingLevel::H2)
    );
}

#[test]
fn test_tag_into_static_outlives_source() {
    let static_tag: Tag<'static> = {
        let source = String::from("https://example.org/path");
        let dest = String::from("dest-value");
        let title = String::from("title-value");
        let id = String::from("id-value");

        let tag: Tag<'_> = Tag::Link {
            link_type: LinkType::Inline,
            dest_url: dest.as_str().into(),
            title: title.as_str().into(),
            id: id.as_str().into(),
        };
        let _ = source;
        tag.into_static()
    };

    match &static_tag {
        Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        } => {
            assert_eq!(*link_type, LinkType::Inline);
            assert_eq!(dest_url.as_ref(), "dest-value");
            assert_eq!(title.as_ref(), "title-value");
            assert_eq!(id.as_ref(), "id-value");
        }
        _ => panic!("expected Link tag"),
    }

    assert_eq!(static_tag.to_end(), TagEnd::Link);
}

#[test]
fn test_collect_static_tags_from_parser() {
    let md = "# Hello\n\nworld [link](https://example.org)\n";
    let static_tags: Vec<Tag<'static>> = Parser::new(md)
        .filter_map(|e| match e {
            Event::Start(t) => Some(t.into_static()),
            _ => None,
        })
        .collect();

    assert!(static_tags.len() >= 3);

    let has_heading = static_tags
        .iter()
        .any(|t| matches!(t, Tag::Heading { level: HeadingLevel::H1, .. }));
    assert!(has_heading);

    let has_paragraph = static_tags.iter().any(|t| matches!(t, Tag::Paragraph));
    assert!(has_paragraph);

    let link_tag = static_tags
        .iter()
        .find(|t| matches!(t, Tag::Link { .. }))
        .expect("should have link");
    if let Tag::Link { dest_url, .. } = link_tag {
        assert_eq!(dest_url.as_ref(), "https://example.org");
    }


    for t in &static_tags {
        let end = t.to_end();

        assert_eq!(end, t.clone().into_static().to_end());
    }
}