#![cfg(all(target_arch = "wasm32", getrandom_backend = "wasm_js"))]

use core::mem::MaybeUninit;
use getrandom::backends::wasm_js;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_buffer_and_preserves_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = wasm_js::fill_inner(&mut empty);

    assert!(result.is_ok(), "wasm_js fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_single_and_larger_buffers_without_reallocating() {
    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = wasm_js::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "wasm_js fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut larger = [MaybeUninit::<u8>::uninit(); 256];
    let larger_ptr = larger.as_mut_ptr();

    let larger_result = wasm_js::fill_inner(&mut larger);

    assert!(
        larger_result.is_ok(),
        "wasm_js fill_inner should initialize a larger buffer"
    );
    assert_eq!(larger.as_mut_ptr(), larger_ptr);
    assert_eq!(initialized_bytes(&larger).len(), 256);
}

#[test]
fn fill_inner_can_be_used_to_fill_adjacent_chunks_of_one_buffer() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 96];
    let buffer_ptr = buffer.as_mut_ptr();

    let (first, rest) = buffer.split_at_mut(32);
    let (second, third) = rest.split_at_mut(32);

    let first_result = wasm_js::fill_inner(first);
    let second_result = wasm_js::fill_inner(second);
    let third_result = wasm_js::fill_inner(third);

    assert!(first_result.is_ok(), "first chunk should be filled");
    assert!(second_result.is_ok(), "second chunk should be filled");
    assert!(third_result.is_ok(), "third chunk should be filled");

    assert_eq!(first.len(), 32);
    assert_eq!(second.len(), 32);
    assert_eq!(third.len(), 32);

    assert_eq!(first.as_mut_ptr(), buffer_ptr);
    assert_eq!(second.as_mut_ptr(), unsafe { buffer_ptr.add(32) });
    assert_eq!(third.as_mut_ptr(), unsafe { buffer_ptr.add(64) });

    let initialized = initialized_bytes(&buffer);
    assert_eq!(initialized.len(), 96);
}