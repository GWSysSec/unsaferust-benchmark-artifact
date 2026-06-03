#[cfg(all(target_vendor = "apple", not(target_os = "macos")))]
mod apple_other_backend_tests {
    use getrandom::backends::apple_other;
    use std::mem::MaybeUninit;

    fn initialized_bytes(slice: &[MaybeUninit<u8>]) -> &[u8] {
        unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), slice.len()) }
    }

    #[test]
    fn fill_inner_accepts_empty_and_initializes_requested_buffer_lengths() {
        let mut empty: [MaybeUninit<u8>; 0] = [];
        let empty_ptr = empty.as_mut_ptr();

        let empty_result = apple_other::fill_inner(&mut empty);

        assert!(empty_result.is_ok(), "empty random fill should succeed");
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.as_mut_ptr(), empty_ptr);

        let mut one_byte = [MaybeUninit::<u8>::uninit(); 1];
        let one_byte_result = apple_other::fill_inner(&mut one_byte);

        assert!(one_byte_result.is_ok(), "single-byte random fill should succeed");
        assert_eq!(initialized_bytes(&one_byte).len(), 1);

        let mut larger = [MaybeUninit::<u8>::uninit(); 128];
        let larger_result = apple_other::fill_inner(&mut larger);

        assert!(larger_result.is_ok(), "larger random fill should succeed");
        assert_eq!(initialized_bytes(&larger).len(), 128);
    }

    #[test]
    fn fill_inner_supports_chunked_multi_step_workflow() {
        let mut buffer = [MaybeUninit::<u8>::uninit(); 96];

        let (first, rest) = buffer.split_at_mut(17);
        let (middle, last) = rest.split_at_mut(31);

        assert_eq!(first.len(), 17);
        assert_eq!(middle.len(), 31);
        assert_eq!(last.len(), 48);

        assert!(apple_other::fill_inner(first).is_ok(), "first chunk should fill");
        assert!(apple_other::fill_inner(middle).is_ok(), "middle chunk should fill");
        assert!(apple_other::fill_inner(last).is_ok(), "last chunk should fill");

        let initialized = initialized_bytes(&buffer);
        assert_eq!(initialized.len(), 96);

        let copied = initialized.to_vec();
        assert_eq!(copied.len(), buffer.len());
    }
}

#[cfg(not(all(target_vendor = "apple", not(target_os = "macos"))))]
mod portability_smoke_tests {
    #[test]
    fn public_random_api_smoke_test_on_non_apple_other_targets() {
        let mut buffer = [0u8; 32];

        let result = getrandom::fill(&mut buffer);

        assert!(result.is_ok(), "getrandom::fill should succeed on supported test targets");
        assert_eq!(buffer.len(), 32);

        let first = getrandom::u32();
        let second = getrandom::u64();

        assert!(first.is_ok(), "getrandom::u32 should succeed");
        assert!(second.is_ok(), "getrandom::u64 should succeed");
        assert_eq!(std::mem::size_of_val(&first.unwrap()), 4);
        assert_eq!(std::mem::size_of_val(&second.unwrap()), 8);
    }
}