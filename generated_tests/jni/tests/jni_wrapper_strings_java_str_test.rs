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
fn java_str_raw_pointer_can_be_observed_taken_and_restored_for_ascii_content() {
    let env = java_vm()
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let original_text = "raw pointer round trip from Rust";
    let jstring: JString<'_> = env
        .new_string(original_text)
        .expect("Java string should be created");

    let java_str = JavaStr::from_env(&env, &jstring).expect("JavaStr::from_env should succeed");
    let borrowed_raw = JavaStr::get_raw(&java_str);

    assert!(
        !borrowed_raw.is_null(),
        "JavaStr::get_raw should expose a non-null JNI UTF pointer"
    );

    let observed = unsafe { CStr::from_ptr(borrowed_raw) }
        .to_str()
        .expect("ASCII Java string should be valid UTF-8 through modified UTF-8 bytes");
    assert_eq!(
        observed, original_text,
        "raw JNI string bytes should match the Java string content for ASCII input"
    );

    let owned_raw = JavaStr::into_raw(java_str);
    assert_eq!(
        owned_raw, borrowed_raw,
        "JavaStr::into_raw should return the same pointer previously exposed by get_raw"
    );
    assert!(
        !owned_raw.is_null(),
        "JavaStr::into_raw should not turn a valid Java string pointer into null"
    );

    let restored = unsafe { JavaStr::from_raw(&env, &jstring, owned_raw) };
    let restored_raw = JavaStr::get_raw(&restored);
    assert_eq!(
        restored_raw, owned_raw,
        "JavaStr::from_raw should wrap the exact pointer returned by into_raw"
    );

    let restored_observed = unsafe { CStr::from_ptr(restored_raw) }
        .to_str()
        .expect("restored JavaStr should still expose valid ASCII bytes");
    assert_eq!(
        restored_observed, original_text,
        "JavaStr restored from a raw pointer should preserve readable string contents"
    );
}

#[test]
fn java_str_raw_pointer_workflow_handles_empty_java_string() {
    let env = java_vm()
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let jstring: JString<'_> = env
        .new_string("")
        .expect("empty Java string should be created");

    let java_str = JavaStr::from_env(&env, &jstring).expect("JavaStr::from_env should succeed");
    let raw = JavaStr::get_raw(&java_str);

    assert!(
        !raw.is_null(),
        "even an empty Java string should have a non-null JNI UTF pointer"
    );

    let bytes = unsafe { CStr::from_ptr(raw) }.to_bytes();
    assert!(
        bytes.is_empty(),
        "empty Java strings should expose an empty NUL-terminated byte sequence"
    );

    let raw_after_into = JavaStr::into_raw(java_str);
    assert_eq!(
        raw_after_into, raw,
        "into_raw should preserve the raw pointer for an empty Java string"
    );

    let restored = unsafe { JavaStr::from_raw(&env, &jstring, raw_after_into) };
    let restored_raw = JavaStr::get_raw(&restored);
    assert_eq!(
        restored_raw, raw_after_into,
        "from_raw should preserve the raw pointer for an empty Java string"
    );
    assert_eq!(
        unsafe { CStr::from_ptr(restored_raw) }.to_bytes(),
        b"",
        "restored empty JavaStr should still read as empty"
    );
}