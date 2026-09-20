use nom::character::complete::{satisfy, tab, digit0, hex_digit0, oct_digit0, bin_digit0, alphanumeric0};
use nom::character::complete::{digit1, alpha1, space0, char as nom_char};
use nom::bytes::complete::tag;
use nom::sequence::preceded;
use nom::branch::alt;
use nom::IResult;
use nom::Parser;

#[test]
fn test_satisfy_basic_predicates() {
    // satisfy with uppercase check
    let mut upper = satisfy(|c: char| c.is_uppercase());
    let result: IResult<&str, char> = upper("Hello");
    assert_eq!(result, Ok(("ello", 'H')));

    // satisfy with digit check
    let mut is_digit = satisfy(|c: char| c.is_ascii_digit());
    let result: IResult<&str, char> = is_digit("9abc");
    assert_eq!(result, Ok(("abc", '9')));

    // satisfy fails when predicate doesn't match
    let mut upper2 = satisfy(|c: char| c.is_uppercase());
    let result: IResult<&str, char> = upper2("lowercase");
    assert!(result.is_err());

    // satisfy with specific character
    let mut is_a = satisfy(|c: char| c == 'a');
    let result: IResult<&str, char> = is_a("abc");
    assert_eq!(result, Ok(("bc", 'a')));

    // satisfy on empty input fails
    let mut any_pred = satisfy(|c: char| c.is_alphabetic());
    let result: IResult<&str, char> = any_pred("");
    assert!(result.is_err());

    // satisfy with unicode
    let mut is_alpha = satisfy(|c: char| c.is_alphabetic());
    let result: IResult<&str, char> = is_alpha("über");
    assert_eq!(result, Ok(("ber", 'ü')));

    // satisfy with punctuation check
    let mut is_punct = satisfy(|c: char| c.is_ascii_punctuation());
    let result: IResult<&str, char> = is_punct("!hello");
    assert_eq!(result, Ok(("hello", '!')));

    // satisfy consumes exactly one char
    let mut any_char = satisfy(|_| true);
    let result: IResult<&str, char> = any_char("xy");
    assert_eq!(result, Ok(("y", 'x')));
}

#[test]
fn test_tab_parser() {
    // tab succeeds on tab character
    let result: IResult<&str, char> = tab("\trest");
    assert_eq!(result, Ok(("rest", '\t')));

    // tab fails on space
    let result: IResult<&str, char> = tab(" nope");
    assert!(result.is_err());

    // tab fails on empty input
    let result: IResult<&str, char> = tab("");
    assert!(result.is_err());

    // tab with only tab character
    let result: IResult<&str, char> = tab("\t");
    assert_eq!(result, Ok(("", '\t')));

    // tab followed by more tabs
    let result: IResult<&str, char> = tab("\t\t\t");
    assert_eq!(result, Ok(("\t\t", '\t')));

    // tab fails on newline
    let result: IResult<&str, char> = tab("\n");
    assert!(result.is_err());

    // tab fails on regular character
    let result: IResult<&str, char> = tab("a");
    assert!(result.is_err());

    // tab with content after
    let result: IResult<&str, char> = tab("\thello world");
    assert_eq!(result, Ok(("hello world", '\t')));
}

#[test]
fn test_digit0_various_inputs() {
    // digit0 on pure digits
    let result: IResult<&str, &str> = digit0("12345abc");
    assert_eq!(result, Ok(("abc", "12345")));

    // digit0 on no digits returns empty match
    let result: IResult<&str, &str> = digit0("abc");
    assert_eq!(result, Ok(("abc", "")));

    // digit0 on empty input returns empty match
    let result: IResult<&str, &str> = digit0("");
    assert_eq!(result, Ok(("", "")));

    // digit0 consumes all digits
    let result: IResult<&str, &str> = digit0("9876543210");
    assert_eq!(result, Ok(("", "9876543210")));

    // digit0 stops at first non-digit
    let result: IResult<&str, &str> = digit0("42xyz99");
    assert_eq!(result, Ok(("xyz99", "42")));

    // digit0 with leading zeros
    let result: IResult<&str, &str> = digit0("007bond");
    assert_eq!(result, Ok(("bond", "007")));

    // digit0 with single digit
    let result: IResult<&str, &str> = digit0("5!");
    assert_eq!(result, Ok(("!", "5")));

    // digit0 stops at dot (not a digit)
    let result: IResult<&str, &str> = digit0("123.456");
    assert_eq!(result, Ok((".456", "123")));
}

#[test]
fn test_hex_digit0_various_inputs() {
    // hex_digit0 on valid hex
    let result: IResult<&str, &str> = hex_digit0("1a2B3cXYZ");
    assert_eq!(result, Ok(("XYZ", "1a2B3c")));

    // hex_digit0 on empty input
    let result: IResult<&str, &str> = hex_digit0("");
    assert_eq!(result, Ok(("", "")));

    // hex_digit0 on non-hex input
    let result: IResult<&str, &str> = hex_digit0("ghijk");
    assert_eq!(result, Ok(("ghijk", "")));

    // hex_digit0 full hex string
    let result: IResult<&str, &str> = hex_digit0("DEADBEEF");
    assert_eq!(result, Ok(("", "DEADBEEF")));

    // hex_digit0 mixed case
    let result: IResult<&str, &str> = hex_digit0("aAbBcCdDeEfF");
    assert_eq!(result, Ok(("", "aAbBcCdDeEfF")));

    // hex_digit0 stops at 'g'
    let result: IResult<&str, &str> = hex_digit0("0123456789abcdefg");
    assert_eq!(result, Ok(("g", "0123456789abcdef")));

    // hex_digit0 with only digits
    let result: IResult<&str, &str> = hex_digit0("999");
    assert_eq!(result, Ok(("", "999")));

    // hex_digit0 stops at space
    let result: IResult<&str, &str> = hex_digit0("FF rest");
    assert_eq!(result, Ok((" rest", "FF")));
}

#[test]
fn test_oct_digit0_various_inputs() {
    // oct_digit0 on valid octal digits
    let result: IResult<&str, &str> = oct_digit0("01234567abc");
    assert_eq!(result, Ok(("abc", "01234567")));

    // oct_digit0 on empty input
    let result: IResult<&str, &str> = oct_digit0("");
    assert_eq!(result, Ok(("", "")));

    // oct_digit0 stops at 8
    let result: IResult<&str, &str> = oct_digit0("12389");
    assert_eq!(result, Ok(("89", "123")));

    // oct_digit0 stops at 9
    let result: IResult<&str, &str> = oct_digit0("7779");
    assert_eq!(result, Ok(("9", "777")));

    // oct_digit0 on non-octal input
    let result: IResult<&str, &str> = oct_digit0("abc");
    assert_eq!(result, Ok(("abc", "")));

    // oct_digit0 all valid octal
    let result: IResult<&str, &str> = oct_digit0("76543210");
    assert_eq!(result, Ok(("", "76543210")));

    // oct_digit0 single zero
    let result: IResult<&str, &str> = oct_digit0("0");
    assert_eq!(result, Ok(("", "0")));

    // oct_digit0 with trailing content
    let result: IResult<&str, &str> = oct_digit0("755 permissions");
    assert_eq!(result, Ok((" permissions", "755")));
}

#[test]
fn test_bin_digit0_various_inputs() {
    // bin_digit0 on binary digits
    let result: IResult<&str, &str> = bin_digit0("101010xyz");
    assert_eq!(result, Ok(("xyz", "101010")));

    // bin_digit0 on empty input
    let result: IResult<&str, &str> = bin_digit0("");
    assert_eq!(result, Ok(("", "")));

    // bin_digit0 stops at 2
    let result: IResult<&str, &str> = bin_digit0("1102");
    assert_eq!(result, Ok(("2", "110")));

    // bin_digit0 on non-binary input
    let result: IResult<&str, &str> = bin_digit0("234");
    assert_eq!(result, Ok(("234", "")));

    // bin_digit0 all ones
    let result: IResult<&str, &str> = bin_digit0("11111111");
    assert_eq!(result, Ok(("", "11111111")));

    // bin_digit0 all zeros
    let result: IResult<&str, &str> = bin_digit0("00000000");
    assert_eq!(result, Ok(("", "00000000")));

    // bin_digit0 single bit
    let result: IResult<&str, &str> = bin_digit0("1abc");
    assert_eq!(result, Ok(("abc", "1")));

    // bin_digit0 stops at letter
    let result: IResult<&str, &str> = bin_digit0("0b101");
    // 'b' is not binary, so stops after '0'
    assert_eq!(result, Ok(("b101", "0")));
}

#[test]
fn test_alphanumeric0_various_inputs() {
    // alphanumeric0 on mixed input
    let result: IResult<&str, &str> = alphanumeric0("abc123!!");
    assert_eq!(result, Ok(("!!", "abc123")));

    // alphanumeric0 on empty input
    let result: IResult<&str, &str> = alphanumeric0("");
    assert_eq!(result, Ok(("", "")));

    // alphanumeric0 on pure alpha
    let result: IResult<&str, &str> = alphanumeric0("hello world");
    assert_eq!(result, Ok((" world", "hello")));

    // alphanumeric0 on pure digits
    let result: IResult<&str, &str> = alphanumeric0("12345 ");
    assert_eq!(result, Ok((" ", "12345")));

    // alphanumeric0 on non-alphanumeric start
    let result: IResult<&str, &str> = alphanumeric0("!@#abc");
    assert_eq!(result, Ok(("!@#abc", "")));

    // alphanumeric0 consumes everything alphanumeric
    let result: IResult<&str, &str> = alphanumeric0("Test123Value");
    assert_eq!(result, Ok(("", "Test123Value")));

    // alphanumeric0 stops at underscore
    let result: IResult<&str, &str> = alphanumeric0("abc_def");
    assert_eq!(result, Ok(("_def", "abc")));

    // alphanumeric0 stops at hyphen
    let result: IResult<&str, &str> = alphanumeric0("foo-bar");
    assert_eq!(result, Ok(("-bar", "foo")));
}

#[test]
fn test_combined_parsers_workflow() {
    // Parse a simple key=value where key is alphanumeric and value is digits
    fn key_value(input: &str) -> IResult<&str, (&str, &str)> {
        let (input, key) = alpha1(input)?;
        let (input, _) = nom_char('=')(input)?;
        let (input, value) = digit1(input)?;
        Ok((input, (key, value)))
    }

    let result = key_value("count=42 rest");
    assert_eq!(result, Ok((" rest", ("count", "42"))));

    // Parse tab-separated values
    fn tab_separated_pair(input: &str) -> IResult<&str, (&str, &str)> {
        let (input, first) = digit0(input)?;
        let (input, _) = tab(input)?;
        let (input, second) = digit0(input)?;
        Ok((input, (first, second)))
    }

    let result = tab_separated_pair("123\t456end");
    assert_eq!(result, Ok(("end", ("123", "456"))));

    // Parse hex color code using hex_digit0
    fn hex_color_code(input: &str) -> IResult<&str, &str> {
        let (input, _) = nom_char('#')(input)?;
        hex_digit0(input)
    }

    let result = hex_color_code("#FF00AA rest");
    assert_eq!(result, Ok((" rest", "FF00AA")));

    // Combine satisfy with digit0 for identifier parsing
    fn identifier(input: &str) -> IResult<&str, (char, &str)> {
        let (input, first) = satisfy(|c: char| c.is_alphabetic() || c == '_')(input)?;
        let (input, rest) = alphanumeric0(input)?;
        Ok((input, (first, rest)))
    }

    let result = identifier("_var123 = 5");
    assert_eq!(result, Ok((" = 5", ('_', "var123"))));

    let result = identifier("x");
    assert_eq!(result, Ok(("", ('x', ""))));

    // Verify identifier fails on digit start
    let result = identifier("123abc");
    assert!(result.is_err());

    // Parse octal permission string
    fn octal_perm(input: &str) -> IResult<&str, &str> {
        let (input, _) = nom_char('0')(input)?;
        oct_digit0(input)
    }

    let result = octal_perm("0755 file");
    assert_eq!(result, Ok((" file", "755")));

    // Parse binary literal
    fn binary_literal(input: &str) -> IResult<&str, &str> {
        let (input, _) = tag("0b")(input)?;
        bin_digit0(input)
    }

    let result = binary_literal("0b11010 rest");
    assert_eq!(result, Ok((" rest", "11010")));
}

#[test]
fn test_satisfy_in_sequence_combinators() {
    // Use satisfy to parse a sign character followed by digits
    fn signed_number(input: &str) -> IResult<&str, (Option<char>, &str)> {
        let sign_result: IResult<&str, char> = satisfy(|c: char| c == '+' || c == '-')(input);
        match sign_result {
            Ok((rest, sign)) => {
                let (rest, digits) = digit1(rest)?;
                Ok((rest, (Some(sign), digits)))
            }
            Err(_) => {
                let (rest, digits) = digit1(input)?;
                Ok((rest, (None, digits)))
            }
        }
    }

    let result = signed_number("-42 end");
    assert_eq!(result, Ok((" end", (Some('-'), "42"))));

    let result = signed_number("+100!");
    assert_eq!(result, Ok(("!", (Some('+'), "100"))));

    let result = signed_number("999x");
    assert_eq!(result, Ok(("x", (None, "999"))));

    // Use satisfy to match opening bracket types
    let mut open_bracket = satisfy(|c: char| c == '(' || c == '[' || c == '{');
    let result: IResult<&str, char> = open_bracket("(content)");
    assert_eq!(result, Ok(("content)", '(')));

    let result: IResult<&str, char> = open_bracket("[item]");
    assert_eq!(result, Ok(("item]", '[')));

    let result: IResult<&str, char> = open_bracket("{key}");
    assert_eq!(result, Ok(("key}", '{')));

    // Fails on non-bracket
    let result: IResult<&str, char> = open_bracket("abc");
    assert!(result.is_err());

    // Chain multiple satisfy calls
    fn two_uppercase(input: &str) -> IResult<&str, (char, char)> {
        let (input, c1) = satisfy(|c: char| c.is_uppercase())(input)?;
        let (input, c2) = satisfy(|c: char| c.is_uppercase())(input)?;
        Ok((input, (c1, c2)))
    }

    let result = two_uppercase("ABcde");
    assert_eq!(result, Ok(("cde", ('A', 'B'))));
}

#[test]
fn test_digit0_and_hex_digit0_boundary_conditions() {
    // digit0 with very long digit string
    let long_digits = "9".repeat(100);
    let input = format!("{}end", long_digits);
    let result: IResult<&str, &str> = digit0(&input);
    let (remaining, matched) = result.unwrap();
    assert_eq!(remaining, "end");
    assert_eq!(matched.len(), 100);

    // hex_digit0 with all hex chars
    let result: IResult<&str, &str> = hex_digit0("0123456789abcdefABCDEF!");
    assert_eq!(result, Ok(("!", "0123456789abcdefABCDEF")));

    // digit0 followed by digit1 pattern
    let result_d0: IResult<&str, &str> = digit0("abc");
    assert_eq!(result_d0, Ok(("abc", "")));
    let result_d1: IResult<&str, &str> = digit1("abc");
    assert!(result_d1.is_err());

    // hex_digit0 vs oct_digit0 on same input
    let result_hex: IResult<&str, &str> = hex_digit0("789abc");
    assert_eq!(result_hex, Ok(("", "789abc")));
    let result_oct: IResult<&str, &str> = oct_digit0("789abc");
    assert_eq!(result_oct, Ok(("89abc", "7")));

    // bin_digit0 vs digit0 on same input
    let result_bin: IResult<&str, &str> = bin_digit0("10234");
    assert_eq!(result_bin, Ok(("234", "10")));
    let result_dig: IResult<&str, &str> = digit0("10234");
    assert_eq!(result_dig, Ok(("", "10234")));

    // alphanumeric0 includes both alpha and digit
    let result: IResult<&str, &str> = alphanumeric0("a1b2c3!!!");
    assert_eq!(result, Ok(("!!!", "a1b2c3")));

    // All zero-or-more parsers succeed on empty
    let r1: IResult<&str, &str> = digit0("");
    let r2: IResult<&str, &str> = hex_digit0("");
    let r3: IResult<&str, &str> = oct_digit0("");
    let r4: IResult<&str, &str> = bin_digit0("");
    let r5: IResult<&str, &str> = alphanumeric0("");
    assert_eq!(r1, Ok(("", "")));
    assert_eq!(r2, Ok(("", "")));
    assert_eq!(r3, Ok(("", "")));
    assert_eq!(r4, Ok(("", "")));
    assert_eq!(r5, Ok(("", "")));
}

#[test]
fn test_realistic_config_parser() {
    // Parse a simple config line: key<tab>value where value can be hex, oct, or decimal
    fn config_line(input: &str) -> IResult<&str, (&str, &str, &str)> {
        let (input, key) = alphanumeric0(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = nom_char('=')(input)?;
        let (input, _) = space0(input)?;
        let (input, value) = alt((
            preceded(tag("0x"), hex_digit0),
            preceded(tag("0o"), oct_digit0),
            preceded(tag("0b"), bin_digit0),
        )).parse(input)?;
        // Determine which prefix was used by checking what's left
        Ok((input, (key, value, "")))
    }

    // Parse hex config
    let result = config_line("port = 0x1F90");
    let (remaining, (key, value, _)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(key, "port");
    assert_eq!(value, "1F90");

    // Parse octal config
    let result = config_line("mode = 0o755");
    let (remaining, (key, value, _)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(key, "mode");
    assert_eq!(value, "755");

    // Parse binary config
    let result = config_line("flags = 0b1010");
    let (remaining, (key, value, _)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(key, "flags");
    assert_eq!(value, "1010");

    // Verify satisfy works for custom token parsing
    fn parse_operator(input: &str) -> IResult<&str, char> {
        satisfy(|c: char| "+-*/%".contains(c))(input)
    }

    assert_eq!(parse_operator("+rest"), Ok(("rest", '+')));
    assert_eq!(parse_operator("-rest"), Ok(("rest", '-')));
    assert_eq!(parse_operator("%rest"), Ok(("rest", '%')));
    assert!(parse_operator("abc").is_err());
}