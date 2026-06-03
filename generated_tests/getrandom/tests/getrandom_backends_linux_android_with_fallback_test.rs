#![cfg(all(
    any(target_os = "linux", target_os = "android"),
    not(any(
        getrandom_backend = "linux_getrandom",
        getrandom_backend = "linux_raw",
        getrandom_backend = "custom"
    ))
))]

use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_handles_empty_single_and_large_buffers() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let empty_ptr = empty.as_mut_ptr();

    let empty_result = getrandom::fill_uninit(&mut empty);

    assert!(
        empty_result.is_ok(),
        "empty linux/android fallback fill should succeed"
    );
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), empty_ptr);

    let mut single = [MaybeUninit::<u8>::uninit(); 1];
    let single_ptr = single.as_mut_ptr();

    let single_result = getrandom::fill_uninit(&mut single);

    assert!(
        single_result.is_ok(),
        "single-byte linux/android fallback fill should succeed"
    );
    assert_eq!(initialized_bytes(&single).len(), 1);
    assert_eq!(single.as_mut_ptr(), single_ptr);

    let mut large = [MaybeUninit::<u8>::uninit(); 1024];
    let large_ptr = large.as_mut_ptr();

    let large_result = getrandom::fill_uninit(&mut large);

    assert!(
        large_result.is_ok(),
        "large linux/android fallback fill should succeed"
    );
    assert_eq!(initialized_bytes(&large).len(), 1024);
    assert_eq!(large.as_mut_ptr(), large_ptr);
}

#[test]
fn fill_can_be_used_in_multi_step_buffer_workflow() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 257];
    let original_ptr = buffer.as_mut_ptr();

    {
        let (prefix, remainder) = buffer.split_at_mut(17);
        let (middle, suffix) = remainder.split_at_mut(200);

        assert!(
            getrandom::fill_uninit(prefix).is_ok(),
            "prefix fill should succeed"
        );
        assert!(
            getrandom::fill_uninit(middle).is_ok(),
            "middle fill should succeed"
        );
        assert!(
            getrandom::fill_uninit(suffix).is_ok(),
            "suffix fill should succeed"
        );

        assert_eq!(prefix.len(), 17);
        assert_eq!(middle.len(), 200);
        assert_eq!(suffix.len(), 40);
    }

    assert_eq!(buffer.as_mut_ptr(), original_ptr);
    assert_eq!(initialized_bytes(&buffer).len(), 257);
}

#[test]
fn fill_writes_only_the_requested_subslice() {
    let mut storage = [0xA5_u8; 66];
    storage[65] = 0x5A;

    let target = unsafe {
        std::slice::from_raw_parts_mut(storage[1..65].as_mut_ptr().cast::<MaybeUninit<u8>>(), 64)
    };

    let result = getrandom::fill_uninit(target);

    assert!(
        result.is_ok(),
        "subslice linux/android fallback fill should succeed"
    );
    assert_eq!(storage[0], 0xA5, "byte before requested slice changed");
    assert_eq!(storage[65], 0x5A, "byte after requested slice changed");
    assert_eq!(storage.len(), 66);
}