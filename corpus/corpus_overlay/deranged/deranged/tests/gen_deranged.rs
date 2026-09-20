use deranged::*;
use core::str::FromStr;

#[test]
fn test_parse_int_error_range_and_format() {
    type R = RangedI32<-100, 100>;

    let ok1: Result<R, ParseIntError> = R::from_str("42");
    let ok2: Result<R, ParseIntError> = R::from_str("-100");
    let ok3: Result<R, ParseIntError> = R::from_str("100");
    let ok4: Result<R, ParseIntError> = R::from_str("0");

    assert!(ok1.is_ok());
    assert!(ok2.is_ok());
    assert!(ok3.is_ok());
    assert!(ok4.is_ok());
    assert_eq!(ok1.unwrap().get(), 42);
    assert_eq!(ok2.unwrap().get(), -100);
    assert_eq!(ok3.unwrap().get(), 100);
    assert_eq!(ok4.unwrap().get(), 0);

    let err_high: Result<R, ParseIntError> = R::from_str("101");
    let err_low: Result<R, ParseIntError> = R::from_str("-101");
    let err_garbage: Result<R, ParseIntError> = R::from_str("xyz");
    let err_empty: Result<R, ParseIntError> = R::from_str("");
    let err_float: Result<R, ParseIntError> = R::from_str("3.14");

    assert!(err_high.is_err());
    assert!(err_low.is_err());
    assert!(err_garbage.is_err());
    assert!(err_empty.is_err());
    assert!(err_float.is_err());

    let e = err_high.unwrap_err();
    let display = format!("{}", e);
    let debug = format!("{:?}", e);
    assert!(!display.is_empty());
    assert!(!debug.is_empty());
    assert_ne!(display.len(), 0);
}

#[test]
fn test_try_from_int_error_workflow() {
    type Small = RangedI16<-50, 50>;
    type Tiny = RangedU8<0, 10>;

    let good1: Result<Small, TryFromIntError> = Small::try_from(25i16);
    let good2: Result<Small, TryFromIntError> = Small::try_from(-50i16);
    let good3: Result<Small, TryFromIntError> = Small::try_from(50i16);

    assert!(good1.is_ok());
    assert!(good2.is_ok());
    assert!(good3.is_ok());
    assert_eq!(good1.unwrap().get(), 25);
    assert_eq!(good2.unwrap().get(), -50);
    assert_eq!(good3.unwrap().get(), 50);

    let bad_high: Result<Small, TryFromIntError> = Small::try_from(51i16);
    let bad_low: Result<Small, TryFromIntError> = Small::try_from(-51i16);
    let bad_far: Result<Small, TryFromIntError> = Small::try_from(i16::MAX);

    assert!(bad_high.is_err());
    assert!(bad_low.is_err());
    assert!(bad_far.is_err());

    let err = bad_high.unwrap_err();
    let display = format!("{}", err);
    let debug = format!("{:?}", err);
    assert!(!display.is_empty());
    assert!(!debug.is_empty());

    let tiny_ok: Result<Tiny, TryFromIntError> = Tiny::try_from(5u8);
    let tiny_bad: Result<Tiny, TryFromIntError> = Tiny::try_from(11u8);
    assert!(tiny_ok.is_ok());
    assert!(tiny_bad.is_err());
    assert_eq!(tiny_ok.unwrap().get(), 5);
}

#[test]
fn test_combined_error_paths_multi_step() {
    type Pct = RangedU16<0, 1000>;

    let parsed: Result<Pct, ParseIntError> = Pct::from_str("500");
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap().get(), 500);

    let raw_values: [u16; 5] = [0, 1, 999, 1000, 1001];
    let mut ok_count = 0;
    let mut err_count = 0;
    for &r in &raw_values {
        let res: Result<Pct, TryFromIntError> = Pct::try_from(r);
        if res.is_ok() {
            ok_count += 1;
        } else {
            let _ = format!("{}", res.unwrap_err());
            err_count += 1;
        }
    }
    assert_eq!(ok_count, 4);
    assert_eq!(err_count, 1);

    let strs = ["0", "500", "1000", "1001", "abc", "-1"];
    let mut parse_ok = 0;
    let mut parse_err = 0;
    for s in &strs {
        let res: Result<Pct, ParseIntError> = Pct::from_str(s);
        match res {
            Ok(v) => {
                assert!(v.get() <= 1000);
                parse_ok += 1;
            }
            Err(e) => {
                let _ = format!("{:?}", e);
                parse_err += 1;
            }
        }
    }
    assert_eq!(parse_ok, 3);
    assert_eq!(parse_err, 3);

    let tfe: TryFromIntError = Pct::try_from(2000u16).unwrap_err();
    let pie: ParseIntError = Pct::from_str("oops").unwrap_err();
    assert!(!format!("{:?}", tfe).is_empty());
    assert!(!format!("{:?}", pie).is_empty());
    assert!(!format!("{}", tfe).is_empty());
    assert!(!format!("{}", pie).is_empty());
}