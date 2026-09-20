#![cfg(all(target_os = "wasi", target_env = "p2"))]

use getrandom::backends::wasi_p2;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_buffer_and_preserves_slice_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = wasi_p2::fill_inner(&mut empty);

    assert!(result.is_ok(), "wasi_p2 fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_multiple_buffer_sizes_without_reallocation() {
    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = wasi_p2::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "wasi_p2 fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut medium = [MaybeUninit::<u8>::uninit(); 64];
    let medium_ptr = medium.as_mut_ptr();

    let medium_result = wasi_p2::fill_inner(&mut medium);

    assert!(
        medium_result.is_ok(),
        "wasi_p2 fill_inner should initialize a medium buffer"
    );
    assert_eq!(medium.as_mut_ptr(), medium_ptr);
    assert_eq!(initialized_bytes(&medium).len(), 64);

    let mut large = [MaybeUninit::<u8>::uninit(); 1024];
    let large_ptr = large.as_mut_ptr();

    let large_result = wasi_p2::fill_inner(&mut large);

    assert!(
        large_result.is_ok(),
        "wasi_p2 fill_inner should initialize a larger buffer"
    );
    assert_eq!(large.as_mut_ptr(), large_ptr);
    assert_eq!(initialized_bytes(&large).len(), 1024);
}

#[test]
fn fill_inner_can_be_called_repeatedly_on_distinct_and_reused_buffers() {
    let mut first = [MaybeUninit::<u8>::uninit(); 32];
    let mut second = [MaybeUninit::<u8>::uninit(); 32];

    let first_ptr = first.as_mut_ptr();
    let second_ptr = second.as_mut_ptr();

    let first_result = wasi_p2::fill_inner(&mut first);
    let second_result = wasi_p2::fill_inner(&mut second);

    assert!(first_result.is_ok(), "first fill should succeed");
    assert!(second_result.is_ok(), "second fill should succeed");
    assert_eq!(first.as_mut_ptr(), first_ptr);
    assert_eq!(second.as_mut_ptr(), second_ptr);
    assert_eq!(initialized_bytes(&first).len(), initialized_bytes(&second).len());

    let before_reuse_ptr = first.as_mut_ptr();
    let reuse_result = wasi_p2::fill_inner(&mut first[..]);

    assert!(reuse_result.is_ok(), "reusing an already initialized buffer should succeed");
    assert_eq!(first.as_mut_ptr(), before_reuse_ptr);
    assert_eq!(initialized_bytes(&first).len(), 32);
}