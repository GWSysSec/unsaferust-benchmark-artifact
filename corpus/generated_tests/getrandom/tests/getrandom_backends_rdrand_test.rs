#![cfg(getrandom_backend = "rdrand")]

use getrandom::backends::rdrand;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_single_and_larger_buffers_without_reallocating() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = rdrand::fill_inner(&mut empty);

    assert!(empty_result.is_ok(), "rdrand should accept an empty slice");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = rdrand::fill_inner(&mut single);

    assert!(
        single_result.is_ok(),
        "rdrand should initialize a one-byte slice"
    );
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut larger = [MaybeUninit::<u8>::uninit(); 257];
    let larger_ptr = larger.as_mut_ptr();

    let larger_result = rdrand::fill_inner(&mut larger);

    assert!(
        larger_result.is_ok(),
        "rdrand should initialize a larger non-word-aligned slice"
    );
    assert_eq!(larger.as_mut_ptr(), larger_ptr);
    assert_eq!(initialized_bytes(&larger).len(), 257);
}

#[test]
fn fill_inner_only_writes_to_the_requested_subslice() {
    let mut buffer = [MaybeUninit::new(0xA5_u8); 64];
    let original_ptr = buffer.as_mut_ptr();

    let result = rdrand::fill_inner(&mut buffer[11..53]);

    assert!(
        result.is_ok(),
        "rdrand should fill a middle subslice successfully"
    );
    assert_eq!(buffer.as_mut_ptr(), original_ptr);

    let bytes = initialized_bytes(&buffer);
    assert_eq!(bytes.len(), 64);
    assert!(
        bytes[..11].iter().all(|&byte| byte == 0xA5),
        "prefix outside the destination subslice must remain untouched"
    );
    assert!(
        bytes[53..].iter().all(|&byte| byte == 0xA5),
        "suffix outside the destination subslice must remain untouched"
    );
}

#[test]
fn fill_inner_can_be_called_repeatedly_on_independent_buffers() {
    let mut first = [MaybeUninit::<u8>::uninit(); 32];
    let mut second = [MaybeUninit::<u8>::uninit(); 32];

    let first_ptr = first.as_mut_ptr();
    let second_ptr = second.as_mut_ptr();

    let first_result = rdrand::fill_inner(&mut first);
    let second_result = rdrand::fill_inner(&mut second);

    assert!(first_result.is_ok(), "first rdrand fill should succeed");
    assert!(second_result.is_ok(), "second rdrand fill should succeed");
    assert_eq!(first.as_mut_ptr(), first_ptr);
    assert_eq!(second.as_mut_ptr(), second_ptr);
    assert_eq!(initialized_bytes(&first).len(), initialized_bytes(&second).len());
}