use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, char, digit1, u8 as parse_u8};
use nom::combinator::map_res;
use nom::multi::{
    count, fill, fold_many0, fold_many1, fold_many_m_n, length_count, many0_count, many1_count,
    many_till, separated_list0, separated_list1,
};
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

#[test]
fn test_separated_list0_basic() {
    let mut parser = separated_list0(char(','), alpha1);
    let result: IResult<&str, Vec<&str>> = parser.parse("abc,def,ghi");
    let (remaining, items) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], "abc");
    assert_eq!(items[1], "def");
    assert_eq!(items[2], "ghi");


    let result2: IResult<&str, Vec<&str>> = separated_list0(char(','), alpha1).parse("");
    let (remaining2, items2) = result2.unwrap();
    assert_eq!(remaining2, "");
    assert_eq!(items2.len(), 0);


    let result3: IResult<&str, Vec<&str>> = separated_list0(char(','), alpha1).parse("hello");
    let (remaining3, items3) = result3.unwrap();
    assert_eq!(remaining3, "");
    assert_eq!(items3.len(), 1);
    assert_eq!(items3[0], "hello");
}

#[test]
fn test_separated_list0_trailing_separator() {

    let mut parser = separated_list0(char(','), alpha1);
    let result: IResult<&str, Vec<&str>> = parser.parse("abc,def,");
    let (remaining, items) = result.unwrap();
    assert_eq!(remaining, ",");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], "abc");
    assert_eq!(items[1], "def");


    let result2: IResult<&str, Vec<&str>> = separated_list0(char(','), alpha1).parse("123");
    let (remaining2, items2) = result2.unwrap();
    assert_eq!(remaining2, "123");
    assert_eq!(items2.len(), 0);
}

#[test]
fn test_separated_list1_basic() {
    let mut parser = separated_list1(char(';'), digit1);
    let result: IResult<&str, Vec<&str>> = parser.parse("1;22;333;4444");
    let (remaining, items) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(items.len(), 4);
    assert_eq!(items[0], "1");
    assert_eq!(items[1], "22");
    assert_eq!(items[2], "333");
    assert_eq!(items[3], "4444");


    let result2: IResult<&str, Vec<&str>> = separated_list1(char(';'), digit1).parse("42");
    let (remaining2, items2) = result2.unwrap();
    assert_eq!(remaining2, "");
    assert_eq!(items2.len(), 1);
    assert_eq!(items2[0], "42");
}

#[test]
fn test_separated_list1_fails_on_empty() {
    let mut parser = separated_list1(char(','), alpha1);
    let result: IResult<&str, Vec<&str>> = parser.parse("");
    assert!(result.is_err());


    let result2: IResult<&str, Vec<&str>> = separated_list1(char(','), alpha1).parse("123,abc");
    assert!(result2.is_err());


    assert!(result.is_err());
    assert!(result2.is_err());


    let result3: IResult<&str, Vec<&str>> = separated_list1(char(','), alpha1).parse("abc,def");
    assert!(result3.is_ok());
    let (rem, vals) = result3.unwrap();
    assert_eq!(rem, "");
    assert_eq!(vals, vec!["abc", "def"]);
}

#[test]
fn test_many0_count_basic() {
    let mut parser = many0_count(tag("ab"));
    let result: IResult<&str, usize> = parser.parse("ababab");
    let (remaining, count_val) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(count_val, 3);


    let result2: IResult<&str, usize> = many0_count(tag("ab")).parse("abababc");
    let (remaining2, count_val2) = result2.unwrap();
    assert_eq!(remaining2, "c");
    assert_eq!(count_val2, 3);


    let result3: IResult<&str, usize> = many0_count(tag("ab")).parse("xyz");
    let (remaining3, count_val3) = result3.unwrap();
    assert_eq!(remaining3, "xyz");
    assert_eq!(count_val3, 0);


    let result4: IResult<&str, usize> = many0_count(tag("ab")).parse("");
    let (remaining4, count_val4) = result4.unwrap();
    assert_eq!(remaining4, "");
    assert_eq!(count_val4, 0);
}

#[test]
fn test_many1_count_basic() {
    let mut parser = many1_count(tag("xy"));
    let result: IResult<&str, usize> = parser.parse("xyxyxyxy");
    let (remaining, count_val) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(count_val, 4);


    let result2: IResult<&str, usize> = many1_count(tag("xy")).parse("xyz");
    let (remaining2, count_val2) = result2.unwrap();
    assert_eq!(remaining2, "z");
    assert_eq!(count_val2, 1);


    let result3: IResult<&str, usize> = many1_count(tag("xy")).parse("abc");
    assert!(result3.is_err());


    let result4: IResult<&str, usize> = many1_count(tag("xy")).parse("");
    assert!(result4.is_err());


    let result5: IResult<&str, usize> = many1_count(tag("ab")).parse("ababcab");
    let (remaining5, count_val5) = result5.unwrap();
    assert_eq!(remaining5, "cab");
    assert_eq!(count_val5, 2);
}

#[test]
fn test_count_parser() {
    let mut parser = count(tag("abc"), 3);
    let result: IResult<&str, Vec<&str>> = parser.parse("abcabcabc");
    let (remaining, items) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], "abc");
    assert_eq!(items[1], "abc");
    assert_eq!(items[2], "abc");


    let result2: IResult<&str, Vec<&str>> = count(tag("ab"), 2).parse("ababxyz");
    let (remaining2, items2) = result2.unwrap();
    assert_eq!(remaining2, "xyz");
    assert_eq!(items2.len(), 2);
    assert_eq!(items2[0], "ab");
    assert_eq!(items2[1], "ab");


    let result3: IResult<&str, Vec<&str>> = count(tag("ab"), 3).parse("abab");
    assert!(result3.is_err());
}

#[test]
fn test_count_zero() {
    let mut parser = count(tag("x"), 0);
    let result: IResult<&str, Vec<&str>> = parser.parse("hello");
    let (remaining, items) = result.unwrap();
    assert_eq!(remaining, "hello");
    assert_eq!(items.len(), 0);


    let result2: IResult<&str, Vec<&str>> = count(tag("x"), 0).parse("");
    let (remaining2, items2) = result2.unwrap();
    assert_eq!(remaining2, "");
    assert_eq!(items2.len(), 0);


    let result3: IResult<&str, Vec<&str>> = count(tag("hi"), 1).parse("hiworld");
    let (remaining3, items3) = result3.unwrap();
    assert_eq!(remaining3, "world");
    assert_eq!(items3.len(), 1);
    assert_eq!(items3[0], "hi");
}

#[test]
fn test_many_till_basic() {
    let mut parser = many_till(tag("ab"), tag("cd"));
    let result: IResult<&str, (Vec<&str>, &str)> = parser.parse("ababcd");
    let (remaining, (items, terminator)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], "ab");
    assert_eq!(items[1], "ab");
    assert_eq!(terminator, "cd");


    let result2: IResult<&str, (Vec<&str>, &str)> = many_till(tag("ab"), tag("cd")).parse("cd");
    let (remaining2, (items2, terminator2)) = result2.unwrap();
    assert_eq!(remaining2, "");
    assert_eq!(items2.len(), 0);
    assert_eq!(terminator2, "cd");
}

#[test]
fn test_many_till_with_remaining() {
    let mut parser = many_till(tag("x"), tag("end"));
    let result: IResult<&str, (Vec<&str>, &str)> = parser.parse("xxxendmore");
    let (remaining, (items, terminator)) = result.unwrap();
    assert_eq!(remaining, "more");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], "x");
    assert_eq!(items[1], "x");
    assert_eq!(items[2], "x");
    assert_eq!(terminator, "end");


    let result2: IResult<&str, (Vec<&str>, &str)> = many_till(tag("x"), tag("end")).parse("xend");
    let (remaining2, (items2, terminator2)) = result2.unwrap();
    assert_eq!(remaining2, "");
    assert_eq!(items2.len(), 1);
    assert_eq!(items2[0], "x");
    assert_eq!(terminator2, "end");
}

#[test]
fn test_fill_basic() {
    let mut buf = [""; 3];
    {
        let mut parser = fill(tag("ab"), &mut buf);
        let result: IResult<&str, ()> = parser.parse("ababab");
        let (remaining, _) = result.unwrap();
        assert_eq!(remaining, "");
    }
    assert_eq!(buf[0], "ab");
    assert_eq!(buf[1], "ab");
    assert_eq!(buf[2], "ab");


    let mut buf2 = [""; 2];
    {
        let mut parser2 = fill(tag("xy"), &mut buf2);
        let result2: IResult<&str, ()> = parser2.parse("xyxyrest");
        let (remaining2, _) = result2.unwrap();
        assert_eq!(remaining2, "rest");
    }
    assert_eq!(buf2[0], "xy");
    assert_eq!(buf2[1], "xy");
}

#[test]
fn test_fill_fails_insufficient_input() {
    let mut buf = [""; 3];
    {
        let mut parser = fill(tag("ab"), &mut buf);
        let result: IResult<&str, ()> = parser.parse("abab");
        assert!(result.is_err());
    }



    assert_eq!(buf[0], "ab");
    assert_eq!(buf[1], "ab");
    assert_eq!(buf[2], "");


    let mut empty_buf: [&str; 0] = [];
    {
        let mut parser2 = fill(tag("ab"), &mut empty_buf);
        let result2: IResult<&str, ()> = parser2.parse("anything");
        let (remaining2, _) = result2.unwrap();
        assert_eq!(remaining2, "anything");
    }
}

#[test]
fn test_fold_many0_basic() {
    let mut parser = fold_many0(
        tag("ab"),
        || 0usize,
        |acc, _item: &str| acc + 1,
    );
    let result: IResult<&str, usize> = parser.parse("ababab");
    let (remaining, total) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(total, 3);


    let result2: IResult<&str, usize> = fold_many0(
        tag("ab"),
        || 0usize,
        |acc, _item: &str| acc + 1,
    ).parse("xyz");
    let (remaining2, total2) = result2.unwrap();
    assert_eq!(remaining2, "xyz");
    assert_eq!(total2, 0);


    let result3: IResult<&str, usize> = fold_many0(
        alpha1,
        || 0usize,
        |acc, item: &str| acc + item.len(),
    ).parse("");
    let (remaining3, total3) = result3.unwrap();
    assert_eq!(remaining3, "");
    assert_eq!(total3, 0);
}

#[test]
fn test_fold_many0_sum_digits() {
    fn parse_num(input: &str) -> IResult<&str, u32> {
        map_res(digit1, |s: &str| s.parse::<u32>()).parse(input)
    }

    let mut parser = fold_many0(
        preceded(char(','), parse_num),
        || 0u32,
        |acc, val| acc + val,
    );
    let result: IResult<&str, u32> = parser.parse(",1,2,3,4");
    let (remaining, sum) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(sum, 10);


    let result2: IResult<&str, u32> = fold_many0(
        preceded(char(','), parse_num),
        || 0u32,
        |acc, val| acc + val,
    ).parse("nope");
    let (remaining2, sum2) = result2.unwrap();
    assert_eq!(remaining2, "nope");
    assert_eq!(sum2, 0);
}

#[test]
fn test_fold_many1_basic() {
    let mut parser = fold_many1(
        tag("x"),
        || String::new(),
        |mut acc, item: &str| {
            acc.push_str(item);
            acc
        },
    );
    let result: IResult<&str, String> = parser.parse("xxxyz");
    let (remaining, accumulated) = result.unwrap();
    assert_eq!(remaining, "yz");
    assert_eq!(accumulated, "xxx");
    assert_eq!(accumulated.len(), 3);


    let result2: IResult<&str, String> = fold_many1(
        tag("x"),
        || String::new(),
        |mut acc, item: &str| {
            acc.push_str(item);
            acc
        },
    ).parse("yz");
    assert!(result2.is_err());


    let result3: IResult<&str, String> = fold_many1(
        tag("hello"),
        || String::new(),
        |mut acc, item: &str| {
            acc.push_str(item);
            acc
        },
    ).parse("helloworld");
    let (remaining3, accumulated3) = result3.unwrap();
    assert_eq!(remaining3, "world");
    assert_eq!(accumulated3, "hello");
}

#[test]
fn test_fold_many_m_n_basic() {
    let mut parser = fold_many_m_n(
        2,
        4,
        tag("ab"),
        || Vec::new(),
        |mut acc: Vec<&str>, item| {
            acc.push(item);
            acc
        },
    );


    let result: IResult<&str, Vec<&str>> = parser.parse("ababab");
    let (remaining, items) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], "ab");


    let result2: IResult<&str, Vec<&str>> = fold_many_m_n(
        2,
        4,
        tag("ab"),
        || Vec::new(),
        |mut acc: Vec<&str>, item| {
            acc.push(item);
            acc
        },
    ).parse("ababababab");
    let (remaining2, items2) = result2.unwrap();
    assert_eq!(remaining2, "ab");
    assert_eq!(items2.len(), 4);


    let result3: IResult<&str, Vec<&str>> = fold_many_m_n(
        2,
        4,
        tag("ab"),
        || Vec::new(),
        |mut acc: Vec<&str>, item| {
            acc.push(item);
            acc
        },
    ).parse("abxyz");
    assert!(result3.is_err());
}

#[test]
fn test_fold_many_m_n_zero_min() {
    let mut parser = fold_many_m_n(
        0,
        3,
        tag("z"),
        || 0u32,
        |acc, _: &str| acc + 1,
    );


    let result: IResult<&str, u32> = parser.parse("abc");
    let (remaining, count_val) = result.unwrap();
    assert_eq!(remaining, "abc");
    assert_eq!(count_val, 0);


    let result2: IResult<&str, u32> = fold_many_m_n(
        0,
        3,
        tag("z"),
        || 0u32,
        |acc, _: &str| acc + 1,
    ).parse("zzz");
    let (remaining2, count_val2) = result2.unwrap();
    assert_eq!(remaining2, "");
    assert_eq!(count_val2, 3);


    let result3: IResult<&str, u32> = fold_many_m_n(
        0,
        3,
        tag("z"),
        || 0u32,
        |acc, _: &str| acc + 1,
    ).parse("zzzzzz");
    let (remaining3, count_val3) = result3.unwrap();
    assert_eq!(remaining3, "zzz");
    assert_eq!(count_val3, 3);
}

#[test]
fn test_length_count_basic() {
    fn parse_count(input: &[u8]) -> IResult<&[u8], u8> {
        parse_u8(input)
    }

    fn parse_item(input: &[u8]) -> IResult<&[u8], &[u8]> {
        nom::bytes::complete::tag(&b"ab"[..])(input)
    }

    let mut parser = length_count(parse_count, parse_item);
    let input: &[u8] = b"3ababab";
    let result: IResult<&[u8], Vec<&[u8]>> = parser.parse(input);
    let (remaining, items) = result.unwrap();
    assert_eq!(remaining, b"");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], b"ab");
    assert_eq!(items[1], b"ab");
    assert_eq!(items[2], b"ab");


    let input2: &[u8] = b"0abab";
    let result2: IResult<&[u8], Vec<&[u8]>> = length_count(parse_count, parse_item).parse(input2);
    let (remaining2, items2) = result2.unwrap();
    assert_eq!(remaining2, b"abab");
    assert_eq!(items2.len(), 0);
}

#[test]
fn test_length_count_str() {
    use nom::character::complete::u8 as char_u8;
    use nom::bytes::complete::take_while1;

    fn parse_lowercase_word(input: &str) -> IResult<&str, &str> {
        take_while1(|c: char| c.is_lowercase())(input)
    }



    let mut parser = length_count(char_u8, preceded(char(','), parse_lowercase_word));
    let result: IResult<&str, Vec<&str>> = parser.parse("3,hello,world,foo");
    let (remaining, items) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], "hello");
    assert_eq!(items[1], "world");
    assert_eq!(items[2], "foo");



    let result2: IResult<&str, Vec<&str>> = length_count(char_u8, preceded(char(','), parse_lowercase_word)).parse("2,ab,cdEXTRA");
    let (remaining2, items2) = result2.unwrap();
    assert_eq!(remaining2, "EXTRA");
    assert_eq!(items2.len(), 2);
    assert_eq!(items2[0], "ab");
    assert_eq!(items2[1], "cd");
}