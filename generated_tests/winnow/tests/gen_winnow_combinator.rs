use winnow::Parser;
use winnow::ascii::{dec_uint, digit1};
use winnow::combinator::{backtrack_err, cond, fill, iterator, separated_foldl1, separated_foldr1};
use winnow::error::{ContextError, ErrMode, InputError};
use winnow::token::literal;

#[test]
fn test_separated_foldl1_subtraction() {

    let mut parser = separated_foldl1(
        digit1::<&str, ContextError>.parse_to::<i32>(),
        literal('-'),
        |a, _, b| a - b,
    );
    let mut input = "10-3-2";
    let r = parser.parse_next(&mut input).unwrap();
    assert_eq!(r, 5);
    assert_eq!(input, "");

    let mut input2 = "42";
    let r2 = parser.parse_next(&mut input2).unwrap();
    assert_eq!(r2, 42);
    assert_eq!(input2, "");

    let mut input3 = "1-2-3-4-5";
    let r3 = parser.parse_next(&mut input3).unwrap();
    assert_eq!(r3, -13);
    assert_eq!(input3, "");

    let mut bad = "abc";
    let rbad: Result<i32, _> = parser.parse_next(&mut bad);
    assert!(rbad.is_err());
}

#[test]
fn test_separated_foldr1_subtraction() {

    let mut parser = separated_foldr1(
        digit1::<&str, ContextError>.parse_to::<i32>(),
        literal('-'),
        |a, _, b| a - b,
    );
    let mut input = "10-3-2";
    let r = parser.parse_next(&mut input).unwrap();
    assert_eq!(r, 9);
    assert_eq!(input, "");

    let mut input2 = "100";
    let r2 = parser.parse_next(&mut input2).unwrap();
    assert_eq!(r2, 100);

    let mut input3 = "1-2-3-4-5";

    let r3 = parser.parse_next(&mut input3).unwrap();
    assert_eq!(r3, 3);
    assert_eq!(input3, "");

    let mut bad = "";
    let rbad: Result<i32, _> = parser.parse_next(&mut bad);
    assert!(rbad.is_err());
}

#[test]
fn test_fill_into_buffer() {
    let mut buf3 = [0u32; 3];
    let mut input3 = "10,20,30,tail";
    let res3: Result<(), ContextError> = fill(
        (dec_uint::<&str, u32, ContextError>, literal(",")).map(|(n, _)| n),
        &mut buf3,
    )
    .parse_next(&mut input3);
    assert!(res3.is_ok());
    assert_eq!(buf3, [10u32, 20, 30]);
    assert_eq!(input3, "tail");


    let mut buf4 = [0u32; 5];
    let mut input4 = "1,2,3,";
    let res4: Result<(), ContextError> = fill(
        (dec_uint::<&str, u32, ContextError>, literal(",")).map(|(n, _)| n),
        &mut buf4,
    )
    .parse_next(&mut input4);
    assert!(res4.is_err());


    let mut buf5: [u32; 0] = [];
    let mut input5 = "abc";
    let res5: Result<(), ContextError> =
        fill(dec_uint::<&str, u32, ContextError>, &mut buf5).parse_next(&mut input5);
    assert!(res5.is_ok());
    assert_eq!(input5, "abc");
    assert_eq!(buf5.len(), 0);
}

#[test]
fn test_cond_true_and_false() {
    let mut p_true = cond(true, digit1::<&str, ContextError>);
    let mut input = "123rest";
    let r = p_true.parse_next(&mut input).unwrap();
    assert_eq!(r, Some("123"));
    assert_eq!(input, "rest");

    let mut p_false = cond(false, digit1::<&str, ContextError>);
    let mut input2 = "123rest";
    let r2 = p_false.parse_next(&mut input2).unwrap();
    assert_eq!(r2, None);
    assert_eq!(input2, "123rest");


    let mut input3 = "abc";
    let r3: Result<Option<&str>, ContextError> =
        cond(true, digit1::<_, ContextError>).parse_next(&mut input3);
    assert!(r3.is_err());
    assert_eq!(input3, "abc");


    let mut input4 = "abc";
    let r4: Result<Option<&str>, ContextError> =
        cond(false, digit1::<_, ContextError>).parse_next(&mut input4);
    assert_eq!(r4.unwrap(), None);
    assert_eq!(input4, "abc");
}

#[test]
fn test_backtrack_err_demotes_cut() {
    use winnow::combinator::{alt, cut_err};


    let mut p_cut = alt((
        (
            literal::<_, _, ErrMode<InputError<&str>>>("ab"),
            cut_err(literal("XY")),
        )
            .map(|_| 1),
        literal::<_, _, ErrMode<InputError<&str>>>("abZ").map(|_| 2),
    ));
    let mut input = "abZ";
    let r: Result<i32, ErrMode<InputError<&str>>> = p_cut.parse_next(&mut input);
    assert!(r.is_err());


    let mut p_bt = alt((
        backtrack_err((
            literal::<_, _, ErrMode<InputError<&str>>>("ab"),
            cut_err(literal("XY")),
        ))
        .map(|_| 1),
        literal::<_, _, ErrMode<InputError<&str>>>("abZ").map(|_| 2),
    ));
    let mut input2 = "abZ";
    let r2: Result<i32, ErrMode<InputError<&str>>> = p_bt.parse_next(&mut input2);
    assert_eq!(r2.unwrap(), 2);
    assert_eq!(input2, "");


    let mut input3 = "abXYrest";
    let r3: Result<i32, ErrMode<InputError<&str>>> = alt((
        backtrack_err((
            literal::<_, _, ErrMode<InputError<&str>>>("ab"),
            cut_err(literal("XY")),
        ))
        .map(|_| 1),
        literal::<_, _, ErrMode<InputError<&str>>>("abZ").map(|_| 2),
    ))
    .parse_next(&mut input3);
    assert_eq!(r3.unwrap(), 1);
    assert_eq!(input3, "rest");
}

#[test]
#[should_panic]
fn test_todo_panics() {
    let mut input = "anything";
    let _: Result<(), ContextError> = winnow::combinator::todo(&mut input);
}

#[test]
fn test_iterator_basic() {
    let input = "1,2,3,4,5,end";
    let mut it = iterator(
        input,
        (
            dec_uint::<&str, u32, ContextError>,
            cond(true, literal(",")),
        )
            .map(|(n, _)| n),
    );
    let collected: Vec<u32> = (&mut it).collect();
    let res = it.finish();
    assert_eq!(collected, vec![1, 2, 3, 4, 5]);
    let (remaining, _) = res.unwrap();
    assert_eq!(remaining, "end");
    assert_eq!(collected.len(), 5);
    assert_eq!(collected.iter().sum::<u32>(), 15);


    let input2 = "abc";
    let mut it2 = iterator(input2, dec_uint::<&str, u32, ContextError>);
    let v2: Vec<u32> = (&mut it2).collect();
    assert_eq!(v2.len(), 0);
    let (rem2, _) = it2.finish().unwrap();
    assert_eq!(rem2, "abc");
}