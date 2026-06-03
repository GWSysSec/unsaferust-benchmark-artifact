#![cfg(all(windows, getrandom_backend = "windows"))]

use getrandom::backends::windows;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_buffer_without_changing_slice_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = windows::fill_inner(&mut empty);

    assert!(result.is_ok(), "windows fill_inner should accept empty slices");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_small_buffers_and_preserves_allocation_identity() {
    let mut one = [MaybeUninit::<u8>::uninit(); 1];
    let one_ptr = one.as_mut_ptr();

    let one_result = windows::fill_inner(&mut one);

    assert!(
        one_result.is_ok(),
        "windows fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(one.as_mut_ptr(), one_ptr);
    assert_eq!(initialized_bytes(&one).len(), 1);

    let mut sixteen = [MaybeUninit::<u8>::uninit(); 16];
    let sixteen_ptr = sixteen.as_mut_ptr();

    let sixteen_result = windows::fill_inner(&mut sixteen);

    assert!(
        sixteen_result.is_ok(),
        "windows fill_inner should initialize a sixteen-byte buffer"
    );
    assert_eq!(sixteen.as_mut_ptr(), sixteen_ptr);

    let bytes = initialized_bytes(&sixteen);
    assert_eq!(bytes.len(), 16);

    let first_word = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let reconstructed = first_word.to_ne_bytes();
    assert_eq!(reconstructed, [bytes[0], bytes[1], bytes[2], bytes[3]]);
}

#[test]
fn fill_inner_supports_repeated_large_fills_over_distinct_regions() {
    let mut first = [MaybeUninit::<u8>::uninit(); 1024];
    let mut second = [MaybeUninit::<u8>::uninit(); 1024];

    let first_ptr = first.as_mut_ptr();
    let second_ptr = second.as_mut_ptr();

    let first_result = windows::fill_inner(&mut first);
    let second_result = windows::fill_inner(&mut second);

    assert!(first_result.is_ok(), "first large windows fill should succeed");
    assert!(second_result.is_ok(), "second large windows fill should succeed");
    assert_eq!(first.as_mut_ptr(), first_ptr);
    assert_eq!(second.as_mut_ptr(), second_ptr);

    let first_bytes = initialized_bytes(&first);
    let second_bytes = initialized_bytes(&second);

    assert_eq!(first_bytes.len(), 1024);
    assert_eq!(second_bytes.len(), 1024);
    assert_ne!(first_bytes.as_ptr(), second_bytes.as_ptr());

    let first_prefix = &first_bytes[..32];
    let second_prefix = &second_bytes[..32];

    assert_eq!(first_prefix.len(), 32);
    assert_eq!(second_prefix.len(), 32);
}