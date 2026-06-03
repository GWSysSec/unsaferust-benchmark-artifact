#![cfg(feature = "invocation")]

use jni::objects::JString;
use jni::strings::JavaStr;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use std::ffi::CStr;
use std::sync::OnceLock;

static JVM: OnceLock<JavaVM> = OnceLock::new();

fn java_vm() -> &'static JavaVM {
    JVM.get_or_init(|| {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .expect("JVM init args should build successfully");

        JavaVM::new(args).expect("a JVM should be created for JavaStr integration tests")
    })
}

#[test]
fn java_str_from_env_exposes_stable_raw_modified_utf8_until_drop() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let jstring: JString<'_> = env
        .new_string("plain ascii from Java")
        .expect("Java string should be created");

    let java_str = JavaStr::from_env(&env, &jstring).expect("JavaStr::from_env should succeed");
    let raw = JavaStr::get_raw(&java_str);

    assert!(
        !raw.is_null(),
        "JavaStr::get_raw should return a non-null pointer for a valid Java string"
    );

    let c_str = unsafe { CStr::from_ptr(raw) };
    assert_eq!(
        c_str.to_bytes(),
        b"plain ascii from Java",
        "ASCII Java strings should be exposed unchanged as modified UTF-8 bytes"
    );
    assert_eq!(
        c_str.to_bytes_with_nul().last().copied(),
        Some(0),
        "raw JavaStr data should be NUL terminated"
    );

    let second_raw = JavaStr::get_raw(&java_str);
    assert_eq!(
        second_raw, raw,
        "repeated JavaStr::get_raw calls on the same JavaStr should return the same pointer"
    );
}

#[test]
fn java_str_into_raw_and_from_raw_round_trip_preserves_pointer_and_contents() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let jstring: JString<'_> = env
        .new_string("round trip through raw pointer")
        .expect("Java string should be created");

    let java_str = JavaStr::from_env(&env, &jstring).expect("JavaStr::from_env should succeed");
    let borrowed_raw = JavaStr::get_raw(&java_str);
    assert!(
        !borrowed_raw.is_null(),
        "JavaStr::get_raw should expose a non-null pointer before into_raw"
    );

    let owned_raw = JavaStr::into_raw(java_str);
    assert_eq!(
        owned_raw, borrowed_raw,
        "JavaStr::into_raw should return the same pointer previously exposed by get_raw"
    );

    let raw_contents = unsafe { CStr::from_ptr(owned_raw) };
    assert_eq!(
        raw_contents.to_bytes(),
        b"round trip through raw pointer",
        "the pointer returned by into_raw should still reference the string contents"
    );

    let reconstructed = unsafe { JavaStr::from_raw(&env, &jstring, owned_raw) };
    let reconstructed_raw = JavaStr::get_raw(&reconstructed);

    assert_eq!(
        reconstructed_raw, owned_raw,
        "JavaStr::from_raw should reconstruct a JavaStr around the exact raw pointer"
    );
    assert_eq!(
        unsafe { CStr::from_ptr(reconstructed_raw) }.to_bytes(),
        b"round trip through raw pointer",
        "a JavaStr reconstructed with from_raw should expose the original contents"
    );
}

#[test]
fn java_str_raw_workflow_handles_empty_java_string() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let empty: JString<'_> = env
        .new_string("")
        .expect("empty Java string should be created");

    let java_str = JavaStr::from_env(&env, &empty).expect("JavaStr::from_env should handle empty strings");
    let raw = JavaStr::get_raw(&java_str);

    assert!(
        !raw.is_null(),
        "even an empty Java string should have a non-null modified UTF-8 pointer"
    );
    assert_eq!(
        unsafe { CStr::from_ptr(raw) }.to_bytes(),
        b"",
        "empty Java strings should expose zero content bytes before the terminator"
    );
    assert_eq!(
        unsafe { CStr::from_ptr(raw) }.to_bytes_with_nul(),
        b"\0",
        "empty Java strings should expose exactly a NUL terminator"
    );

    let raw_from_into = JavaStr::into_raw(java_str);
    assert_eq!(
        raw_from_into, raw,
        "into_raw should preserve the empty string pointer returned by get_raw"
    );

    let reconstructed = unsafe { JavaStr::from_raw(&env, &empty, raw_from_into) };
    assert_eq!(
        unsafe { CStr::from_ptr(JavaStr::get_raw(&reconstructed)) }.to_bytes_with_nul(),
        b"\0",
        "from_raw should reconstruct an empty JavaStr that still reads as an empty C string"
    );
}