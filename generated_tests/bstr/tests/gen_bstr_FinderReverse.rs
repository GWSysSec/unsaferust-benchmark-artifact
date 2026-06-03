use bstr::FinderReverse;

#[test]
fn test_finder_reverse_into_owned_basic() {
    let needle = b"hello";
    let finder = FinderReverse::new(needle);


    assert_eq!(finder.needle(), b"hello");
    assert_eq!(finder.needle().len(), 5);


    let owned_finder: FinderReverse<'static> = finder.into_owned();


    assert_eq!(owned_finder.needle(), b"hello");
    assert_eq!(owned_finder.needle().len(), 5);
    assert_eq!(owned_finder.needle()[0], b'h');
    assert_eq!(owned_finder.needle()[4], b'o');
    assert_ne!(owned_finder.needle(), b"world");
}

#[test]
fn test_finder_reverse_into_owned_empty_needle() {
    let needle: &[u8] = b"";
    let finder = FinderReverse::new(needle);

    assert_eq!(finder.needle(), b"");
    assert_eq!(finder.needle().len(), 0);

    let owned_finder = finder.into_owned();

    assert_eq!(owned_finder.needle(), b"");
    assert_eq!(owned_finder.needle().len(), 0);
    assert!(owned_finder.needle().is_empty());


    let collected: Vec<u8> = owned_finder.needle().to_vec();
    assert_eq!(collected, Vec::<u8>::new());
    assert_eq!(collected.len(), 0);
}

#[test]
fn test_finder_reverse_into_owned_from_local_string() {

    let owned_finder: FinderReverse<'static>;
    {
        let local_data = String::from("pattern");
        let finder = FinderReverse::new(local_data.as_bytes());
        assert_eq!(finder.needle(), b"pattern");
        owned_finder = finder.into_owned();

    }


    assert_eq!(owned_finder.needle(), b"pattern");
    assert_eq!(owned_finder.needle().len(), 7);
    assert_eq!(owned_finder.needle()[0], b'p');
    assert_eq!(owned_finder.needle()[3], b't');
    assert_eq!(owned_finder.needle()[6], b'n');
    assert_ne!(owned_finder.needle(), b"other");
    assert_ne!(owned_finder.needle().len(), 0);
    assert_eq!(&owned_finder.needle()[..3], b"pat");
}

#[test]
fn test_finder_reverse_needle_with_special_bytes() {

    let needle: &[u8] = &[0x00, 0xFF, 0x80, 0x7F, 0x01];
    let finder = FinderReverse::new(needle);

    assert_eq!(finder.needle(), &[0x00, 0xFF, 0x80, 0x7F, 0x01]);
    assert_eq!(finder.needle().len(), 5);
    assert_eq!(finder.needle()[0], 0x00);
    assert_eq!(finder.needle()[1], 0xFF);
    assert_eq!(finder.needle()[2], 0x80);
    assert_eq!(finder.needle()[3], 0x7F);

    let owned = finder.into_owned();
    assert_eq!(owned.needle(), &[0x00, 0xFF, 0x80, 0x7F, 0x01]);
    assert_eq!(owned.needle()[4], 0x01);
    assert_eq!(owned.needle().len(), 5);
}

#[test]
fn test_finder_reverse_into_owned_large_needle() {

    let large_needle: Vec<u8> = (0..256).map(|i| (i % 256) as u8).collect();
    let finder = FinderReverse::new(&large_needle);

    assert_eq!(finder.needle().len(), 256);
    assert_eq!(finder.needle()[0], 0);
    assert_eq!(finder.needle()[127], 127);
    assert_eq!(finder.needle()[255], 255);

    let owned = finder.into_owned();

    assert_eq!(owned.needle().len(), 256);
    assert_eq!(owned.needle()[0], 0);
    assert_eq!(owned.needle()[127], 127);
    assert_eq!(owned.needle()[255], 255);
    assert_eq!(owned.needle(), large_needle.as_slice());
}

#[test]
fn test_finder_reverse_needle_single_byte() {
    let finder = FinderReverse::new(b"x");

    assert_eq!(finder.needle(), b"x");
    assert_eq!(finder.needle().len(), 1);
    assert_eq!(finder.needle()[0], b'x');

    let owned = finder.into_owned();
    assert_eq!(owned.needle(), b"x");
    assert_eq!(owned.needle().len(), 1);
    assert_eq!(owned.needle()[0], b'x');
    assert_ne!(owned.needle(), b"y");
    assert_ne!(owned.needle(), b"");
}

#[test]
fn test_finder_reverse_into_owned_multiple_conversions() {

    let mut finders: Vec<FinderReverse<'static>> = Vec::new();

    for i in 0u8..5 {
        let data = vec![i; (i as usize) + 1];
        let finder = FinderReverse::new(&data);
        assert_eq!(finder.needle().len(), (i as usize) + 1);
        finders.push(finder.into_owned());
    }


    assert_eq!(finders[0].needle(), &[0]);
    assert_eq!(finders[1].needle(), &[1, 1]);
    assert_eq!(finders[2].needle(), &[2, 2, 2]);
    assert_eq!(finders[3].needle(), &[3, 3, 3, 3]);
    assert_eq!(finders[4].needle(), &[4, 4, 4, 4, 4]);
    assert_eq!(finders.len(), 5);
    assert_eq!(finders[4].needle().len(), 5);
    assert_eq!(finders[0].needle().len(), 1);
}

#[test]
fn test_finder_reverse_needle_utf8_multibyte() {

    let emoji = "🦀🔥";
    let finder = FinderReverse::new(emoji.as_bytes());


    assert_eq!(finder.needle().len(), 8);
    assert_eq!(finder.needle(), emoji.as_bytes());

    let owned = finder.into_owned();
    assert_eq!(owned.needle().len(), 8);
    assert_eq!(owned.needle(), emoji.as_bytes());

    assert_eq!(owned.needle()[0], 0xF0);

    assert_eq!(owned.needle()[4], 0xF0);
    assert_ne!(owned.needle(), b"rust");
    assert_eq!(&owned.needle()[..4], "🦀".as_bytes());
}