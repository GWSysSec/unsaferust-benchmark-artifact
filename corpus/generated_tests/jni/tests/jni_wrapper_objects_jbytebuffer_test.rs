#![cfg(feature = "invocation")]

use jni::objects::JByteBuffer;
use jni::sys::jobject;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use std::ptr;
use std::slice;
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
fn direct_byte_buffer_round_trips_through_raw_and_remains_usable() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let mut backing = vec![10_u8, 20, 30, 40, 50, 60];
    let original_ptr = backing.as_mut_ptr();
    let original_len = backing.len();

    let direct_buffer: JByteBuffer<'_> = unsafe {
        env.new_direct_byte_buffer(original_ptr, original_len)
            .expect("direct byte buffer should be created from valid Rust memory")
    };

    let raw: jobject = JByteBuffer::into_raw(direct_buffer);
    assert!(
        !raw.is_null(),
        "JByteBuffer::into_raw should return a non-null jobject for a valid direct buffer"
    );

    let rewrapped: JByteBuffer<'_> = unsafe { JByteBuffer::from_raw(raw) };

    let capacity = env
        .get_direct_buffer_capacity(&rewrapped)
        .expect("rewrapped direct byte buffer should still expose its capacity");
    assert_eq!(
        capacity, original_len,
        "raw round-trip should preserve direct buffer capacity"
    );

    let address = env
        .get_direct_buffer_address(&rewrapped)
        .expect("rewrapped direct byte buffer should still expose its native address");
    assert_eq!(
        address, original_ptr,
        "raw round-trip should preserve the direct buffer native address"
    );

    unsafe {
        let bytes = slice::from_raw_parts_mut(address, capacity);
        assert_eq!(
            bytes,
            &[10, 20, 30, 40, 50, 60],
            "direct buffer address should point at the original backing bytes"
        );

        bytes[1] = 77;
        bytes[4] = 99;
    }

    assert_eq!(
        backing,
        vec![10, 77, 30, 40, 99, 60],
        "mutating through the rewrapped JByteBuffer address should update the original backing storage"
    );

    let raw_again: jobject = JByteBuffer::into_raw(rewrapped);
    assert_eq!(
        raw_again, raw,
        "JByteBuffer::from_raw followed by JByteBuffer::into_raw should preserve the exact jobject"
    );
}

#[test]
fn null_jbytebuffer_raw_pointer_round_trips_without_change() {
    let raw_null: jobject = ptr::null_mut();

    let buffer: JByteBuffer<'static> = unsafe { JByteBuffer::from_raw(raw_null) };
    let round_tripped: jobject = JByteBuffer::into_raw(buffer);

    assert!(
        round_tripped.is_null(),
        "JByteBuffer::from_raw(null) should produce a wrapper that unwraps back to null"
    );
    assert_eq!(
        round_tripped, raw_null,
        "JByteBuffer raw null pointer should round-trip exactly"
    );
    assert_eq!(
        round_tripped,
        ptr::null_mut(),
        "round-tripped null JByteBuffer pointer should equal the platform null jobject"
    );
}