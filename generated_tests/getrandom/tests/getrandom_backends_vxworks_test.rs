#![cfg(target_os = "vxworks")]

use getrandom::backends::vxworks;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_buffer_and_preserves_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = vxworks::fill_inner(&mut empty);

    assert!(result.is_ok(), "vxworks fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_buffers_of_multiple_sizes_without_reallocating() {
    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = vxworks::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "vxworks fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut medium = [MaybeUninit::<u8>::uninit(); 64];
    let medium_ptr = medium.as_mut_ptr();

    let medium_result = vxworks::fill_inner(&mut medium);

    assert!(
        medium_result.is_ok(),
        "vxworks fill_inner should initialize a medium-sized buffer"
    );
    assert_eq!(medium.as_mut_ptr(), medium_ptr);
    assert_eq!(initialized_bytes(&medium).len(), 64);

    let mut large = [MaybeUninit::<u8>::uninit(); 1024];
    let large_ptr = large.as_mut_ptr();

    let large_result = vxworks::fill_inner(&mut large);

    assert!(
        large_result.is_ok(),
        "vxworks fill_inner should initialize a large buffer"
    );
    assert_eq!(large.as_mut_ptr(), large_ptr);
    assert_eq!(initialized_bytes(&large).len(), 1024);
}

#[test]
fn repeated_fill_inner_calls_can_be_chained_on_independent_buffers() {
    let mut first = [MaybeUninit::<u8>::uninit(); 32];
    let mut second = [MaybeUninit::<u8>::uninit(); 32];

    let first_result = vxworks::fill_inner(&mut first);
    let second_result = vxworks::fill_inner(&mut second);

    assert!(first_result.is_ok(), "first vxworks fill should succeed");
    assert!(second_result.is_ok(), "second vxworks fill should succeed");

    let first_bytes = initialized_bytes(&first);
    let second_bytes = initialized_bytes(&second);

    assert_eq!(first_bytes.len(), 32);
    assert_eq!(second_bytes.len(), 32);
    assert_ne!(first.as_ptr(), second.as_ptr());

    let first_has_nonzero = first_bytes.iter().any(|&byte| byte != 0);
    let second_has_nonzero = second_bytes.iter().any(|&byte| byte != 0);

    assert!(
        first_has_nonzero || second_has_nonzero || first_bytes != second_bytes,
        "two successful random fills should provide usable initialized byte slices"
    );
}