use jni::strings::JNIStr;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[test]
fn jni_str_from_ptr_matches_c_string_bytes_and_pointer() {
    let original = CString::new("java/lang/String").expect("test string has no interior nul");
    let raw: *const c_char = original.as_ptr();

    let jni_str: &JNIStr = unsafe { jni::strings::JNIStr::from_ptr(raw) };
    let c_str: &CStr = unsafe { CStr::from_ptr(raw) };

    assert_eq!(jni_str.as_ptr(), raw);
    assert_eq!(jni_str.to_bytes(), b"java/lang/String");
    assert_eq!(jni_str.to_bytes_with_nul(), b"java/lang/String\0");
    assert_eq!(jni_str.to_bytes(), c_str.to_bytes());
    assert_eq!(jni_str.to_str().expect("valid UTF-8"), "java/lang/String");
}

#[test]
fn jni_str_from_ptr_handles_empty_and_unicode_strings() {
    let cases = [
        ("", b"".as_slice()),
        ("hello", b"hello".as_slice()),
        ("com/example/\u{2603}", "com/example/\u{2603}".as_bytes()),
    ];

    for (input, expected_bytes) in cases {
        let original = CString::new(input).expect("case has no interior nul");
        let raw = original.as_ptr();

        let jni_str = unsafe { JNIStr::from_ptr(raw) };

        assert_eq!(jni_str.as_ptr(), raw);
        assert_eq!(jni_str.to_bytes(), expected_bytes);
        assert_eq!(jni_str.to_bytes_with_nul().last().copied(), Some(0));
        assert_eq!(jni_str.to_str().expect("case is valid UTF-8"), input);
        assert_eq!(unsafe { CStr::from_ptr(jni_str.as_ptr()) }.to_bytes(), expected_bytes);
    }
}

#[test]
fn jni_str_from_ptr_can_be_used_multiple_times_for_same_stable_c_string() {
    let original = CString::new("([Ljava/lang/String;)V").expect("signature has no interior nul");
    let raw = original.as_ptr();

    let first = unsafe { JNIStr::from_ptr(raw) };
    let second = unsafe { jni::strings::JNIStr::from_ptr(raw) };

    assert_eq!(first.as_ptr(), second.as_ptr());
    assert_eq!(first.to_bytes(), second.to_bytes());
    assert_eq!(first.to_bytes_with_nul(), second.to_bytes_with_nul());
    assert_eq!(first.to_str().expect("valid UTF-8"), "([Ljava/lang/String;)V");
    assert_eq!(second.to_str().expect("valid UTF-8"), "([Ljava/lang/String;)V");
}