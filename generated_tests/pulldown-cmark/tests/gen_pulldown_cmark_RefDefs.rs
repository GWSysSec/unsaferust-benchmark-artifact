use pulldown_cmark::{Options, Parser};






#[test]
fn refdefs_iter_basic_single_definition() {
    let md = "[foo]: https://example.com\n\n[foo]\n";
    let parser = Parser::new(md);
    let refdefs = parser.reference_definitions();

    let collected: Vec<_> = refdefs.iter().collect();
    assert_eq!(collected.len(), 1);

    let (label, linkdef) = collected[0];
    assert_eq!(label, "foo");
    assert_eq!(&*linkdef.dest, "https://example.com");
    assert!(linkdef.title.is_none());


    let count_second: usize = refdefs.iter().count();
    assert_eq!(count_second, 1);


    let third: Vec<&str> = refdefs.iter().map(|(k, _)| k).collect();
    assert_eq!(third, vec!["foo"]);
    assert_ne!(third.len(), 0);
}

#[test]
fn refdefs_iter_multiple_definitions_with_titles() {
    let md = r#"[alpha]: https://alpha.example "Alpha Title"
[beta]: https://beta.example 'Beta Title'
[gamma]: https://gamma.example (Gamma Title)

[alpha] [beta] [gamma]
"#;
    let parser = Parser::new(md);
    let refdefs = parser.reference_definitions();

    let mut entries: Vec<(String, String, Option<String>)> = refdefs
        .iter()
        .map(|(k, v)| {
            (
                k.to_string(),
                v.dest.to_string(),
                v.title.as_ref().map(|t| t.to_string()),
            )
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, "alpha");
    assert_eq!(entries[1].0, "beta");
    assert_eq!(entries[2].0, "gamma");

    assert_eq!(entries[0].1, "https://alpha.example");
    assert_eq!(entries[1].1, "https://beta.example");
    assert_eq!(entries[2].1, "https://gamma.example");

    assert_eq!(entries[0].2.as_deref(), Some("Alpha Title"));
    assert_eq!(entries[1].2.as_deref(), Some("Beta Title"));
    assert_eq!(entries[2].2.as_deref(), Some("Gamma Title"));
}

#[test]
fn refdefs_iter_empty_when_no_definitions() {
    let md = "Just a paragraph with no link definitions.\n\nAnother paragraph.\n";
    let parser = Parser::new(md);
    let refdefs = parser.reference_definitions();

    let collected: Vec<_> = refdefs.iter().collect();
    assert_eq!(collected.len(), 0);
    assert_eq!(refdefs.iter().count(), 0);


    let again: Vec<_> = refdefs.iter().collect();
    assert_eq!(again.len(), 0);


    let mut it = refdefs.iter();
    assert!(it.next().is_none());
    assert!(it.next().is_none());


    let md2 = "# Heading\n\n- list item\n- another\n";
    let p2 = Parser::new(md2);
    assert_eq!(p2.reference_definitions().iter().count(), 0);
}

#[test]
fn refdefs_iter_case_insensitive_label_normalization() {

    let md = r#"[FOO]: https://upper.example
[BaR]: https://mixed.example "mixed title"

[foo] and [bar]
"#;
    let parser = Parser::new(md);
    let refdefs = parser.reference_definitions();

    let mut keys: Vec<String> = refdefs.iter().map(|(k, _)| k.to_string()).collect();
    keys.sort();
    assert_eq!(keys.len(), 2);



    let foo_def = refdefs
        .iter()
        .find(|(_, v)| v.dest.as_ref() == "https://upper.example");
    assert!(foo_def.is_some());
    let bar_def = refdefs
        .iter()
        .find(|(_, v)| v.dest.as_ref() == "https://mixed.example");
    assert!(bar_def.is_some());

    let (_, bar_linkdef) = bar_def.unwrap();
    assert_eq!(bar_linkdef.title.as_deref(), Some("mixed title"));
    assert_ne!(bar_linkdef.dest.as_ref(), "https://upper.example");
}

#[test]
fn refdefs_iter_duplicate_labels_keeps_first() {

    let md = r#"[dup]: https://first.example "first"
[dup]: https://second.example "second"

[dup]
"#;
    let parser = Parser::new(md);
    let refdefs = parser.reference_definitions();

    let collected: Vec<_> = refdefs.iter().collect();
    assert_eq!(collected.len(), 1);

    let (label, linkdef) = collected[0];
    assert_eq!(label, "dup");
    assert_eq!(linkdef.dest.as_ref(), "https://first.example");
    assert_eq!(linkdef.title.as_deref(), Some("first"));
    assert_ne!(linkdef.dest.as_ref(), "https://second.example");
    assert_ne!(linkdef.title.as_deref(), Some("second"));
}

#[test]
fn refdefs_iter_span_fields_are_valid() {
    let md = "prelude\n\n[one]: https://one.example\n[two]: https://two.example \"t2\"\n\n[one][two]\n";
    let parser = Parser::new(md);
    let refdefs = parser.reference_definitions();

    let entries: Vec<_> = refdefs.iter().collect();
    assert_eq!(entries.len(), 2);


    for (label, linkdef) in refdefs.iter() {
        assert!(!label.is_empty());
        assert!(linkdef.span.start < linkdef.span.end);
        assert!(linkdef.span.end <= md.len());


        let slice = &md[linkdef.span.clone()];
        assert!(slice.contains(linkdef.dest.as_ref()));
    }


    let spans: Vec<_> = refdefs.iter().map(|(_, v)| v.span.clone()).collect();
    assert_eq!(spans.len(), 2);
    assert_ne!(spans[0], spans[1]);
}

#[test]
fn refdefs_iter_many_definitions_stress() {

    let mut md = String::new();
    let n = 50usize;
    for i in 0..n {
        md.push_str(&format!("[label{}]: https://example.com/{}\n", i, i));
    }
    md.push_str("\nParagraph referencing [label0] and [label25] and [label49].\n");

    let parser = Parser::new_ext(&md, Options::all());
    let refdefs = parser.reference_definitions();

    let collected: Vec<_> = refdefs.iter().collect();
    assert_eq!(collected.len(), n);


    let l0 = refdefs
        .iter()
        .find(|(k, _)| *k == "label0")
        .expect("label0 should exist");
    assert_eq!(l0.1.dest.as_ref(), "https://example.com/0");

    let l25 = refdefs
        .iter()
        .find(|(k, _)| *k == "label25")
        .expect("label25 should exist");
    assert_eq!(l25.1.dest.as_ref(), "https://example.com/25");

    let l49 = refdefs
        .iter()
        .find(|(k, _)| *k == "label49")
        .expect("label49 should exist");
    assert_eq!(l49.1.dest.as_ref(), "https://example.com/49");


    let titled: usize = refdefs.iter().filter(|(_, v)| v.title.is_some()).count();
    assert_eq!(titled, 0);


    let total: usize = refdefs.iter().map(|_| 1usize).sum();
    assert_eq!(total, n);
}

#[test]
fn refdefs_iter_after_full_parse_still_available() {


    let md = r#"See [link-a] and [link-b].

[link-a]: https://a.example "A"
[link-b]: https://b.example
"#;


    let mut event_count = 0usize;
    {
        let parser = Parser::new(md);
        for _ in parser {
            event_count += 1;
        }
    }
    assert_ne!(event_count, 0);


    let parser2 = Parser::new(md);
    let refdefs = parser2.reference_definitions();

    let all: Vec<_> = refdefs.iter().collect();
    assert_eq!(all.len(), 2);

    let a = refdefs.iter().find(|(k, _)| *k == "link-a").unwrap();
    assert_eq!(a.1.dest.as_ref(), "https://a.example");
    assert_eq!(a.1.title.as_deref(), Some("A"));

    let b = refdefs.iter().find(|(k, _)| *k == "link-b").unwrap();
    assert_eq!(b.1.dest.as_ref(), "https://b.example");
    assert!(b.1.title.is_none());
    assert_ne!(a.1.dest.as_ref(), b.1.dest.as_ref());
}