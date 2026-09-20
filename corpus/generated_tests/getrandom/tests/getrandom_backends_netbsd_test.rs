#![cfg(target_os = "netbsd")]

use getrandom::backends::netbsd;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_and_preserves_slice_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = netbsd::fill_inner(&mut empty);

    assert!(result.is_ok(), "netbsd fill_inner should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_initializes_single_and_larger_buffers_without_reallocating() {
    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = netbsd::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "netbsd fill_inner should initialize a one-byte buffer"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut larger = [MaybeUninit::<u8>::uninit(); 512];
    let larger_ptr = larger.as_mut_ptr();

    let larger_result = netbsd::fill_inner(&mut larger);

    assert!(
        larger_result.is_ok(),
        "netbsd fill_inner should initialize a larger buffer"
    );
    assert_eq!(larger.as_mut_ptr(), larger_ptr);
    assert_eq!(initialized_bytes(&larger).len(), 512);
}

#[test]
fn fill_inner_only_writes_the_requested_subslice() {
    let sentinel = 0xCC;
    let mut storage = [MaybeUninit::new(sentinel); 34];
    let storage_ptr = storage.as_mut_ptr();

    let result = netbsd::fill_inner(&mut storage[1..33]);

    assert!(
        result.is_ok(),
        "netbsd fill_inner should fill a middle subslice"
    );
    assert_eq!(storage.as_mut_ptr(), storage_ptr);

    let bytes = initialized_bytes(&storage);
    assert_eq!(bytes.len(), 34);
    assert_eq!(bytes[0], sentinel, "byte before filled range must be untouched");
    assert_eq!(
        bytes[33], sentinel,
        "byte after filled range must be untouched"
    );
    assert_eq!(&bytes[1..33].len(), &32);
}

#[test]
fn repeated_fills_can_be_chained_over_distinct_buffers() {
    let mut first = [MaybeUninit::<u8>::uninit(); 32];
    let mut second = [MaybeUninit::<u8>::uninit(); 32];

    let first_result = netbsd::fill_inner(&mut first);
    let second_result = netbsd::fill_inner(&mut second);

    assert!(first_result.is_ok(), "first netbsd fill should succeed");
    assert!(second_result.is_ok(), "second netbsd fill should succeed");
    assert_eq!(initialized_bytes(&first).len(), initialized_bytes(&second).len());

    let first_ptr = first.as_ptr();
    let second_ptr = second.as_ptr();
    assert_ne!(
        first_ptr, second_ptr,
        "separate destination buffers should remain distinct"
    );
}