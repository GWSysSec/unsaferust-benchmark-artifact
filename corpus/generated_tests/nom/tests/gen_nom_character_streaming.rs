
use nom::character::streaming::{satisfy, tab, digit0, hex_digit0, oct_digit0, bin_digit0, alphanumeric0};
use nom::IResult;
use nom::Err;
use nom::error::ErrorKind;

#[test]
fn test_streaming_satisfy_basic_matching() {
    let result: IResult<&str, char> = satisfy(|c| c.is_uppercase())("Hello");
    assert!(result.is_ok());
    let (remaining, matched) = result.unwrap();
    assert_eq!(matched, 'H');
    assert_eq!(remaining, "ello");

    let result2: IResult<&str, char> = satisfy(|c| c == 'a')("abc");
    assert!(result2.is_ok());
    let (remaining2, matched2) = result2.unwrap();
    assert_eq!(matched2, 'a');
    assert_eq!(remaining2, "bc");

    let result3: IResult<&str, char> = satisfy(|c| c.is_digit(10))("9xyz");
    assert!(result3.is_ok());
    let (remaining3, matched3) = result3.unwrap();
    assert_eq!(matched3, '9');
    assert_eq!(remaining3, "xyz");
}

#[test]
fn test_streaming_satisfy_failure_and_incomplete() {
    // When the character doesn't match the predicate, it should error
    let result: IResult<&str, char> = satisfy(|c| c.is_uppercase())("hello");
    assert!(result.is_err());
    match result {
        Err(Err::Error(e)) => {
            assert_eq!(e.code, ErrorKind::Satisfy);
            assert_eq!(e.input, "hello");
        }
        _ => panic!("Expected Error, got {:?}", result),
    }

    // Empty input in streaming mode should return Incomplete
    let result_empty: IResult<&str, char> = satisfy(|c| c.is_alphabetic())("");
    assert!(result_empty.is_err());
    match result_empty {
        Err(Err::Incomplete(_)) => {}
        _ => panic!("Expected Incomplete for empty input, got {:?}", result_empty),
    }

    // Satisfy with a predicate that rejects the first char
    let result_reject: IResult<&str, char> = satisfy(|c| c == 'z')("abc");
    assert!(result_reject.is_err());
    match result_reject {
        Err(Err::Error(e)) => {
            assert_eq!(e.code, ErrorKind::Satisfy);
        }
        _ => panic!("Expected Error for non-matching char"),
    }
}

#[test]
fn test_streaming_tab_success_and_failure() {
    let result: IResult<&str, char> = tab("\thello");
    assert!(result.is_ok());
    let (remaining, matched) = result.unwrap();
    assert_eq!(matched, '\t');
    assert_eq!(remaining, "hello");

    // Multiple tabs - only consumes one
    let result2: IResult<&str, char> = tab("\t\tworld");
    assert!(result2.is_ok());
    let (remaining2, matched2) = result2.unwrap();
    assert_eq!(matched2, '\t');
    assert_eq!(remaining2, "\tworld");

    // Non-tab input should fail
    let result3: IResult<&str, char> = tab("hello");
    assert!(result3.is_err());
    match result3 {
        Err(Err::Error(e)) => {
            assert_eq!(e.input, "hello");
        }
        _ => panic!("Expected Error for non-tab input"),
    }

    // Empty input should be Incomplete in streaming
    let result4: IResult<&str, char> = tab("");
    assert!(result4.is_err());
    match result4 {
        Err(Err::Incomplete(_)) => {}
        _ => panic!("Expected Incomplete for empty input, got {:?}", result4),
    }
}

#[test]
fn test_streaming_digit0_various_inputs() {
    // All digits
    let result: IResult<&str, &str> = digit0("12345abc");
    assert!(result.is_ok());
    let (remaining, matched) = result.unwrap();
    assert_eq!(matched, "12345");
    assert_eq!(remaining, "abc");

    // No digits at start - digit0 returns empty match
    let result2: IResult<&str, &str> = digit0("abc123");
    assert!(result2.is_ok());
    let (remaining2, matched2) = result2.unwrap();
    assert_eq!(matched2, "");
    assert_eq!(remaining2, "abc123");

    // Mixed content
    let result3: IResult<&str, &str> = digit0("007bond");
    assert!(result3.is_ok());
    let (remaining3, matched3) = result3.unwrap();
    assert_eq!(matched3, "007");
    assert_eq!(remaining3, "bond");

    // All digits - streaming returns Incomplete since it can't know if more digits follow
    let result4: IResult<&str, &str> = digit0("999");
    assert!(result4.is_err());
    match result4 {
        Err(Err::Incomplete(_)) => {}
        _ => panic!("Expected Incomplete for all-digit input in streaming, got {:?}", result4),
    }
}

#[test]
fn test_streaming_hex_digit0_various_inputs() {
    // Valid hex digits followed by non-hex
    let result: IResult<&str, &str> = hex_digit0("1a2bFFgg");
    assert!(result.is_ok());
    let (remaining, matched) = result.unwrap();
    assert_eq!(matched, "1a2bFF");
    assert_eq!(remaining, "gg");

    // No hex digits at start
    let result2: IResult<&str, &str> = hex_digit0("xyz123");
    assert!(result2.is_ok());
    let (remaining2, matched2) = result2.unwrap();
    assert_eq!(matched2, "");
    assert_eq!(remaining2, "xyz123");

    // Uppercase hex
    let result3: IResult<&str, &str> = hex_digit0("DEADBEEF rest");
    assert!(result3.is_ok());
    let (remaining3, matched3) = result3.unwrap();
    assert_eq!(matched3, "DEADBEEF");
    assert_eq!(remaining3, " rest");

    // All hex - streaming Incomplete
    let result4: IResult<&str, &str> = hex_digit0("cafe");
    assert!(result4.is_err());
    match result4 {
        Err(Err::Incomplete(_)) => {}
        _ => panic!("Expected Incomplete for all-hex input in streaming, got {:?}", result4),
    }
}

#[test]
fn test_streaming_oct_digit0_various_inputs() {
    // Valid octal digits followed by non-octal
    let result: IResult<&str, &str> = oct_digit0("01234567abc");
    assert!(result.is_ok());
    let (remaining, matched) = result.unwrap();
    assert_eq!(matched, "01234567");
    assert_eq!(remaining, "abc");

    // 8 and 9 are not octal
    let result2: IResult<&str, &str> = oct_digit0("12389");
    assert!(result2.is_ok());
    let (remaining2, matched2) = result2.unwrap();
    assert_eq!(matched2, "123");
    assert_eq!(remaining2, "89");

    // No octal digits
    let result3: IResult<&str, &str> = oct_digit0("abc");
    assert!(result3.is_ok());
    let (remaining3, matched3) = result3.unwrap();
    assert_eq!(matched3, "");
    assert_eq!(remaining3, "abc");

    // All octal - streaming Incomplete
    let result4: IResult<&str, &str> = oct_digit0("0777");
    assert!(result4.is_err());
    match result4 {
        Err(Err::Incomplete(_)) => {}
        _ => panic!("Expected Incomplete for all-octal input in streaming, got {:?}", result4),
    }
}

#[test]
fn test_streaming_bin_digit0_various_inputs() {
    // Valid binary digits followed by non-binary
    let result: IResult<&str, &str> = bin_digit0("101010abc");
    assert!(result.is_ok());
    let (remaining, matched) = result.unwrap();
    assert_eq!(matched, "101010");
    assert_eq!(remaining, "abc");

    // 2 is not binary
    let result2: IResult<&str, &str> = bin_digit0("110210");
    assert!(result2.is_ok());
    let (remaining2, matched2) = result2.unwrap();
    assert_eq!(matched2, "110");
    assert_eq!(remaining2, "210");

    // No binary digits
    let result3: IResult<&str, &str> = bin_digit0("hello");
    assert!(result3.is_ok());
    let (remaining3, matched3) = result3.unwrap();
    assert_eq!(matched3, "");
    assert_eq!(remaining3, "hello");

    // All binary - streaming Incomplete
    let result4: IResult<&str, &str> = bin_digit0("1100");
    assert!(result4.is_err());
    match result4 {
        Err(Err::Incomplete(_)) => {}
        _ => panic!("Expected Incomplete for all-binary input in streaming, got {:?}", result4),
    }
}

#[test]
fn test_streaming_alphanumeric0_various_inputs() {
    // Alphanumeric followed by special chars
    let result: IResult<&str, &str> = alphanumeric0("hello123!world");
    assert!(result.is_ok());
    let (remaining, matched) = result.unwrap();
    assert_eq!(matched, "hello123");
    assert_eq!(remaining, "!world");

    // Starts with special char
    let result2: IResult<&str, &str> = alphanumeric0("!hello");
    assert!(result2.is_ok());
    let (remaining2, matched2) = result2.unwrap();
    assert_eq!(matched2, "");
    assert_eq!(remaining2, "!hello");

    // Space terminates
    let result3: IResult<&str, &str> = alphanumeric0("abc def");
    assert!(result3.is_ok());
    let (remaining3, matched3) = result3.unwrap();
    assert_eq!(matched3, "abc");
    assert_eq!(remaining3, " def");

    // All alphanumeric - streaming Incomplete
    let result4: IResult<&str, &str> = alphanumeric0("test42");
    assert!(result4.is_err());
    match result4 {
        Err(Err::Incomplete(_)) => {}
        _ => panic!("Expected Incomplete for all-alphanumeric input in streaming, got {:?}", result4),
    }
}

#[test]
fn test_streaming_satisfy_multibyte_unicode() {
    // Multi-byte unicode character
    let result: IResult<&str, char> = satisfy(|c| c == 'é')("élan");
    assert!(result.is_ok());
    let (remaining, matched) = result.unwrap();
    assert_eq!(matched, 'é');
    assert_eq!(remaining, "lan");

    // CJK character
    let result2: IResult<&str, char> = satisfy(|c| c > '\u{4E00}')("中文");
    assert!(result2.is_ok());
    let (remaining2, matched2) = result2.unwrap();
    assert_eq!(matched2, '中');
    assert_eq!(remaining2, "文");

    // Emoji
    let result3: IResult<&str, char> = satisfy(|c| !c.is_ascii())("🎉party");
    assert!(result3.is_ok());
    let (remaining3, matched3) = result3.unwrap();
    assert_eq!(matched3, '🎉');
    assert_eq!(remaining3, "party");

    // Predicate checking for specific unicode category
    let result4: IResult<&str, char> = satisfy(|c| c.is_alphabetic())("ñoño");
    assert!(result4.is_ok());
    let (remaining4, matched4) = result4.unwrap();
    assert_eq!(matched4, 'ñ');
    assert_eq!(remaining4, "oño");
}

#[test]
fn test_streaming_combined_workflow() {
    // Simulate parsing a simple binary protocol header:
    // digit field, then tab separator, then hex field

    let input = "42\tFF rest";

    // Parse digits
    let result1: IResult<&str, &str> = digit0(input);
    assert!(result1.is_ok());
    let (after_digits, digits) = result1.unwrap();
    assert_eq!(digits, "42");
    assert_eq!(after_digits, "\tFF rest");

    // Parse tab
    let result2: IResult<&str, char> = tab(after_digits);
    assert!(result2.is_ok());
    let (after_tab, tab_char) = result2.unwrap();
    assert_eq!(tab_char, '\t');
    assert_eq!(after_tab, "FF rest");

    // Parse hex digits
    let result3: IResult<&str, &str> = hex_digit0(after_tab);
    assert!(result3.is_ok());
    let (after_hex, hex_val) = result3.unwrap();
    assert_eq!(hex_val, "FF");
    assert_eq!(after_hex, " rest");
}