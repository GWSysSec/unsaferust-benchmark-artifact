use bstr::{concat, join, decode_utf8, decode_last_utf8, B};

#[test]
fn test_concat_basic_and_edge_cases() {

    let empty: Vec<&[u8]> = vec![];
    let result = concat(empty);
    assert_eq!(result, b"");
    assert_eq!(result.len(), 0);


    let result = concat(vec![b"hello".as_slice()]);
    assert_eq!(result, b"hello");
    assert_eq!(result.len(), 5);


    let result = concat(vec![b"foo".as_slice(), b"bar".as_slice(), b"baz".as_slice()]);
    assert_eq!(result, b"foobarbaz");
    assert_eq!(result.len(), 9);


    let result = concat(vec![b"a".as_slice(), b"".as_slice(), b"b".as_slice(), b"".as_slice(), b"c".as_slice()]);
    assert_eq!(result, b"abc");
    assert_eq!(result.len(), 3);


    let result = concat(vec![b"\x00\x01".as_slice(), b"\x02\x03".as_slice()]);
    assert_eq!(result, b"\x00\x01\x02\x03");
    assert_eq!(result.len(), 4);
    assert_eq!(result[0], 0x00);
    assert_eq!(result[3], 0x03);
}

#[test]
fn test_concat_with_utf8_and_invalid_sequences() {

    let result = concat(vec!["héllo".as_bytes(), " ".as_bytes(), "wörld".as_bytes()]);
    assert_eq!(result, "héllo wörld".as_bytes());


    let invalid = b"\xff\xfe";
    let valid = "abc".as_bytes();
    let result = concat(vec![valid, invalid.as_slice()]);
    assert_eq!(result.len(), 5);
    assert_eq!(result[0], b'a');
    assert_eq!(result[3], 0xff);
    assert_eq!(result[4], 0xfe);


    let cjk = "日本語".as_bytes();
    let result = concat(vec![cjk, cjk]);
    assert_eq!(result.len(), cjk.len() * 2);
    assert_eq!(&result[..cjk.len()], cjk);
    assert_eq!(&result[cjk.len()..], cjk);
}

#[test]
fn test_concat_large_number_of_elements() {
    let elements: Vec<&[u8]> = (0..1000).map(|_| b"x".as_slice()).collect();
    let result = concat(elements);
    assert_eq!(result.len(), 1000);
    assert!(result.iter().all(|&b| b == b'x'));
    assert_eq!(result[0], b'x');
    assert_eq!(result[999], b'x');
    assert_eq!(result[500], b'x');


    let elements: Vec<Vec<u8>> = (0u8..10).map(|i| vec![i; i as usize]).collect();
    let refs: Vec<&[u8]> = elements.iter().map(|v| v.as_slice()).collect();
    let result = concat(refs);

    assert_eq!(result.len(), 45);
    assert_eq!(result[0], 1);
    assert_eq!(result[44], 9);
}

#[test]
fn test_join_basic_and_edge_cases() {

    let empty: Vec<&[u8]> = vec![];
    let result = join(b",", empty);
    assert_eq!(result, b"");
    assert_eq!(result.len(), 0);


    let result = join(b",", vec![b"hello".as_slice()]);
    assert_eq!(result, b"hello");
    assert_eq!(result.len(), 5);


    let result = join(b",", vec![b"a".as_slice(), b"b".as_slice(), b"c".as_slice()]);
    assert_eq!(result, b"a,b,c");
    assert_eq!(result.len(), 5);


    let result = join(b"---", vec![b"foo".as_slice(), b"bar".as_slice()]);
    assert_eq!(result, b"foo---bar");
    assert_eq!(result.len(), 9);


    let result = join(b"", vec![b"x".as_slice(), b"y".as_slice(), b"z".as_slice()]);
    assert_eq!(result, b"xyz");
    assert_eq!(result.len(), 3);
}

#[test]
fn test_join_with_empty_elements_and_binary_data() {

    let result = join(b"|", vec![b"".as_slice(), b"a".as_slice(), b"".as_slice(), b"b".as_slice(), b"".as_slice()]);
    assert_eq!(result, b"|a||b|");
    assert_eq!(result.len(), 6);


    let result = join(b",", vec![b"".as_slice(), b"".as_slice(), b"".as_slice()]);
    assert_eq!(result, b",,");
    assert_eq!(result.len(), 2);


    let result = join(b"\x00", vec![b"A".as_slice(), b"B".as_slice()]);
    assert_eq!(result, b"A\x00B");
    assert_eq!(result.len(), 3);
    assert_eq!(result[1], 0x00);


    let sep = b"<SEP>";
    let result = join(&sep[..], vec![b"first".as_slice(), b"second".as_slice(), b"third".as_slice()]);
    assert_eq!(result, b"first<SEP>second<SEP>third");
    assert_eq!(result.len(), 5 + 5 + 6 + 5 + 5);
}

#[test]
fn test_join_with_strings_and_utf8() {

    let result = join(", ", vec!["hello", "world", "rust"]);
    assert_eq!(result, b"hello, world, rust");
    assert_eq!(result.len(), 18);


    let result = join("→", vec!["start", "middle", "end"]);
    let expected = "start→middle→end".as_bytes();
    assert_eq!(result, expected);


    let result = join(b"\n", vec![b"line1".as_slice(), b"line2".as_slice(), b"line3".as_slice()]);
    assert_eq!(result, b"line1\nline2\nline3");
    assert_ne!(result, b"line1\nline2\nline3\n");


    let last_byte = result[result.len() - 1];
    assert_eq!(last_byte, b'3');
}

#[test]
fn test_decode_utf8_valid_ascii() {

    let (ch, size) = decode_utf8(b"A");
    assert_eq!(ch, Some('A'));
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"hello");
    assert_eq!(ch, Some('h'));
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"\x00rest");
    assert_eq!(ch, Some('\0'));
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b" ");
    assert_eq!(ch, Some(' '));
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"\x7f");
    assert_eq!(ch, Some('\x7f'));
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"~");
    assert_eq!(ch, Some('~'));
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"9xyz");
    assert_eq!(ch, Some('9'));
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"");
    assert_eq!(ch, None);
    assert_eq!(size, 0);
}

#[test]
fn test_decode_utf8_multibyte_characters() {

    let bytes = "é".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('é'));
    assert_eq!(size, 2);


    let bytes = "日".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('日'));
    assert_eq!(size, 3);


    let bytes = "🦀".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('🦀'));
    assert_eq!(size, 4);


    let bytes = "über".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('ü'));
    assert_eq!(size, 2);


    let bytes = "中abc".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('中'));
    assert_eq!(size, 3);


    let bytes = "\u{FEFF}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{FEFF}'));
    assert_eq!(size, 3);


    let bytes = "\u{FFFD}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{FFFD}'));
    assert_eq!(size, 3);


    let bytes = "\u{10FFFF}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{10FFFF}'));
    assert_eq!(size, 4);
}

#[test]
fn test_decode_utf8_invalid_sequences() {

    let (ch, size) = decode_utf8(b"\x80");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"\xfe");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"\xff");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"\xc2");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"\xe0\xa0");
    assert_eq!(ch, None);
    assert_eq!(size, 2);


    let (ch, size) = decode_utf8(b"\xf0\x9f");
    assert_eq!(ch, None);
    assert_eq!(size, 2);


    let (ch, size) = decode_utf8(b"\xc0\x80");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_utf8(b"\xe0\x80\x80");
    assert_eq!(ch, None);
    assert_eq!(size, 1);
}

#[test]
fn test_decode_utf8_iterative_decoding() {

    let input = "Aé日🦀".as_bytes();
    let mut pos = 0;
    let mut chars = Vec::new();
    let mut sizes = Vec::new();

    while pos < input.len() {
        let (ch, size) = decode_utf8(&input[pos..]);
        assert!(size > 0);
        chars.push(ch.unwrap());
        sizes.push(size);
        pos += size;
    }

    assert_eq!(chars, vec!['A', 'é', '日', '🦀']);
    assert_eq!(sizes, vec![1, 2, 3, 4]);
    assert_eq!(pos, input.len());
    assert_eq!(pos, 10);


    let input = b"a\xff\xc3\xa9z";
    let mut pos = 0;
    let mut results = Vec::new();

    while pos < input.len() {
        let (ch, size) = decode_utf8(&input[pos..]);
        assert!(size > 0);
        results.push((ch, size));
        pos += size;
    }

    assert_eq!(results[0], (Some('a'), 1));
    assert_eq!(results[1], (None, 1));
    assert_eq!(results[2], (Some('é'), 2));
    assert_eq!(results[3], (Some('z'), 1));
}

#[test]
fn test_decode_last_utf8_valid_ascii() {

    let (ch, size) = decode_last_utf8(b"A");
    assert_eq!(ch, Some('A'));
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"hello");
    assert_eq!(ch, Some('o'));
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"");
    assert_eq!(ch, None);
    assert_eq!(size, 0);


    let (ch, size) = decode_last_utf8(b"\x00");
    assert_eq!(ch, Some('\0'));
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"abc7");
    assert_eq!(ch, Some('7'));
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"line\n");
    assert_eq!(ch, Some('\n'));
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"data\t");
    assert_eq!(ch, Some('\t'));
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"word ");
    assert_eq!(ch, Some(' '));
    assert_eq!(size, 1);
}

#[test]
fn test_decode_last_utf8_multibyte_characters() {

    let bytes = "café".as_bytes();
    let (ch, size) = decode_last_utf8(bytes);
    assert_eq!(ch, Some('é'));
    assert_eq!(size, 2);


    let bytes = "hello日".as_bytes();
    let (ch, size) = decode_last_utf8(bytes);
    assert_eq!(ch, Some('日'));
    assert_eq!(size, 3);


    let bytes = "rust🦀".as_bytes();
    let (ch, size) = decode_last_utf8(bytes);
    assert_eq!(ch, Some('🦀'));
    assert_eq!(size, 4);


    let bytes = "🎉".as_bytes();
    let (ch, size) = decode_last_utf8(bytes);
    assert_eq!(ch, Some('🎉'));
    assert_eq!(size, 4);


    let bytes = "xü".as_bytes();
    let (ch, size) = decode_last_utf8(bytes);
    assert_eq!(ch, Some('ü'));
    assert_eq!(size, 2);


    let bytes = "\u{10FFFF}".as_bytes();
    let (ch, size) = decode_last_utf8(bytes);
    assert_eq!(ch, Some('\u{10FFFF}'));
    assert_eq!(size, 4);


    let bytes = "x\u{FFFD}".as_bytes();
    let (ch, size) = decode_last_utf8(bytes);
    assert_eq!(ch, Some('\u{FFFD}'));
    assert_eq!(size, 3);


    let bytes = "日本語".as_bytes();
    let (ch, size) = decode_last_utf8(bytes);
    assert_eq!(ch, Some('語'));
    assert_eq!(size, 3);
}

#[test]
fn test_decode_last_utf8_invalid_sequences() {

    let (ch, size) = decode_last_utf8(b"abc\x80");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"test\xff");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"x\xfe");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"abc\xc2");
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"\x80\x80\x80");
    assert_eq!(ch, None);
    assert_eq!(size, 1);






    let (ch, size) = decode_last_utf8(b"\xc0\x80");
    assert_eq!(ch, None);








    assert_eq!(size, 1);


    let input = b"A\x80";
    let (ch, size) = decode_last_utf8(input);
    assert_eq!(ch, None);
    assert_eq!(size, 1);


    let (ch, size) = decode_last_utf8(b"\xff");
    assert_eq!(ch, None);
    assert_eq!(size, 1);
}

#[test]
fn test_decode_last_utf8_iterative_reverse_decoding() {

    let input = "Aé日🦀".as_bytes();
    let mut end = input.len();
    let mut chars = Vec::new();
    let mut sizes = Vec::new();

    while end > 0 {
        let (ch, size) = decode_last_utf8(&input[..end]);
        assert!(size > 0);
        chars.push(ch.unwrap());
        sizes.push(size);
        end -= size;
    }

    chars.reverse();
    sizes.reverse();
    assert_eq!(chars, vec!['A', 'é', '日', '🦀']);
    assert_eq!(sizes, vec![1, 2, 3, 4]);
    assert_eq!(end, 0);


    let mut forward_chars = Vec::new();
    let mut pos = 0;
    while pos < input.len() {
        let (ch, size) = decode_utf8(&input[pos..]);
        forward_chars.push(ch.unwrap());
        pos += size;
    }
    assert_eq!(chars, forward_chars);
}

#[test]
fn test_decode_utf8_and_decode_last_utf8_symmetry() {

    let single_chars = ["A", "é", "日", "🦀", "\u{0}", "\u{7F}", "\u{80}", "\u{7FF}", "\u{800}", "\u{FFFF}", "\u{10000}"];

    for s in &single_chars {
        let bytes = s.as_bytes();
        let (fwd_ch, fwd_size) = decode_utf8(bytes);
        let (bwd_ch, bwd_size) = decode_last_utf8(bytes);
        assert_eq!(fwd_ch, bwd_ch);
        assert_eq!(fwd_size, bwd_size);
        assert_eq!(fwd_size, bytes.len());
    }


    for b in [0x80u8, 0xBF, 0xC0, 0xC1, 0xFE, 0xFF] {
        let bytes = [b];
        let (fwd_ch, fwd_size) = decode_utf8(&bytes);
        let (bwd_ch, bwd_size) = decode_last_utf8(&bytes);
        assert_eq!(fwd_ch, None);
        assert_eq!(bwd_ch, None);
        assert_eq!(fwd_size, 1);
        assert_eq!(bwd_size, 1);
    }
}

#[test]
fn test_concat_and_join_equivalence() {

    let elements: Vec<&[u8]> = vec![b"alpha", b"beta", b"gamma"];
    let concat_result = concat(elements.clone());
    let join_result = join(b"", elements);
    assert_eq!(concat_result, join_result);
    assert_eq!(concat_result, b"alphabetagamma");


    let elements: Vec<&[u8]> = vec![b"only"];
    let concat_result = concat(elements.clone());
    let join_result = join(b"---", elements);
    assert_eq!(concat_result, b"only");
    assert_eq!(join_result, b"only");
    assert_eq!(concat_result, join_result);


    let elements: Vec<&[u8]> = vec![];
    let concat_result = concat(elements.clone());
    let join_result = join(b",", elements);
    assert_eq!(concat_result, b"");
    assert_eq!(join_result, b"");
    assert_eq!(concat_result, join_result);
}

#[test]
fn test_b_function_with_concat_and_join() {

    let a = B("hello");
    let b_val = B(" world");
    assert_eq!(a, b"hello");
    assert_eq!(b_val, b" world");

    let result = concat(vec![a, b_val]);
    assert_eq!(result, b"hello world");

    let result = join(B(", "), vec![B("one"), B("two"), B("three")]);
    assert_eq!(result, b"one, two, three");
    assert_eq!(result.len(), 15);


    let raw: &[u8] = &[0xff, 0xfe, 0xfd];
    let b_raw = B(raw);
    assert_eq!(b_raw, raw);
    let result = concat(vec![b_raw, B("end")]);
    assert_eq!(result.len(), 6);
    assert_eq!(result[0], 0xff);
    assert_eq!(result[3], b'e');
}

#[test]
fn test_join_path_like_construction() {

    let parts = vec!["usr", "local", "bin", "program"];
    let path = join(b"/", parts);
    assert_eq!(path, b"usr/local/bin/program");
    assert_eq!(path.len(), 21);


    let fields = vec!["name", "age", "city"];
    let row = join(b",", fields);
    assert_eq!(row, b"name,age,city");


    let items = vec!["item1", "item2", "item3"];
    let display = join(b" | ", items);
    assert_eq!(display, b"item1 | item2 | item3");
    assert_eq!(display.len(), 21);


    let lines = vec!["first line", "second line", "third line"];
    let text = join(b"\n", lines);
    assert_eq!(text, b"first line\nsecond line\nthird line");
    assert_ne!(text.last(), Some(&b'\n'));
}

#[test]
fn test_decode_utf8_boundary_codepoints() {

    let (ch, size) = decode_utf8(b"\x7f");
    assert_eq!(ch, Some('\u{7F}'));
    assert_eq!(size, 1);


    let bytes = "\u{80}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{80}'));
    assert_eq!(size, 2);


    let bytes = "\u{7FF}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{7FF}'));
    assert_eq!(size, 2);


    let bytes = "\u{800}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{800}'));
    assert_eq!(size, 3);


    let bytes = "\u{FFFF}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{FFFF}'));
    assert_eq!(size, 3);


    let bytes = "\u{10000}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{10000}'));
    assert_eq!(size, 4);


    let bytes = "\u{10FFFF}".as_bytes();
    let (ch, size) = decode_utf8(bytes);
    assert_eq!(ch, Some('\u{10FFFF}'));
    assert_eq!(size, 4);


    let (ch, size) = decode_utf8(b"\xed\xa0\x80");
    assert_eq!(ch, None);
    assert_eq!(size, 1);
}