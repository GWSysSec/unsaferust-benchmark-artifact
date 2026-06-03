#![cfg(all(windows, getrandom_backend = "windows7"))]

use getrandom::backends::windows7;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_buffer_and_preserves_slice_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = windows7::fill_inner(&mut empty);

    assert!(result.is_ok(), "windows7 fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_single_and_larger_buffers_without_reallocating() {
    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = windows7::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "windows7 fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut larger = [MaybeUninit::<u8>::uninit(); 512];
    let larger_ptr = larger.as_mut_ptr();

    let larger_result = windows7::fill_inner(&mut larger);

    assert!(
        larger_result.is_ok(),
        "windows7 fill_inner should initialize a larger buffer"
    );
    assert_eq!(larger.as_mut_ptr(), larger_ptr);
    assert_eq!(initialized_bytes(&larger).len(), 512);

    let non_zero_count = initialized_bytes(&larger)
        .iter()
        .filter(|&&byte| byte != 0)
        .count();
    assert!(
        non_zero_count > 0,
        "a 512-byte random buffer should not be entirely zero"
    );
}

#[test]
fn fill_inner_can_be_called_repeatedly_on_distinct_buffers() {
    let mut first = [MaybeUninit::<u8>::uninit(); 64];
    let mut second = [MaybeUninit::<u8>::uninit(); 64];

    let first_result = windows7::fill_inner(&mut first);
    let second_result = windows7::fill_inner(&mut second);

    assert!(first_result.is_ok(), "first windows7 fill should succeed");
    assert!(second_result.is_ok(), "second windows7 fill should succeed");

    let first_bytes = initialized_bytes(&first);
    let second_bytes = initialized_bytes(&second);

    assert_eq!(first_bytes.len(), 64);
    assert_eq!(second_bytes.len(), 64);
    assert_ne!(
        first_bytes, second_bytes,
        "two independent 64-byte random fills should not produce identical output"
    );
}

#[test]
fn fill_inner_fills_only_the_requested_subslice() {
    let mut buffer = [MaybeUninit::<u8>::new(0xA5); 96];

    let (prefix, rest) = buffer.split_at_mut(16);
    let (middle, suffix) = rest.split_at_mut(64);

    let result = windows7::fill_inner(middle);

    assert!(
        result.is_ok(),
        "windows7 fill_inner should initialize a middle subslice"
    );
    assert!(
        initialized_bytes(prefix).iter().all(|&byte| byte == 0xA5),
        "bytes before the requested subslice must be left unchanged"
    );
    assert!(
        initialized_bytes(suffix).iter().all(|&byte| byte == 0xA5),
        "bytes after the requested subslice must be left unchanged"
    );
    assert_eq!(initialized_bytes(middle).len(), 64);

    let changed_middle_bytes = initialized_bytes(middle)
        .iter()
        .filter(|&&byte| byte != 0xA5)
        .count();
    assert!(
        changed_middle_bytes > 0,
        "the filled subslice should contain bytes written by the RNG"
    );
}