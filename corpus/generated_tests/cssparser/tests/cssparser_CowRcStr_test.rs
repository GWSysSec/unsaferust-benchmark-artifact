use cssparser::CowRcStr;

fn cow(value: &'static str) -> CowRcStr<'static> {
    CowRcStr::from(value)
}

fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }

    let mut index = index;
    while !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }

    let mut index = index;
    while !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

#[test]
fn cow_rc_str_utf8_boundaries_and_raw_views_match_string_semantics() {
    let s = cow("Löwe 老虎 Léopard");

    assert_eq!(s.len(), "Löwe 老虎 Léopard".len());
    assert!(!s.is_empty());
    assert!(s.is_char_boundary(0));
    assert!(s.is_char_boundary(6));
    assert!(s.is_char_boundary(s.len()));
    assert!(!s.is_char_boundary(2));
    assert!(!s.is_char_boundary(8));
    assert_eq!(s.as_bytes(), "Löwe 老虎 Léopard".as_bytes());
    assert_eq!(unsafe { *s.as_ptr() }, b'L');

    let emoji = cow("❤️🧡💛💚💙💜");
    let floor = floor_char_boundary(&emoji, 13);
    let ceil = ceil_char_boundary(&emoji, 13);

    assert_eq!(emoji.len(), 26);
    assert!(!emoji.is_char_boundary(13));
    assert_eq!(floor, 10);
    assert_eq!(ceil, 14);
    assert_eq!(unsafe { emoji.get_unchecked(0..floor) }, "❤️🧡");
    assert_eq!(unsafe { emoji.get_unchecked(0..ceil) }, "❤️🧡💛");

    let mountain = cow("🗻∈🌏");
    assert_eq!(unsafe { mountain.get_unchecked(0..4) }, "🗻");
    assert_eq!(unsafe { mountain.get_unchecked(4..7) }, "∈");
    assert_eq!(unsafe { mountain.get_unchecked(7..11) }, "🌏");

    let name = cow("Per Martin-Löf");
    let (first, rest) = name.split_at(3);
    assert_eq!(first, "Per");
    assert_eq!(rest, " Martin-Löf");
    assert_eq!(name.split_at_checked(3), Some(("Per", " Martin-Löf")));
    assert_eq!(name.split_at_checked(13), None);
    assert_eq!(name.split_at_checked(16), None);

    let empty = cow("");
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
}

#[test]
fn cow_rc_str_iterators_cover_chars_bytes_whitespace_lines_and_utf16() {
    let word = cow("goodbye");
    assert_eq!(word.chars().count(), 7);
    assert_eq!(
        word.chars().collect::<Vec<char>>(),
        vec!['g', 'o', 'o', 'd', 'b', 'y', 'e']
    );
    assert_eq!(
        word.char_indices().collect::<Vec<(usize, char)>>(),
        vec![(0, 'g'), (1, 'o'), (2, 'o'), (3, 'd'), (4, 'b'), (5, 'y'), (6, 'e')]
    );

    let bytes = cow("bors");
    assert_eq!(bytes.bytes().collect::<Vec<u8>>(), b"bors".to_vec());

    let unicode_space = cow("A\u{2003}few\twords\n");
    assert_eq!(
        unicode_space.split_whitespace().collect::<Vec<&str>>(),
        vec!["A", "few", "words"]
    );
    assert_eq!(
        unicode_space.split_ascii_whitespace().collect::<Vec<&str>>(),
        vec!["A\u{2003}few", "words"]
    );

    let text = cow("foo\r\nbar\n\nbaz\r");
    assert_eq!(
        text.lines().collect::<Vec<&str>>(),
        vec!["foo", "bar", "", "baz\r"]
    );
    assert_eq!(
        text.lines().collect::<Vec<&str>>(),
        vec!["foo", "bar", "", "baz\r"]
    );

    let polish = cow("Zażółć gęślą jaźń");
    let utf8_len = polish.len();
    let utf16: Vec<u16> = polish.encode_utf16().collect();
    assert!(utf16.len() <= utf8_len);
    assert_eq!(String::from_utf16(&utf16).unwrap(), "Zażółć gęślą jaźń");
}

#[test]
fn cow_rc_str_search_and_split_workflows_handle_forward_and_reverse_cases() {
    let animals = cow("Löwe 老虎 Léopard Gepardi");
    assert!(animals.ends_with("Gepardi"));
    assert!(!animals.ends_with("Léopard"));
    assert_eq!(animals.find('L'), Some(0));
    assert_eq!(animals.find('é'), Some(14));
    assert_eq!(animals.find("pard"), Some(17));
    assert_eq!(animals.rfind('L'), Some(13));
    assert_eq!(animals.rfind("pard"), Some(24));

    let sentence = cow("Mary had a little lamb");
    assert_eq!(
        sentence.split(' ').collect::<Vec<&str>>(),
        vec!["Mary", "had", "a", "little", "lamb"]
    );
    assert_eq!(
        sentence.rsplit(' ').collect::<Vec<&str>>(),
        vec!["lamb", "little", "a", "had", "Mary"]
    );
    assert_eq!(
        sentence.rsplitn(3, ' ').collect::<Vec<&str>>(),
        vec!["lamb", "little", "Mary had a"]
    );

    let lamb_lines = cow("Mary had a little lamb\nlittle lamb\nlittle lamb.");
    assert_eq!(
        lamb_lines.split_inclusive('\n').collect::<Vec<&str>>(),
        vec!["Mary had a little lamb\n", "little lamb\n", "little lamb."]
    );

    let terminated = cow("A.B:C.D");
    assert_eq!(
        terminated.split_terminator(&['.', ':'][..]).collect::<Vec<&str>>(),
        vec!["A", "B", "C", "D"]
    );
    assert_eq!(
        terminated.rsplit_terminator(&['.', ':'][..]).collect::<Vec<&str>>(),
        vec!["D", "C", "B", "A"]
    );

    let limited = cow("lionXXtigerXleopard");
    assert_eq!(
        limited.splitn(3, "X").collect::<Vec<&str>>(),
        vec!["lion", "", "tigerXleopard"]
    );
    assert_eq!(
        limited.rsplitn(3, 'X').collect::<Vec<&str>>(),
        vec!["leopard", "tiger", "lionX"]
    );

    let cfg = cow("cfg=foo=bar");
    assert_eq!(cfg.split_once('='), Some(("cfg", "foo=bar")));
    assert_eq!(cfg.rsplit_once('='), Some(("cfg=foo", "bar")));
    assert_eq!(cow("cfg").split_once('='), None);
    assert_eq!(cow("cfg").rsplit_once('='), None);
}

#[test]
fn cow_rc_str_match_iterators_report_disjoint_matches_and_indices() {
    let repeated = cow("abcXXXabcYYYabc");
    assert_eq!(
        repeated.matches("abc").collect::<Vec<&str>>(),
        vec!["abc", "abc", "abc"]
    );
    assert_eq!(
        repeated.rmatches("abc").collect::<Vec<&str>>(),
        vec!["abc", "abc", "abc"]
    );
    assert_eq!(
        repeated.match_indices("abc").collect::<Vec<(usize, &str)>>(),
        vec![(0, "abc"), (6, "abc"), (12, "abc")]
    );
    assert_eq!(
        repeated.rmatch_indices("abc").collect::<Vec<(usize, &str)>>(),
        vec![(12, "abc"), (6, "abc"), (0, "abc")]
    );

    let digits = cow("1abc2abc3");
    assert_eq!(
        digits.matches(char::is_numeric).collect::<Vec<&str>>(),
        vec!["1", "2", "3"]
    );
    assert_eq!(
        digits.rmatches(char::is_numeric).collect::<Vec<&str>>(),
        vec!["3", "2", "1"]
    );

    let overlapping = cow("ababa");
    assert_eq!(
        overlapping.match_indices("aba").collect::<Vec<(usize, &str)>>(),
        vec![(0, "aba")]
    );
    assert_eq!(
        overlapping.rmatch_indices("aba").collect::<Vec<(usize, &str)>>(),
        vec![(2, "aba")]
    );
}

#[test]
fn cow_rc_str_trimming_prefix_suffix_and_ascii_helpers_are_consistent() {
    let padded = cow("\n Hello\tworld\t\n");
    assert_eq!(padded.trim(), "Hello\tworld");
    assert_eq!(padded.trim_start(), "Hello\tworld\t\n");
    assert_eq!(padded.trim_end(), "\n Hello\tworld");

    let left_right = cow(" Hello\tworld\t");
    assert_eq!(left_right.trim_start(), "Hello\tworld\t");
    assert_eq!(left_right.trim_end(), " Hello\tworld");

    let numbered = cow("123foo1bar123");
    assert_eq!(numbered.trim_matches(char::is_numeric), "foo1bar");
    assert_eq!(numbered.trim_start_matches(char::is_numeric), "foo1bar123");
    assert_eq!(numbered.trim_end_matches(char::is_numeric), "123foo1bar");
    assert_eq!(numbered.trim_start_matches(char::is_numeric), "foo1bar123");
    assert_eq!(numbered.trim_end_matches(char::is_numeric), "123foo1bar");

    let prefixed = cow("foo:bar");
    assert_eq!(prefixed.strip_prefix("foo:"), Some("bar"));
    assert_eq!(prefixed.strip_prefix("bar"), None);

    let suffixed = cow("bar:foo");
    assert_eq!(suffixed.strip_suffix(":foo"), Some("bar"));
    assert_eq!(suffixed.strip_suffix("bar"), None);

    let ascii = cow("hello!\n");
    let non_ascii = cow("Grüße, Jürgen ❤");
    assert!(ascii.is_ascii());
    assert!(ascii.as_bytes().iter().all(u8::is_ascii));
    assert!(!non_ascii.is_ascii());
    assert!(!non_ascii.as_bytes().iter().all(u8::is_ascii));

    assert!(cow("Ferris").eq_ignore_ascii_case("FERRIS"));
    assert!(cow("Ferrös").eq_ignore_ascii_case("FERRöS"));
    assert!(!cow("Ferrös").eq_ignore_ascii_case("FERRÖS"));

    assert_eq!(cow(" \t \u{3000}hello world\n").trim_ascii_start(), "\u{3000}hello world\n");
    assert_eq!(cow("\r hello world\u{3000}\n ").trim_ascii_end(), "\r hello world\u{3000}");
    assert_eq!(cow(" \t hello world\n ").trim_ascii(), "hello world");
}

#[test]
fn cow_rc_str_escaping_replacement_case_mapping_and_repetition_allocate_expected_strings() {
    let escaped = cow("❤\n!");
    assert_eq!(escaped.escape_debug().to_string(), "❤\\n!");
    assert_eq!(escaped.escape_default().to_string(), "\\u{2764}\\n!");
    assert_eq!(escaped.escape_unicode().to_string(), "\\u{2764}\\u{a}\\u{21}");

    let old = cow("this is old");
    assert_eq!(old.replace("old", "new"), "this is new");
    assert_eq!(old.replace("is", "an"), "than an old");

    let repeated_words = cow("foo foo 123 foo");
    assert_eq!(repeated_words.replacen("foo", "new", 2), "new new 123 foo");
    assert_eq!(repeated_words.replacen('o', "a", 3), "faa fao 123 foo");
    assert_eq!(
        repeated_words.replacen(char::is_numeric, "new", 1),
        "foo foo new23 foo"
    );

    assert_eq!(cow("HELLO").to_lowercase(), "hello");
    assert_eq!(cow("hello").to_uppercase(), "HELLO");
    assert_eq!(cow("abc").repeat(4), String::from("abcabcabcabc"));

    let mixed = cow("Grüße, Jürgen ❤");
    assert_eq!(mixed.to_ascii_uppercase(), "GRüßE, JüRGEN ❤");
    assert_eq!(mixed.to_ascii_lowercase(), "grüße, jürgen ❤");
}