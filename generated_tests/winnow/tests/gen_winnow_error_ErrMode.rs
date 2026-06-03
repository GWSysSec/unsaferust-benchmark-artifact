use winnow::Parser;
use winnow::combinator::{cut_err, fail};
use winnow::error::{ErrMode, InputError};

#[test]
fn test_errmode_backtrack_converts_cut_to_backtrack() {
    let mut input: &str = "abcdef";
    let res: Result<(), ErrMode<InputError<&str>>> =
        cut_err(fail).parse_next(&mut input);
    let err = res.expect_err("cut_err(fail) must fail");


    assert!(matches!(err, ErrMode::Cut(_)), "expected Cut variant");
    assert!(!matches!(err, ErrMode::Backtrack(_)));
    assert!(!matches!(err, ErrMode::Incomplete(_)));


    let demoted = err.backtrack();
    assert!(matches!(demoted, ErrMode::Backtrack(_)), "should be Backtrack after backtrack()");
    assert!(!matches!(demoted, ErrMode::Cut(_)));
    assert!(!matches!(demoted, ErrMode::Incomplete(_)));


    let still_backtrack = demoted.backtrack();
    assert!(matches!(still_backtrack, ErrMode::Backtrack(_)));
}

#[test]
fn test_errmode_backtrack_idempotent_on_backtrack() {
    let mut input: &str = "xyz";

    let res: Result<(), ErrMode<InputError<&str>>> = fail.parse_next(&mut input);
    let err = res.expect_err("fail must fail");

    assert!(matches!(err, ErrMode::Backtrack(_)));
    let again = err.backtrack();
    assert!(matches!(again, ErrMode::Backtrack(_)));
    assert!(!matches!(again, ErrMode::Cut(_)));


    assert_eq!(input, "xyz");
    assert_eq!(input.len(), 3);
}

#[test]
fn test_errmode_map_input_changes_input_type() {
    let mut input: &str = "hello world";
    let res: Result<(), ErrMode<InputError<&str>>> =
        cut_err(fail).parse_next(&mut input);
    let err = res.expect_err("should fail");

    assert!(matches!(err, ErrMode::Cut(_)));


    let mapped: ErrMode<InputError<usize>> = err.map_input(|s: &str| s.len());

    assert!(matches!(mapped, ErrMode::Cut(_)));
    assert!(!matches!(mapped, ErrMode::Backtrack(_)));


    let demoted = mapped.backtrack();
    assert!(matches!(demoted, ErrMode::Backtrack(_)));


    let remapped: ErrMode<InputError<usize>> = demoted.map_input(|n: usize| n * 2);
    assert!(matches!(remapped, ErrMode::Backtrack(_)));
}

#[test]
fn test_errmode_convert_identity() {
    let mut input: &str = "input-data";
    let res: Result<(), ErrMode<InputError<&str>>> =
        cut_err(fail).parse_next(&mut input);
    let err = res.expect_err("should fail");

    assert!(matches!(err, ErrMode::Cut(_)));


    let converted: ErrMode<InputError<(&str, usize)>> = err.convert();
    assert!(matches!(converted, ErrMode::Cut(_)));
    assert!(!matches!(converted, ErrMode::Backtrack(_)));
    assert!(!matches!(converted, ErrMode::Incomplete(_)));


    let chained = converted.convert::<InputError<&str>>().backtrack();
    assert!(matches!(chained, ErrMode::Backtrack(_)));
}

#[test]
fn test_errmode_combined_workflow() {

    let mut input: &str = "abc123";
    let res: Result<(), ErrMode<InputError<&str>>> =
        cut_err(fail).parse_next(&mut input);
    let err = res.expect_err("must error");


    assert!(matches!(err, ErrMode::Cut(_)));


    let step2_intermediate: ErrMode<InputError<(&str, usize)>> = err.convert();
    assert!(matches!(step2_intermediate, ErrMode::Cut(_)));
    let step2: ErrMode<InputError<&str>> = step2_intermediate.convert();
    assert!(matches!(step2, ErrMode::Cut(_)));


    let step3: ErrMode<InputError<usize>> = step2.map_input(|s: &str| s.chars().count());
    assert!(matches!(step3, ErrMode::Cut(_)));


    let step4 = step3.backtrack();
    assert!(matches!(step4, ErrMode::Backtrack(_)));
    assert!(!matches!(step4, ErrMode::Cut(_)));


    let step5: ErrMode<InputError<u64>> = step4.map_input(|n: usize| n as u64 + 1);
    assert!(matches!(step5, ErrMode::Backtrack(_)));


    let step6 = step5.backtrack();
    assert!(matches!(step6, ErrMode::Backtrack(_)));
}