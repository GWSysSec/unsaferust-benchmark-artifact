extern crate rustc_demangle;

use rustc_demangle::{demangle, try_demangle};

#[test]
fn test_demangle_basic() {
    let demangled = demangle("_ZN4test5hello17h05af221e174051e9E");
    let output = format!("{}", demangled);
    assert!(output.contains("test"));
    assert!(output.contains("hello"));
}

#[test]
fn test_demangle_as_str() {
    let demangled = demangle("_ZN4test5hello17h05af221e174051e9E");
    let s = demangled.as_str();
    assert_eq!(s, "_ZN4test5hello17h05af221e174051e9E");
}

#[test]
fn test_try_demangle_valid() {
    let result = try_demangle("_ZN4test5hello17h05af221e174051e9E");
    assert!(result.is_ok());
    let demangled = result.unwrap();
    let output = format!("{}", demangled);
    assert!(output.contains("test"));
}

#[test]
fn test_try_demangle_invalid() {
    let result = try_demangle("not_a_mangled_symbol");
    assert!(result.is_err());
}

#[test]
fn test_try_demangle_error_clone() {
    let result = try_demangle("not_a_mangled_symbol");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err2 = err.clone();
    let s1 = format!("{:?}", err);
    let s2 = format!("{:?}", err2);
    assert_eq!(s1, s2);
}

#[test]
fn test_demangle_display_vs_debug() {
    let demangled = demangle("_ZN4test5hello17h05af221e174051e9E");
    let display_output = format!("{}", demangled);
    let debug_output = format!("{:?}", demangled);
    assert!(!display_output.is_empty());
    assert!(!debug_output.is_empty());
}

#[test]
fn test_demangle_many_symbols() {
    let symbols = vec![
        "_ZN4test5hello17h05af221e174051e9E",
        "_ZN3std2io5stdio6_print17h1ded803b1aed6cccE",
        "_ZN4core3fmt9Arguments6new_v117h4c4c12502a2d0295E",
        "not_mangled",
        "_ZN71_$LT$std..path..PathBuf$u20$as$u20$core..convert..From$LT$$RF$str$GT$$GT$4from17h65e04b1a4f22af45E",
    ];

    for sym in &symbols {
        let demangled = demangle(sym);
        let output = format!("{}", demangled);
        assert!(!output.is_empty());
    }
}