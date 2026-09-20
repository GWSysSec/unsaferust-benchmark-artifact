use pulldown_cmark::{html, Options, Parser};
use std::io::Cursor;

#[test]
fn write_html_io_basic_paragraph() {
    let md = "Hello **world**!";
    let parser = Parser::new(md);
    let mut buf: Vec<u8> = Vec::new();
    let res = html::write_html_io(&mut buf, parser);
    assert!(res.is_ok());
    let out = String::from_utf8(buf).expect("utf8");
    assert_eq!(out, "<p>Hello <strong>world</strong>!</p>\n");
    assert!(out.contains("<strong>"));
    assert!(out.contains("</strong>"));
    assert!(out.starts_with("<p>"));
    assert!(out.ends_with("</p>\n"));
    assert_ne!(out, md);
    assert_eq!(out.matches("<p>").count(), 1);
}

#[test]
fn write_html_io_matches_push_html() {
    let inputs = [
        "# Heading\n\nSome *emphasized* text.",
        "- a\n- b\n- c\n",
        "> quote\n> more",
        "```\ncode block\n```\n",
        "[link](https://example.com)",
    ];
    let mut count = 0;
    for md in inputs {
        let mut pushed = String::new();
        html::push_html(&mut pushed, Parser::new(md));

        let mut buf: Vec<u8> = Vec::new();
        html::write_html_io(&mut buf, Parser::new(md)).expect("write ok");
        let written = String::from_utf8(buf).expect("utf8");

        assert_eq!(pushed, written);
        assert!(!written.is_empty());
        count += 1;
    }
    assert_eq!(count, 5);
    assert_eq!(inputs.len(), 5);
    assert_ne!(inputs[0], inputs[1]);
}

#[test]
fn write_html_io_with_cursor_and_tables() {
    let md = "A | B\n---|---\nfoo | bar\n";
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(md, opts);

    let backing: Vec<u8> = Vec::with_capacity(256);
    let mut cursor = Cursor::new(backing);
    let pre_pos = cursor.position();
    assert_eq!(pre_pos, 0);

    html::write_html_io(&mut cursor, parser).expect("write ok");

    let post_pos = cursor.position();
    assert_ne!(post_pos, 0);
    let inner = cursor.into_inner();
    assert_eq!(inner.len() as u64, post_pos);
    let s = String::from_utf8(inner).expect("utf8");
    assert!(s.contains("<table>"));
    assert!(s.contains("<thead>"));
    assert!(s.contains("<tbody>"));
    assert!(s.contains("<th>A</th>"));
    assert!(s.contains("<th>B</th>"));
    assert!(s.contains("<td>foo</td>"));
    assert!(s.contains("<td>bar</td>"));
    assert!(s.ends_with("</table>\n"));
}

#[test]
fn write_html_io_large_input_capacity() {
    let mut md = String::new();
    for i in 0..500 {
        md.push_str(&format!("# Heading {}\n\nParagraph number {} with **bold**.\n\n", i, i));
    }
    let parser = Parser::new(&md);
    let mut buf: Vec<u8> = Vec::new();
    let pre_cap = buf.capacity();
    assert_eq!(pre_cap, 0);
    assert_eq!(buf.len(), 0);

    html::write_html_io(&mut buf, parser).expect("write ok");

    assert!(buf.len() > md.len() / 2);
    assert!(buf.capacity() >= buf.len());
    let s = std::str::from_utf8(&buf).expect("utf8");
    assert_eq!(s.matches("<h1>").count(), 500);
    assert_eq!(s.matches("</h1>").count(), 500);
    assert_eq!(s.matches("<strong>bold</strong>").count(), 500);
    assert!(s.contains("Heading 0"));
    assert!(s.contains("Heading 499"));
    assert!(!s.contains("Heading 500"));
}

#[test]
fn write_html_io_empty_input() {
    let parser = Parser::new("");
    let mut buf: Vec<u8> = Vec::new();
    let res = html::write_html_io(&mut buf, parser);
    assert!(res.is_ok());
    assert_eq!(buf.len(), 0);
    assert!(buf.is_empty());
    let s = String::from_utf8(buf).expect("utf8");
    assert_eq!(s, "");
    assert_eq!(s.len(), 0);
    assert!(s.is_empty());
    assert_ne!("not empty", s);
}

#[test]
fn write_html_io_error_propagation() {
    struct FailingWriter {
        writes: usize,
    }
    impl std::io::Write for FailingWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.writes += 1;
            Err(std::io::Error::new(std::io::ErrorKind::Other, "nope"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
        fn write_all(&mut self, _buf: &[u8]) -> std::io::Result<()> {
            self.writes += 1;
            Err(std::io::Error::new(std::io::ErrorKind::Other, "nope"))
        }
    }

    let parser = Parser::new("# hi\n\nparagraph");
    let mut w = FailingWriter { writes: 0 };
    assert_eq!(w.writes, 0);
    let res = html::write_html_io(&mut w, parser);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Other);
    assert!(w.writes >= 1);
    assert_ne!(w.writes, 0);
    assert!(format!("{}", err).contains("nope"));
}