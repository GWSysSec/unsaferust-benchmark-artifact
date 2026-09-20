#![cfg(getrandom_backend = "rndr")]

use getrandom::backends::rndr;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_slice_and_preserves_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = rndr::fill_inner(&mut empty);

    assert!(result.is_ok(), "rndr fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(
        empty.as_mut_ptr(),
        original_ptr,
        "fill_inner must not replace or reallocate the caller's empty buffer"
    );
}

#[test]
fn fill_inner_initializes_single_and_larger_buffers_without_reallocation() {
    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = rndr::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "rndr fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(
        single.as_mut_ptr(),
        single_ptr,
        "single-byte buffer address should be unchanged"
    );
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut larger = [MaybeUninit::<u8>::uninit(); 257];
    let larger_ptr = larger.as_mut_ptr();

    let larger_result = rndr::fill_inner(&mut larger);

    assert!(
        larger_result.is_ok(),
        "rndr fill_inner should initialize a larger, non-word-aligned buffer"
    );
    assert_eq!(
        larger.as_mut_ptr(),
        larger_ptr,
        "larger buffer address should be unchanged"
    );

    let bytes = initialized_bytes(&larger);
    assert_eq!(bytes.len(), 257);
    assert_eq!(
        bytes.as_ptr(),
        larger_ptr.cast::<u8>(),
        "initialized byte view should point at the same storage"
    );
}

#[test]
fn fill_inner_can_be_called_repeatedly_on_adjacent_regions() {
    let mut storage = [MaybeUninit::<u8>::uninit(); 96];
    let storage_ptr = storage.as_mut_ptr();

    let (first, rest) = storage.split_at_mut(32);
    let (second, third) = rest.split_at_mut(32);

    let first_result = rndr::fill_inner(first);
    let second_result = rndr::fill_inner(second);
    let third_result = rndr::fill_inner(third);

    assert!(first_result.is_ok(), "first rndr fill should succeed");
    assert!(second_result.is_ok(), "second rndr fill should succeed");
    assert!(third_result.is_ok(), "third rndr fill should succeed");
    assert_eq!(
        storage.as_mut_ptr(),
        storage_ptr,
        "splitting and filling should not move the backing array"
    );

    let initialized = initialized_bytes(&storage);
    assert_eq!(initialized.len(), 96);
    assert_eq!(&initialized[..32].len(), &32);
    assert_eq!(&initialized[32..64].len(), &32);
    assert_eq!(&initialized[64..].len(), &32);
}