use jni::strings::{JNIStr, JNIString};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn jni_str_to_rust_string(value: &JNIStr) -> String {
    let raw: *const c_char = value.as_ptr();
    assert!(
        !raw.is_null(),
        "a borrowed JNIStr should expose a non-null C string pointer"
    );

    unsafe { CStr::from_ptr(raw) }
        .to_str()
        .expect("JNIStr contents should be valid UTF-8")
        .to_owned()
}

#[test]
fn borrowed_jni_string_preserves_contents_and_pointer_stability() {
    let cases = [
        "",
        "plain ascii",
        "jni/class/Name",
        "(Ljava/lang/String;I)Z",
        "unicode café ☕",
    ];

    for case in cases {
        let owned = JNIString::from(case);

        let first_borrow: &JNIStr = JNIString::borrowed(&owned);
        let second_borrow: &JNIStr = JNIString::borrowed(&owned);

        let first_raw = first_borrow.as_ptr();
        let second_raw = second_borrow.as_ptr();

        assert!(
            !first_raw.is_null(),
            "JNIString::borrowed should expose a non-null pointer"
        );
        assert_eq!(
            first_raw, second_raw,
            "repeated borrows from the same JNIString should point at the same backing storage"
        );
        assert_eq!(
            jni_str_to_rust_string(first_borrow),
            case,
            "borrowed JNI string should decode to the original input"
        );
        assert_eq!(
            jni_str_to_rust_string(second_borrow),
            case,
            "a second borrow should decode identically"
        );
    }
}

#[test]
fn from_ptr_borrows_existing_nul_terminated_storage_without_copying() {
    let original = CString::new("java/lang/String").expect("test input contains no interior NUL");
    let raw: *const c_char = original.as_ptr();

    let borrowed_from_ptr: &JNIStr = unsafe { JNIStr::from_ptr(raw) };

    assert_eq!(
        borrowed_from_ptr.as_ptr(),
        raw,
        "JNIStr::from_ptr should borrow the exact pointer it was given"
    );
    assert_eq!(
        jni_str_to_rust_string(borrowed_from_ptr),
        "java/lang/String",
        "JNIStr::from_ptr should expose the original NUL-terminated bytes"
    );

    let owned_copy = JNIString::from(jni_str_to_rust_string(borrowed_from_ptr).as_str());
    let borrowed_owned_copy: &JNIStr = JNIString::borrowed(&owned_copy);

    assert_eq!(
        jni_str_to_rust_string(borrowed_owned_copy),
        "java/lang/String",
        "a JNIString rebuilt from a JNIStr should preserve the textual value"
    );
    assert_ne!(
        borrowed_owned_copy.as_ptr(),
        raw,
        "rebuilding as JNIString should create independent owned storage"
    );

    assert_eq!(
        unsafe { CStr::from_ptr(raw) },
        original.as_c_str(),
        "borrowing through JNIStr::from_ptr should not mutate the original CString"
    );
}

#[test]
fn borrowed_and_from_ptr_agree_for_empty_c_string_edge_case() {
    let empty_c_string = CString::new("").expect("empty CString should be valid");
    let from_raw_empty: &JNIStr = unsafe { JNIStr::from_ptr(empty_c_string.as_ptr()) };

    let empty_jni_string = JNIString::from("");
    let borrowed_empty: &JNIStr = JNIString::borrowed(&empty_jni_string);

    assert_eq!(
        jni_str_to_rust_string(from_raw_empty),
        "",
        "JNIStr::from_ptr should handle an empty C string"
    );
    assert_eq!(
        jni_str_to_rust_string(borrowed_empty),
        "",
        "JNIString::borrowed should handle an empty JNIString"
    );
    assert_eq!(
        unsafe { CStr::from_ptr(from_raw_empty.as_ptr()) }.to_bytes_with_nul(),
        &[0],
        "the from_ptr empty string should still be represented by a single NUL byte"
    );
    assert_eq!(
        unsafe { CStr::from_ptr(borrowed_empty.as_ptr()) }.to_bytes_with_nul(),
        &[0],
        "the borrowed empty JNIString should still be represented by a single NUL byte"
    );
}