#![cfg(target_os = "fuchsia")]

use getrandom::backends::fuchsia;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_handles_empty_single_and_larger_buffers() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = fuchsia::fill_inner(&mut empty);

    assert!(empty_result.is_ok(), "empty Fuchsia RNG fill should succeed");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut one_byte = [MaybeUninit::<u8>::uninit(); 1];
    let one_byte_ptr = one_byte.as_mut_ptr();

    let one_byte_result = fuchsia::fill_inner(&mut one_byte);

    assert!(
        one_byte_result.is_ok(),
        "single-byte Fuchsia RNG fill should succeed"
    );
    assert_eq!(initialized_bytes(&one_byte).len(), 1);
    assert_eq!(one_byte.as_mut_ptr(), one_byte_ptr);

    let mut larger = [MaybeUninit::<u8>::uninit(); 256];
    let larger_ptr = larger.as_mut_ptr();

    let larger_result = fuchsia::fill_inner(&mut larger);

    assert!(
        larger_result.is_ok(),
        "larger Fuchsia RNG fill should succeed"
    );
    assert_eq!(initialized_bytes(&larger).len(), 256);
    assert_eq!(larger.as_mut_ptr(), larger_ptr);
}

#[test]
fn fill_inner_supports_chunked_workflows_without_reallocating() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 128];
    let original_ptr = buffer.as_mut_ptr();
    let original_len = buffer.len();

    let (first, rest) = buffer.split_at_mut(17);
    let (middle, last) = rest.split_at_mut(73);

    let first_ptr = first.as_mut_ptr();
    let middle_ptr = middle.as_mut_ptr();
    let last_ptr = last.as_mut_ptr();

    assert!(fuchsia::fill_inner(first).is_ok(), "first chunk should fill");
    assert!(fuchsia::fill_inner(middle).is_ok(), "middle chunk should fill");
    assert!(fuchsia::fill_inner(last).is_ok(), "last chunk should fill");

    assert_eq!(first.len(), 17);
    assert_eq!(middle.len(), 73);
    assert_eq!(last.len(), 38);
    assert_eq!(first.as_mut_ptr(), first_ptr);
    assert_eq!(middle.as_mut_ptr(), middle_ptr);
    assert_eq!(last.as_mut_ptr(), last_ptr);

    assert_eq!(buffer.len(), original_len);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
    assert_eq!(initialized_bytes(&buffer).len(), original_len);
}

#[test]
fn repeated_fill_inner_calls_can_reuse_the_same_buffer() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 64];
    let ptr_before_first_fill = buffer.as_mut_ptr();

    let first_result = fuchsia::fill_inner(&mut buffer);
    assert!(first_result.is_ok(), "first fill should succeed");

    let bytes_after_first_fill = initialized_bytes(&buffer);
    assert_eq!(bytes_after_first_fill.len(), 64);
    assert_eq!(buffer.as_mut_ptr(), ptr_before_first_fill);

    let ptr_before_second_fill = buffer.as_mut_ptr();

    let second_result = fuchsia::fill_inner(&mut buffer);
    assert!(second_result.is_ok(), "second fill should succeed");

    let bytes_after_second_fill = initialized_bytes(&buffer);
    assert_eq!(bytes_after_second_fill.len(), 64);
    assert_eq!(buffer.as_mut_ptr(), ptr_before_second_fill);
}