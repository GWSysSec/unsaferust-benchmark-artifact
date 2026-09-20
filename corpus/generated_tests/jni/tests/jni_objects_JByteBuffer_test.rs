#![cfg(feature = "invocation")]

use jni::objects::JByteBuffer;
use jni::sys::jobject;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use std::ptr;
use std::sync::OnceLock;

static JVM: OnceLock<JavaVM> = OnceLock::new();

fn java_vm() -> &'static JavaVM {
    JVM.get_or_init(|| {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .expect("JVM init args should build successfully");

        JavaVM::new(args).expect("a JVM should be created for JByteBuffer integration tests")
    })
}

#[test]
fn byte_buffer_from_raw_round_trips_direct_buffer_and_preserves_native_access() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let mut backing = vec![10_u8, 20, 30, 40, 50, 60];
    let original_ptr = backing.as_mut_ptr();
    let original_len = backing.len();

    let direct = unsafe {
        env.new_direct_byte_buffer(original_ptr, original_len)
            .expect("direct byte buffer should be created from native memory")
    };

    let raw: jobject = JByteBuffer::into_raw(direct);
    assert!(!raw.is_null(), "direct ByteBuffer raw jobject must not be null");

    let wrapped = unsafe { JByteBuffer::from_raw(raw) };

    let capacity = env
        .get_direct_buffer_capacity(&wrapped)
        .expect("wrapped raw ByteBuffer should still expose its capacity");
    assert_eq!(capacity, original_len);

    let recovered_ptr = env
        .get_direct_buffer_address(&wrapped)
        .expect("wrapped raw ByteBuffer should still expose its native address");
    assert_eq!(recovered_ptr, original_ptr);

    unsafe {
        *recovered_ptr.add(2) = 99;
    }
    assert_eq!(backing, vec![10_u8, 20, 99, 40, 50, 60]);

    backing[4] = 77;
    let observed_from_buffer = unsafe { *recovered_ptr.add(4) };
    assert_eq!(observed_from_buffer, 77);

    let raw_again: jobject = JByteBuffer::into_raw(wrapped);
    assert_eq!(raw_again, raw);

    let rewrapped = unsafe { JByteBuffer::from_raw(raw_again) };
    let capacity_after_second_wrap = env
        .get_direct_buffer_capacity(&rewrapped)
        .expect("ByteBuffer should remain usable after a second from_raw reconstruction");
    assert_eq!(capacity_after_second_wrap, original_len);
}

#[test]
fn byte_buffer_from_raw_and_into_raw_preserve_null_without_jvm_calls() {
    let null_raw: jobject = ptr::null_mut();

    let wrapped_null = unsafe { JByteBuffer::from_raw(null_raw) };
    let round_tripped_null: jobject = JByteBuffer::into_raw(wrapped_null);

    assert!(round_tripped_null.is_null());
    assert_eq!(round_tripped_null, null_raw);
}