#![cfg(getrandom_backend = "use_file")]

use getrandom::backends::use_file;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_slice_and_preserves_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = use_file::fill_inner(&mut empty);

    assert!(result.is_ok(), "use_file fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_single_and_large_buffers_without_reallocating() {
    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = use_file::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "use_file fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut large = [MaybeUninit::<u8>::uninit(); 1024];
    let large_ptr = large.as_mut_ptr();

    let large_result = use_file::fill_inner(&mut large);

    assert!(
        large_result.is_ok(),
        "use_file fill_inner should initialize a larger buffer"
    );
    assert_eq!(large.as_mut_ptr(), large_ptr);
    assert_eq!(initialized_bytes(&large).len(), 1024);
}

#[test]
fn fill_inner_only_writes_requested_subslice() {
    let sentinel = 0xA5;
    let mut buffer = [MaybeUninit::new(sentinel); 64];

    let result = use_file::fill_inner(&mut buffer[16..48]);

    assert!(
        result.is_ok(),
        "use_file fill_inner should initialize a middle subslice"
    );

    let bytes = initialized_bytes(&buffer);
    assert_eq!(bytes.len(), 64);
    assert!(
        bytes[..16].iter().all(|&byte| byte == sentinel),
        "prefix outside the destination subslice must remain unchanged"
    );
    assert!(
        bytes[48..].iter().all(|&byte| byte == sentinel),
        "suffix outside the destination subslice must remain unchanged"
    );
    assert_eq!(bytes[16..48].len(), 32);
}

#[test]
fn repeated_fill_inner_calls_can_reuse_the_same_buffer() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 128];
    let original_ptr = buffer.as_mut_ptr();

    let first = use_file::fill_inner(&mut buffer);
    assert!(first.is_ok(), "first use_file fill_inner call should succeed");

    let first_snapshot = initialized_bytes(&buffer).to_vec();
    assert_eq!(first_snapshot.len(), 128);

    let second = use_file::fill_inner(&mut buffer);
    assert!(second.is_ok(), "second use_file fill_inner call should succeed");

    let second_snapshot = initialized_bytes(&buffer).to_vec();
    assert_eq!(second_snapshot.len(), 128);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
}