#![cfg(any(target_os = "solaris", target_os = "illumos"))]

use getrandom::backends::solaris;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_buffer_without_changing_slice_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = solaris::fill_inner(&mut empty);

    assert!(result.is_ok(), "solaris fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_single_and_large_buffers_without_reallocating() {
    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = solaris::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "solaris fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut large = [MaybeUninit::<u8>::uninit(); 1024];
    let large_ptr = large.as_mut_ptr();

    let large_result = solaris::fill_inner(&mut large);

    assert!(
        large_result.is_ok(),
        "solaris fill_inner should initialize a larger buffer"
    );
    assert_eq!(large.as_mut_ptr(), large_ptr);
    assert_eq!(initialized_bytes(&large).len(), 1024);
}

#[test]
fn fill_inner_only_writes_the_requested_subslice() {
    let mut buffer = [MaybeUninit::new(0xA5_u8); 64];

    let result = solaris::fill_inner(&mut buffer[16..48]);

    assert!(
        result.is_ok(),
        "solaris fill_inner should initialize a middle subslice"
    );

    let initialized = initialized_bytes(&buffer);
    assert_eq!(&initialized[..16], &[0xA5_u8; 16]);
    assert_eq!(&initialized[48..], &[0xA5_u8; 16]);
    assert_eq!(initialized.len(), 64);
}

#[test]
fn fill_inner_can_be_called_repeatedly_on_the_same_buffer() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 128];
    let original_ptr = buffer.as_mut_ptr();

    let first = solaris::fill_inner(&mut buffer);
    assert!(first.is_ok(), "first solaris fill_inner call should succeed");
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
    assert_eq!(initialized_bytes(&buffer).len(), 128);

    let second = solaris::fill_inner(&mut buffer);
    assert!(second.is_ok(), "second solaris fill_inner call should succeed");
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
    assert_eq!(initialized_bytes(&buffer).len(), 128);
}