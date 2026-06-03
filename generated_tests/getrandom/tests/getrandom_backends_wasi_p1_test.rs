#![cfg(getrandom_backend = "wasi_p1")]

use getrandom::backends::wasi_p1;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_handles_empty_single_and_large_buffers_without_reallocating() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = wasi_p1::fill_inner(&mut empty);

    assert!(empty_result.is_ok(), "empty WASI preview1 fill should succeed");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = wasi_p1::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "single-byte WASI preview1 fill should succeed"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut large = [MaybeUninit::<u8>::uninit(); 1024];
    let large_ptr = large.as_mut_ptr();

    let large_result = wasi_p1::fill_inner(&mut large);

    assert!(
        large_result.is_ok(),
        "large WASI preview1 fill should succeed"
    );
    assert_eq!(large.as_mut_ptr(), large_ptr);
    assert_eq!(initialized_bytes(&large).len(), 1024);
}

#[test]
fn fill_inner_can_be_used_for_repeated_chunked_random_data_workflow() {
    let mut first = [MaybeUninit::<u8>::uninit(); 64];
    let mut second = [MaybeUninit::<u8>::uninit(); 64];
    let first_ptr = first.as_mut_ptr();
    let second_ptr = second.as_mut_ptr();

    let first_result = wasi_p1::fill_inner(&mut first);
    let second_result = wasi_p1::fill_inner(&mut second);

    assert!(first_result.is_ok(), "first WASI preview1 fill should succeed");
    assert!(
        second_result.is_ok(),
        "second WASI preview1 fill should succeed"
    );
    assert_eq!(first.as_mut_ptr(), first_ptr);
    assert_eq!(second.as_mut_ptr(), second_ptr);

    let first_bytes = initialized_bytes(&first);
    let second_bytes = initialized_bytes(&second);

    assert_eq!(first_bytes.len(), 64);
    assert_eq!(second_bytes.len(), 64);
    assert_ne!(
        first_bytes, second_bytes,
        "two independently filled 64-byte buffers should not be identical"
    );

    let combined: Vec<u8> = first_bytes
        .iter()
        .copied()
        .chain(second_bytes.iter().copied())
        .collect();

    assert_eq!(combined.len(), 128);
    assert_eq!(&combined[..64], first_bytes);
    assert_eq!(&combined[64..], second_bytes);
}