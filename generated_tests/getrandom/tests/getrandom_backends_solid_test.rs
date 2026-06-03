#![cfg(getrandom_backend = "solid")]

use getrandom::backends::solid;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn solid_fill_inner_handles_empty_single_and_larger_buffers() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = solid::fill_inner(&mut empty);

    assert!(empty_result.is_ok(), "empty SOLID fill should succeed");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = solid::fill_inner(&mut single);

    assert!(single_result.is_ok(), "single-byte SOLID fill should succeed");
    assert_eq!(single.as_mut_ptr(), single_ptr);
    assert_eq!(initialized_bytes(&single).len(), 1);

    let mut larger = [MaybeUninit::<u8>::uninit(); 256];
    let larger_ptr = larger.as_mut_ptr();

    let larger_result = solid::fill_inner(&mut larger);

    assert!(larger_result.is_ok(), "larger SOLID fill should succeed");
    assert_eq!(larger.as_mut_ptr(), larger_ptr);
    assert_eq!(initialized_bytes(&larger).len(), 256);
}

#[test]
fn direct_solid_rng_call_reports_success_and_can_be_followed_by_fill_inner() {
    let mut direct = [0_u8; 64];
    let direct_ptr = direct.as_mut_ptr();

    let status = solid::SOLID_RNG_SampleRandomBytes(direct.as_mut_ptr(), direct.len());

    assert_eq!(status, 0, "SOLID_RNG_SampleRandomBytes should report success");
    assert_eq!(direct.as_mut_ptr(), direct_ptr);
    assert_eq!(direct.len(), 64);

    let mut via_backend = [MaybeUninit::<u8>::uninit(); 64];
    let backend_ptr = via_backend.as_mut_ptr();

    let backend_result = solid::fill_inner(&mut via_backend);

    assert!(
        backend_result.is_ok(),
        "fill_inner should succeed after a direct SOLID RNG call"
    );
    assert_eq!(via_backend.as_mut_ptr(), backend_ptr);
    assert_eq!(initialized_bytes(&via_backend).len(), direct.len());
}

#[test]
fn direct_solid_rng_accepts_zero_length_request_without_touching_buffer() {
    let mut sentinel = [0xA5_u8; 8];
    let before = sentinel;
    let ptr = sentinel.as_mut_ptr();

    let status = solid::SOLID_RNG_SampleRandomBytes(ptr, 0);

    assert_eq!(status, 0, "zero-length SOLID RNG request should succeed");
    assert_eq!(sentinel, before);
    assert_eq!(sentinel.as_mut_ptr(), ptr);
}