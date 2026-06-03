#![cfg(getrandom_backend = "getentropy")]

use getrandom::backends::getentropy;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_handles_empty_single_and_multi_byte_buffers() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = getentropy::fill_inner(&mut empty);

    assert!(empty_result.is_ok(), "empty getentropy fill should succeed");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = getentropy::fill_inner(&mut single);

    assert!(single_result.is_ok(), "single-byte getentropy fill should succeed");
    assert_eq!(initialized_bytes(&single).len(), 1);
    assert_eq!(single.as_mut_ptr(), single_ptr);

    let mut buffer = [MaybeUninit::<u8>::uninit(); 64];
    let buffer_ptr = buffer.as_mut_ptr();

    let buffer_result = getentropy::fill_inner(&mut buffer);

    assert!(buffer_result.is_ok(), "64-byte getentropy fill should succeed");
    assert_eq!(initialized_bytes(&buffer).len(), 64);
    assert_eq!(buffer.as_mut_ptr(), buffer_ptr);
}

#[test]
fn fill_inner_supports_chunked_filling_of_one_allocation() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 128];
    let original_ptr = buffer.as_mut_ptr();

    let (prefix, rest) = buffer.split_at_mut(17);
    let (middle, suffix) = rest.split_at_mut(73);

    let prefix_result = getentropy::fill_inner(prefix);
    let middle_result = getentropy::fill_inner(middle);
    let suffix_result = getentropy::fill_inner(suffix);

    assert!(prefix_result.is_ok(), "prefix fill should succeed");
    assert!(middle_result.is_ok(), "middle fill should succeed");
    assert!(suffix_result.is_ok(), "suffix fill should succeed");
    assert_eq!(prefix.len(), 17);
    assert_eq!(middle.len(), 73);
    assert_eq!(suffix.len(), 38);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
    assert_eq!(initialized_bytes(&buffer).len(), 128);
}

#[test]
fn fill_inner_can_fill_buffers_larger_than_a_single_getentropy_request() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 513];
    let original_ptr = buffer.as_mut_ptr();

    let result = getentropy::fill_inner(&mut buffer);

    assert!(
        result.is_ok(),
        "getentropy backend should fill larger buffers by handling any required chunking"
    );
    assert_eq!(buffer.len(), 513);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);

    let initialized = initialized_bytes(&buffer);
    assert_eq!(initialized.len(), 513);
    assert_eq!(&initialized[..0], &[]);
    assert_eq!(initialized.as_ptr(), original_ptr.cast::<u8>());
}

#[test]
fn fill_inner_works_repeatedly_with_reused_storage() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 32];
    let original_ptr = buffer.as_mut_ptr();

    let first_result = getentropy::fill_inner(&mut buffer);
    assert!(first_result.is_ok(), "first fill should succeed");

    let first = initialized_bytes(&buffer);
    assert_eq!(first.len(), 32);

    let second_result = getentropy::fill_inner(&mut buffer);
    assert!(second_result.is_ok(), "second fill should succeed using the same storage");

    let second = initialized_bytes(&buffer);
    assert_eq!(second.len(), 32);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
}