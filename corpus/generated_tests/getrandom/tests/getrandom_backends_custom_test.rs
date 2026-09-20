#![cfg(getrandom_backend = "custom")]

use getrandom::backends::custom;
use getrandom::Error;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU8, Ordering};

static FILL_SEQUENCE: AtomicU8 = AtomicU8::new(0);

fn deterministic_custom_getrandom(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    let base = FILL_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    for (index, byte) in dest.iter_mut().enumerate() {
        byte.write(base.wrapping_add(index as u8).wrapping_add(11));
    }
    Ok(())
}

getrandom::register_custom_getrandom!(deterministic_custom_getrandom);

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_and_preserves_slice_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = custom::fill_inner(&mut empty);

    assert!(result.is_ok(), "custom fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_single_and_larger_buffers() {
    let mut one = [MaybeUninit::<u8>::uninit(); 1];
    let result = custom::fill_inner(&mut one);

    assert!(result.is_ok(), "custom fill_inner should initialize one byte");
    let one_bytes = initialized_bytes(&one);
    assert_eq!(one_bytes.len(), 1);

    let mut larger = [MaybeUninit::<u8>::uninit(); 64];
    let result = custom::fill_inner(&mut larger);

    assert!(result.is_ok(), "custom fill_inner should initialize larger buffers");
    let larger_bytes = initialized_bytes(&larger);
    assert_eq!(larger_bytes.len(), 64);

    for window in larger_bytes.windows(2) {
        assert_eq!(window[1], window[0].wrapping_add(1));
    }
}

#[test]
fn fill_inner_supports_chunked_multi_step_workflow() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 48];

    let (first, rest) = buffer.split_at_mut(7);
    let (middle, last) = rest.split_at_mut(19);

    let first_result = custom::fill_inner(first);
    let middle_result = custom::fill_inner(middle);
    let last_result = custom::fill_inner(last);

    assert!(first_result.is_ok(), "first chunk should be filled");
    assert!(middle_result.is_ok(), "middle chunk should be filled");
    assert!(last_result.is_ok(), "last chunk should be filled");

    let first_bytes = initialized_bytes(first);
    let middle_bytes = initialized_bytes(middle);
    let last_bytes = initialized_bytes(last);

    assert_eq!(first_bytes.len(), 7);
    assert_eq!(middle_bytes.len(), 19);
    assert_eq!(last_bytes.len(), 22);

    assert_eq!(first_bytes[1], first_bytes[0].wrapping_add(1));
    assert_eq!(middle_bytes[1], middle_bytes[0].wrapping_add(1));
    assert_eq!(last_bytes[1], last_bytes[0].wrapping_add(1));

    assert_ne!(
        first_bytes[0], middle_bytes[0],
        "separate custom backend calls should be distinguishable"
    );
    assert_ne!(
        middle_bytes[0], last_bytes[0],
        "later chunk should be produced by a later backend invocation"
    );
}

#[test]
fn fill_inner_overwrites_existing_initialized_bytes_when_reused_as_uninit() {
    let mut bytes = [0xAA_u8; 16];

    let uninit_view: &mut [MaybeUninit<u8>] =
        unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr().cast::<MaybeUninit<u8>>(), bytes.len()) };

    let result = custom::fill_inner(uninit_view);

    assert!(result.is_ok(), "custom fill_inner should fill a reused byte buffer");
    assert_eq!(bytes.len(), 16);
    assert_ne!(bytes, [0xAA_u8; 16]);

    for window in bytes.windows(2) {
        assert_eq!(window[1], window[0].wrapping_add(1));
    }
}