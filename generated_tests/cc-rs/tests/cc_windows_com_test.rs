#[cfg(windows)]
mod windows_com_tests {
    use std::ffi::OsString;
    use std::ptr;

    #[test]
    fn com_initialize_can_be_called_more_than_once() {
        let first = cc::windows::com::initialize();
        let second = cc::windows::com::initialize();

        assert!(
            first.is_ok(),
            "first COM initialization should succeed or report an accepted initialized state: {first:?}"
        );
        assert!(
            second.is_ok(),
            "second COM initialization should also be accepted: {second:?}"
        );
    }

    #[test]
    fn null_comptr_round_trip_exposes_null_pointer_state() {
        let ptr: *mut std::ffi::c_void = ptr::null_mut();

        let com_ptr = unsafe { cc::windows::com::from_raw(ptr) };

        assert!(
            com_ptr.is_null(),
            "a ComPtr created from a null raw pointer should remain null"
        );

        let cast_result = com_ptr.cast::<std::ffi::c_void>();
        assert!(
            cast_result.is_err(),
            "casting a null COM pointer should fail instead of producing a valid pointer"
        );
    }

    #[test]
    fn bstr_to_osstring_handles_empty_string() {
        let empty = cc::windows::com::BStr::new("");

        let os_string = empty.to_osstring();

        assert_eq!(os_string, OsString::from(""));
        assert!(os_string.is_empty());
    }

    #[test]
    fn bstr_to_osstring_preserves_unicode_content() {
        let original = "Visual Studio 🦀 build tools";
        let bstr = cc::windows::com::BStr::new(original);

        let os_string = bstr.to_osstring();

        assert_eq!(os_string, OsString::from(original));
        assert_eq!(os_string.to_string_lossy(), original);
    }
}

#[cfg(not(windows))]
mod non_windows_smoke_tests {
    #[test]
    fn windows_com_api_is_windows_only_for_external_consumers() {
        assert!(
            !cfg!(windows),
            "this fallback test should only run on non-Windows targets"
        );
        assert_eq!(std::env::consts::FAMILY, "unix");
        assert_ne!(std::env::consts::OS, "windows");
    }
}