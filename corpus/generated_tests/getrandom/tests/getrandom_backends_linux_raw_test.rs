#![cfg(all(
    any(target_os = "linux", target_os = "android"),
    getrandom_backend = "linux_raw"
))]

use getrandom::backends::linux_raw;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_handles_empty_single_and_large_buffers_without_reallocating() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = linux_raw::fill_inner(&mut empty);

    assert!(empty_result.is_ok(), "empty linux_raw fill should succeed");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = linux_raw::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "single-byte linux_raw fill should succeed"
    );
    assert_eq!(initialized_bytes(&single).len(), 1);
    assert_eq!(single.as_mut_ptr(), single_ptr);

    let mut large = [MaybeUninit::<u8>::uninit(); 1024];
    let large_ptr = large.as_mut_ptr();

    let large_result = linux_raw::fill_inner(&mut large);

    assert!(
        large_result.is_ok(),
        "large linux_raw fill should succeed"
    );
    let large_bytes = initialized_bytes(&large);
    assert_eq!(large_bytes.len(), 1024);
    assert_eq!(large.as_mut_ptr(), large_ptr);
}

#[test]
fn fill_inner_can_be_used_for_incremental_buffer_construction() {
    let mut complete = [MaybeUninit::<u8>::uninit(); 96];
    let complete_ptr = complete.as_mut_ptr();

    {
        let (prefix, rest) = complete.split_at_mut(32);
        let (middle, suffix) = rest.split_at_mut(32);

        let prefix_result = linux_raw::fill_inner(prefix);
        let middle_result = linux_raw::fill_inner(middle);
        let suffix_result = linux_raw::fill_inner(suffix);

        assert!(prefix_result.is_ok(), "prefix fill should succeed");
        assert!(middle_result.is_ok(), "middle fill should succeed");
        assert!(suffix_result.is_ok(), "suffix fill should succeed");
    }

    assert_eq!(complete.as_mut_ptr(), complete_ptr);

    let bytes = initialized_bytes(&complete);
    assert_eq!(bytes.len(), 96);

    let prefix = &bytes[..32];
    let middle = &bytes[32..64];
    let suffix = &bytes[64..];

    assert_eq!(prefix.len(), 32);
    assert_eq!(middle.len(), 32);
    assert_eq!(suffix.len(), 32);

    assert!(
        prefix != middle || middle != suffix,
        "independently filled chunks should not all be identical"
    );
}

#[test]
fn repeated_fill_inner_calls_overwrite_existing_initialized_storage() {
    let mut first = [MaybeUninit::<u8>::uninit(); 64];
    let mut second = [MaybeUninit::<u8>::uninit(); 64];

    let first_result = linux_raw::fill_inner(&mut first);
    assert!(first_result.is_ok(), "first fill should succeed");

    let first_snapshot = initialized_bytes(&first).to_vec();
    assert_eq!(first_snapshot.len(), 64);

    let second_result = linux_raw::fill_inner(&mut second);
    assert!(second_result.is_ok(), "second fill should succeed");

    let second_snapshot = initialized_bytes(&second).to_vec();
    assert_eq!(second_snapshot.len(), 64);

    let refill_result = linux_raw::fill_inner(&mut first);
    assert!(refill_result.is_ok(), "refill should succeed");

    let refilled_snapshot = initialized_bytes(&first).to_vec();
    assert_eq!(refilled_snapshot.len(), 64);

    assert_ne!(
        first_snapshot, second_snapshot,
        "two independently filled buffers should not contain identical random bytes"
    );
    assert_ne!(
        first_snapshot, refilled_snapshot,
        "refilling the same buffer should replace the previous random bytes"
    );
}