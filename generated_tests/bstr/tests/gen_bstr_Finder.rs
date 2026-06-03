use bstr::Finder;
use bstr::FinderReverse;
use bstr::ByteSlice;
use bstr::{B, concat, join, decode_utf8, decode_last_utf8};
use bstr::BString;

#[test]
fn test_finder_into_owned_basic() {
    let finder = {
        let needle = b"hello";
        let f = Finder::new(needle);
        assert_eq!(f.needle(), b"hello");
        f.into_owned()
    };

    assert_eq!(finder.needle(), b"hello");

    let haystack = b"say hello world hello again";
    let matches: Vec<usize> = haystack.find_iter(finder.needle()).collect();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0], 4);
    assert_eq!(matches[1], 16);


    assert_eq!(finder.needle().len(), 5);
    assert_eq!(finder.needle()[0], b'h');
    assert_eq!(finder.needle()[4], b'o');
}

#[test]
fn test_finder_into_owned_empty_needle() {
    let finder = {
        let needle: &[u8] = b"";
        let f = Finder::new(needle);
        assert_eq!(f.needle(), b"");
        f.into_owned()
    };
    assert_eq!(finder.needle(), b"");
    assert_eq!(finder.needle().len(), 0);


    let haystack = b"abc";
    let matches: Vec<usize> = haystack.find_iter(finder.needle()).collect();

    assert!(matches.len() >= 3);
    assert_eq!(matches[0], 0);
    assert_eq!(matches[1], 1);
    assert_eq!(matches[2], 2);
}

#[test]
fn test_finder_into_owned_with_non_utf8_needle() {
    let raw_needle: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x01];
    let finder = {
        let f = Finder::new(&raw_needle);
        assert_eq!(f.needle(), &[0xFF, 0xFE, 0x00, 0x01]);
        f.into_owned()
    };

    assert_eq!(finder.needle().len(), 4);
    assert_eq!(finder.needle()[0], 0xFF);
    assert_eq!(finder.needle()[1], 0xFE);
    assert_eq!(finder.needle()[2], 0x00);
    assert_eq!(finder.needle()[3], 0x01);


    let haystack: Vec<u8> = vec![0x00, 0xFF, 0xFE, 0x00, 0x01, 0x02];
    let pos = haystack.find(finder.needle());
    assert_eq!(pos, Some(1));
}

#[test]
fn test_finder_needle_returns_correct_slice() {
    let needle_str = "pattern";
    let finder = Finder::new(needle_str.as_bytes());
    assert_eq!(finder.needle(), b"pattern");
    assert_eq!(finder.needle().len(), 7);

    let finder2 = Finder::new(b"x");
    assert_eq!(finder2.needle(), b"x");
    assert_eq!(finder2.needle().len(), 1);


    let finder3 = Finder::new(&[42u8]);
    assert_eq!(finder3.needle(), &[42]);
    assert_eq!(finder3.needle()[0], 42);


    let long_needle = vec![b'a'; 256];
    let finder4 = Finder::new(&long_needle);
    assert_eq!(finder4.needle().len(), 256);
    assert!(finder4.needle().iter().all(|&b| b == b'a'));
}

#[test]
fn test_finder_into_owned_survives_scope() {
    let owned_finder: Finder<'static> = {
        let local_data = String::from("search_term");
        let f = Finder::new(local_data.as_bytes());
        assert_eq!(f.needle(), b"search_term");
        f.into_owned()

    };


    assert_eq!(owned_finder.needle(), b"search_term");
    assert_eq!(owned_finder.needle().len(), 11);

    let haystack = b"this is a search_term in text and another search_term here";
    let first = haystack.find(owned_finder.needle());
    assert_eq!(first, Some(10));

    let rfind = haystack.rfind(owned_finder.needle());
    assert_eq!(rfind, Some(42));
}

#[test]
fn test_finder_into_owned_clone_behavior() {
    let needle_bytes = b"clone_me";
    let finder = Finder::new(needle_bytes);
    assert_eq!(finder.needle(), b"clone_me");

    let owned = finder.into_owned();
    assert_eq!(owned.needle(), b"clone_me");


    let texts: Vec<&[u8]> = vec![
        b"no match here",
        b"clone_me is present",
        b"another line",
        b"clone_me again clone_me",
    ];

    let mut match_count = 0;
    for text in &texts {
        let count = text.find_iter(owned.needle()).count();
        match_count += count;
    }
    assert_eq!(match_count, 3);


    assert_eq!(owned.needle(), b"clone_me");
    assert_eq!(owned.needle().len(), 8);
}

#[test]
fn test_finder_needle_with_special_bytes() {

    let needle_with_null = b"a\x00b";
    let f1 = Finder::new(needle_with_null);
    assert_eq!(f1.needle(), b"a\x00b");
    assert_eq!(f1.needle().len(), 3);
    assert_eq!(f1.needle()[1], 0x00);

    let owned1 = f1.into_owned();
    assert_eq!(owned1.needle(), b"a\x00b");


    let needle_whitespace = b"\n\r\t";
    let f2 = Finder::new(needle_whitespace);
    assert_eq!(f2.needle(), &[b'\n', b'\r', b'\t']);
    let owned2 = f2.into_owned();
    assert_eq!(owned2.needle().len(), 3);
    assert_eq!(owned2.needle()[0], b'\n');


    let needle_ff = vec![0xFFu8; 10];
    let f3 = Finder::new(&needle_ff);
    assert_eq!(f3.needle().len(), 10);
    let owned3 = f3.into_owned();
    assert!(owned3.needle().iter().all(|&b| b == 0xFF));
}

#[test]
fn test_finder_into_owned_used_with_concat_and_join() {

    let parts: Vec<&[u8]> = vec![b"hello ", b"world ", b"hello ", b"rust"];
    let haystack = concat(parts);
    assert_eq!(&haystack, b"hello world hello rust");

    let finder = Finder::new(b"hello");
    let owned = finder.into_owned();
    assert_eq!(owned.needle(), b"hello");

    let positions: Vec<usize> = haystack.find_iter(owned.needle()).collect();
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0], 0);
    assert_eq!(positions[1], 12);


    let elements: Vec<&[u8]> = vec![b"one", b"two", b"three"];
    let joined = join(b"-", elements);
    assert_eq!(&joined, B("one-two-three"));

    let dash_finder = Finder::new(b"-").into_owned();
    assert_eq!(dash_finder.needle(), b"-");
    let dash_positions: Vec<usize> = joined.find_iter(dash_finder.needle()).collect();
    assert_eq!(dash_positions.len(), 2);
    assert_eq!(dash_positions[0], 3);
    assert_eq!(dash_positions[1], 7);
}

#[test]
fn test_finder_into_owned_with_decode_utf8_workflow() {

    let mixed = b"caf\xC3\xA9 \xFF end";


    let (ch, size) = decode_utf8(&mixed[..]);
    assert_eq!(ch, Some('c'));
    assert_eq!(size, 1);

    let (ch2, size2) = decode_utf8(&mixed[3..]);
    assert_eq!(ch2, Some('é'));
    assert_eq!(size2, 2);


    let (ch3, size3) = decode_utf8(&mixed[6..]);
    assert_eq!(ch3, None);
    assert_eq!(size3, 1);


    let (last_ch, last_size) = decode_last_utf8(&mixed[..]);
    assert_eq!(last_ch, Some('d'));
    assert_eq!(last_size, 1);


    let finder = Finder::new(b"end").into_owned();
    assert_eq!(finder.needle(), b"end");
    let pos = mixed.find(finder.needle());
    assert_eq!(pos, Some(8));
}