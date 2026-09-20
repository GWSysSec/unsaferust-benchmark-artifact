use std::env;
use std::ffi::OsStr;
use std::path::Path;

mod support;
use crate::support::Test;

#[test]
fn test_build_basic() {
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("libfoo.a");
}