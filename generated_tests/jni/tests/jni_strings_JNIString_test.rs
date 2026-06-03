use jni::strings::{JNIStr, JNIString};
use std::ffi::CStr;
use std::os::raw::c_char;

fn borrowed_text(value: &JNIString) -> String {
    let borrowed: &JNIStr = JNIString::borrowed(value);
    let raw: *const c_char = borrowed.as_ptr();

    assert!(
        !raw.is_null(),
        "JNIString::borrowed should expose a non-null NUL-terminated pointer"
    );

    unsafe { CStr::from_ptr(raw) }
        .to_str()
        .expect("borrowed JNI string should contain valid UTF-8")
        .to_owned()
}

#[test]
fn borrowed_exposes_the_original_utf8_contents_for_multiple_inputs() {
    let cases = [
        "",
        "ascii payload",
        "unicode payload: café ☕",
        "jni/class/Name",
        "(Ljava/lang/String;I)Z",
    ];

    for case in cases {
        let owned = JNIString::from(case);
        let borrowed: &JNIStr = JNIString::borrowed(&owned);
        let borrowed_again: &JNIStr = JNIString::borrowed(&owned);

        assert_eq!(
            borrowed_text(&owned),
            case,
            "borrowed JNIStr should decode to the same text used to build JNIString"
        );
        assert_eq!(
            borrowed.as_ptr(),
            borrowed_again.as_ptr(),
            "borrowing the same JNIString repeatedly should not allocate or change the backing pointer"
        );
        assert_eq!(
            unsafe { CStr::from_ptr(borrowed.as_ptr()) }.to_bytes().len(),
            case.as_bytes().len(),
            "borrowed JNI string should preserve the UTF-8 byte length before the trailing NUL"
        );
    }
}

#[test]
fn borrowed_pointer_can_be_reborrowed_with_jnistr_from_ptr() {
    let owned = JNIString::from("round-trip through raw JNIStr pointer");
    let borrowed: &JNIStr = JNIString::borrowed(&owned);
    let raw: *const c_char = borrowed.as_ptr();

    let reborrowed: &JNIStr = unsafe { JNIStr::from_ptr(raw) };

    assert_eq!(
        reborrowed.as_ptr(),
        raw,
        "JNIStr::from_ptr should borrow the exact pointer returned by JNIString::borrowed"
    );
    assert_eq!(
        unsafe { CStr::from_ptr(reborrowed.as_ptr()) }
            .to_str()
            .expect("reborrowed JNIStr should be valid UTF-8"),
        "round-trip through raw JNIStr pointer",
        "reborrowed JNIStr should expose the same contents"
    );
    assert_eq!(
        borrowed.as_ptr(),
        reborrowed.as_ptr(),
        "the original borrowed value and raw-pointer reborrow should point to the same storage"
    );
}

#[test]
fn borrowed_remains_stable_while_the_owned_jnistring_is_alive() {
    let owned = JNIString::from(String::from("stable borrowed view"));
    let first: &JNIStr = JNIString::borrowed(&owned);
    let first_ptr = first.as_ptr();
    let first_text = unsafe { CStr::from_ptr(first_ptr) }
        .to_str()
        .expect("first borrowed value should be valid UTF-8")
        .to_owned();

    let second_text = borrowed_text(&owned);
    let third: &JNIStr = JNIString::borrowed(&owned);

    assert_eq!(
        first_text, "stable borrowed view",
        "initial borrowed view should match the owned JNIString contents"
    );
    assert_eq!(
        second_text, first_text,
        "a later borrowed view should read the same contents"
    );
    assert_eq!(
        third.as_ptr(),
        first_ptr,
        "the borrowed pointer should remain stable while the JNIString is alive"
    );
    assert_eq!(
        unsafe { CStr::from_ptr(third.as_ptr()) }.to_bytes_with_nul().last(),
        Some(&0),
        "the borrowed JNI string should be NUL-terminated"
    );
}