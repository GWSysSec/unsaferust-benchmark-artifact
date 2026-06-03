#![cfg(target_os = "espidf")]

use getrandom::backends::esp_idf;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_handles_empty_single_and_larger_buffers() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = esp_idf::fill_inner(&mut empty);

    assert!(empty_result.is_ok(), "empty ESP-IDF RNG fill should succeed");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut one_byte = [MaybeUninit::<u8>::uninit(); 1];
    let one_byte_result = esp_idf::fill_inner(&mut one_byte);

    assert!(
        one_byte_result.is_ok(),
        "single-byte ESP-IDF RNG fill should succeed"
    );
    assert_eq!(initialized_bytes(&one_byte).len(), 1);

    let mut larger = [MaybeUninit::<u8>::uninit(); 128];
    let larger_result = esp_idf::fill_inner(&mut larger);

    assert!(
        larger_result.is_ok(),
        "larger ESP-IDF RNG fill should succeed"
    );
    assert_eq!(initialized_bytes(&larger).len(), 128);
}

#[test]
fn fill_inner_supports_chunked_workflow_without_reallocating() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 96];
    let original_ptr = buffer.as_mut_ptr();

    let (first, rest) = buffer.split_at_mut(13);
    let (middle, last) = rest.split_at_mut(47);

    let first_result = esp_idf::fill_inner(first);
    let middle_result = esp_idf::fill_inner(middle);
    let last_result = esp_idf::fill_inner(last);

    assert!(first_result.is_ok(), "first chunk fill should succeed");
    assert!(middle_result.is_ok(), "middle chunk fill should succeed");
    assert!(last_result.is_ok(), "last chunk fill should succeed");

    assert_eq!(buffer.as_mut_ptr(), original_ptr);
    assert_eq!(buffer.len(), 96);

    let initialized = initialized_bytes(&buffer);
    assert_eq!(initialized.len(), 96);
    assert_eq!(first.len(), 13);
    assert_eq!(middle.len(), 47);
    assert_eq!(last.len(), 36);
}

#[test]
fn repeated_fill_inner_calls_can_reuse_the_same_buffer() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 64];
    let original_ptr = buffer.as_mut_ptr();

    let first_result = esp_idf::fill_inner(&mut buffer);
    assert!(first_result.is_ok(), "initial fill should succeed");

    let first_snapshot = initialized_bytes(&buffer).to_vec();
    assert_eq!(first_snapshot.len(), 64);

    let second_result = esp_idf::fill_inner(&mut buffer);
    assert!(second_result.is_ok(), "second fill should succeed");

    let second_snapshot = initialized_bytes(&buffer).to_vec();
    assert_eq!(second_snapshot.len(), 64);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);

    let third_result = esp_idf::fill_inner(&mut buffer[..7]);
    assert!(third_result.is_ok(), "partial refill should succeed");
    assert_eq!(initialized_bytes(&buffer).len(), 64);
}