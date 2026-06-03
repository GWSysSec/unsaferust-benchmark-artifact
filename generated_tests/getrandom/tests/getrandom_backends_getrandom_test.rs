#![cfg(all(
    any(target_os = "linux", target_os = "android"),
    getrandom_backend = "linux_getrandom"
))]

use getrandom::backends::getrandom;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_handles_empty_single_and_larger_buffers() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = getrandom::fill_inner(&mut empty);

    assert!(empty_result.is_ok(), "empty getrandom fill should succeed");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut one_byte = [MaybeUninit::<u8>::uninit(); 1];
    let one_byte_ptr = one_byte.as_mut_ptr();

    let one_byte_result = getrandom::fill_inner(&mut one_byte);

    assert!(
        one_byte_result.is_ok(),
        "single-byte getrandom fill should succeed"
    );
    assert_eq!(initialized_bytes(&one_byte).len(), 1);
    assert_eq!(one_byte.as_mut_ptr(), one_byte_ptr);

    let mut larger = [MaybeUninit::<u8>::uninit(); 512];
    let larger_ptr = larger.as_mut_ptr();

    let larger_result = getrandom::fill_inner(&mut larger);

    assert!(
        larger_result.is_ok(),
        "larger getrandom fill should succeed"
    );
    assert_eq!(initialized_bytes(&larger).len(), 512);
    assert_eq!(larger.as_mut_ptr(), larger_ptr);
}

#[test]
fn fill_inner_supports_chunked_workflows_without_reallocating() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 257];
    let original_ptr = buffer.as_mut_ptr();

    let (prefix, remainder) = buffer.split_at_mut(17);
    let (middle, suffix) = remainder.split_at_mut(128);

    let prefix_result = getrandom::fill_inner(prefix);
    let middle_result = getrandom::fill_inner(middle);
    let suffix_result = getrandom::fill_inner(suffix);

    assert!(prefix_result.is_ok(), "prefix fill should succeed");
    assert!(middle_result.is_ok(), "middle fill should succeed");
    assert!(suffix_result.is_ok(), "suffix fill should succeed");
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
    assert_eq!(initialized_bytes(&buffer).len(), 257);

    let prefix_bytes = initialized_bytes(&buffer[..17]);
    let middle_bytes = initialized_bytes(&buffer[17..145]);
    let suffix_bytes = initialized_bytes(&buffer[145..]);

    assert_eq!(prefix_bytes.len(), 17);
    assert_eq!(middle_bytes.len(), 128);
    assert_eq!(suffix_bytes.len(), 112);
}

#[test]
fn fill_inner_can_refill_the_same_buffer_and_keep_slice_shape() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 64];
    let original_ptr = buffer.as_mut_ptr();
    let original_len = buffer.len();

    let first_result = getrandom::fill_inner(&mut buffer);
    assert!(first_result.is_ok(), "initial fill should succeed");

    let first_snapshot = initialized_bytes(&buffer).to_vec();
    assert_eq!(first_snapshot.len(), original_len);

    let second_result = getrandom::fill_inner(&mut buffer);
    assert!(second_result.is_ok(), "refill should succeed");

    let second_snapshot = initialized_bytes(&buffer).to_vec();
    assert_eq!(second_snapshot.len(), original_len);
    assert_eq!(buffer.len(), original_len);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
}