#![cfg(target_os = "hermit")]

use getrandom::backends::hermit;
use std::mem::MaybeUninit;

fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
}

#[test]
fn fill_inner_accepts_empty_slice_without_changing_identity() {
    let mut empty: [MaybeUninit<u8>; 0] = [];
    let original_ptr = empty.as_mut_ptr();

    let result = hermit::fill_inner(&mut empty);

    assert!(result.is_ok(), "empty Hermit RNG fill should succeed");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.as_mut_ptr(), original_ptr);
}

#[test]
fn fill_inner_handles_lengths_across_word_boundaries() {
    for len in [1_usize, 3, 4, 7, 8, 9, 15, 16, 31, 32, 33] {
        let mut buffer = vec![MaybeUninit::<u8>::uninit(); len];
        let original_ptr = buffer.as_mut_ptr();
        let original_len = buffer.len();

        let result = hermit::fill_inner(&mut buffer);

        assert!(
            result.is_ok(),
            "Hermit RNG fill should succeed for length {len}"
        );
        assert_eq!(buffer.len(), original_len);
        assert_eq!(buffer.as_mut_ptr(), original_ptr);

        let bytes = initialized_bytes(&buffer);
        assert_eq!(bytes.len(), len);
    }
}

#[test]
fn fill_inner_only_writes_the_requested_subslice() {
    let mut buffer = [MaybeUninit::new(0xA5_u8); 24];

    let result = hermit::fill_inner(&mut buffer[5..19]);

    assert!(result.is_ok(), "Hermit RNG fill should succeed for subslice");

    let bytes = initialized_bytes(&buffer);
    assert_eq!(&bytes[..5], &[0xA5; 5]);
    assert_eq!(&bytes[19..], &[0xA5; 5]);
    assert_eq!(bytes.len(), 24);
}

#[test]
fn fill_inner_can_be_called_repeatedly_on_the_same_buffer() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 64];
    let original_ptr = buffer.as_mut_ptr();

    let first = hermit::fill_inner(&mut buffer);
    assert!(first.is_ok(), "first Hermit RNG fill should succeed");

    let first_snapshot = initialized_bytes(&buffer).to_vec();
    assert_eq!(first_snapshot.len(), 64);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);

    let second = hermit::fill_inner(&mut buffer);
    assert!(second.is_ok(), "second Hermit RNG fill should succeed");

    let second_snapshot = initialized_bytes(&buffer).to_vec();
    assert_eq!(second_snapshot.len(), 64);
    assert_eq!(buffer.as_mut_ptr(), original_ptr);
}