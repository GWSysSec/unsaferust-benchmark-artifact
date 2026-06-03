use combine::{
    any, chainl1, chainr1, look_ahead, one_of, satisfy, sep_end_by, Parser,
};
use combine::parser::char::{char as char_p, digit};
use combine::parser::repeat::{escaped, iterate, repeat_until};

fn sub(a: i32, b: i32) -> i32 { a - b }
fn add(a: i32, b: i32) -> i32 { a + b }
fn pow(a: i32, b: i32) -> i32 { a.pow(b as u32) }