use jni::objects::JPrimitiveArray;
use jni::sys::{jarray, jbyte, jint};
use std::ptr;

#[test]
fn null_primitive_array_round_trips_for_byte_array_type() {
    let raw: jarray = ptr::null_mut();

    let array: JPrimitiveArray<'static, jbyte> =
        unsafe { jni::objects::JPrimitiveArray::from_raw(raw) };
    let unwrapped: jarray = jni::objects::JPrimitiveArray::into_raw(array);

    assert!(
        unwrapped.is_null(),
        "JPrimitiveArray::into_raw should preserve a null raw jarray"
    );
    assert_eq!(
        unwrapped, raw,
        "a null jarray should round-trip without pointer changes"
    );
    assert_eq!(
        unwrapped,
        ptr::null_mut(),
        "the platform null jarray value should remain exact"
    );
}

#[test]
fn null_primitive_array_can_be_wrapped_repeatedly_with_different_primitive_markers() {
    let first_raw: jarray = ptr::null_mut();

    let byte_array: JPrimitiveArray<'static, jbyte> =
        unsafe { jni::objects::JPrimitiveArray::from_raw(first_raw) };
    let second_raw: jarray = jni::objects::JPrimitiveArray::into_raw(byte_array);

    let int_array: JPrimitiveArray<'static, jint> =
        unsafe { jni::objects::JPrimitiveArray::from_raw(second_raw) };
    let final_raw: jarray = jni::objects::JPrimitiveArray::into_raw(int_array);

    assert!(
        second_raw.is_null(),
        "wrapping and unwrapping a null byte primitive array should produce null"
    );
    assert!(
        final_raw.is_null(),
        "wrapping and unwrapping the same null pointer as another primitive array type should produce null"
    );
    assert_eq!(
        final_raw, first_raw,
        "null primitive arrays should preserve the exact raw pointer across repeated conversions"
    );
    assert_eq!(
        final_raw, second_raw,
        "re-wrapping a null primitive array should not alter the raw pointer value"
    );
}

#[cfg(feature = "invocation")]
mod invocation_tests {
    use super::*;
    use jni::{InitArgsBuilder, JNIVersion, JavaVM};
    use std::sync::OnceLock;

    static JVM: OnceLock<JavaVM> = OnceLock::new();

    fn java_vm() -> &'static JavaVM {
        JVM.get_or_init(|| {
            let args = InitArgsBuilder::new()
                .version(JNIVersion::V8)
                .option("-Xcheck:jni")
                .build()
                .expect("JVM init args should build successfully");

            JavaVM::new(args).expect("a JVM should be created for primitive array tests")
        })
    }

    #[test]
    fn real_byte_array_raw_round_trip_remains_usable_as_primitive_array() {
        let vm = java_vm();
        let env = vm
            .attach_current_thread()
            .expect("current test thread should attach to JVM");

        let original_bytes = [3_u8, 1, 4, 1, 5, 9];
        let byte_array = env
            .byte_array_from_slice(&original_bytes)
            .expect("byte array should be created from Rust slice");

        let original_len = env
            .get_array_length(&byte_array)
            .expect("new byte array length should be readable");
        assert_eq!(
            original_len,
            original_bytes.len() as i32,
            "created byte array should have the same length as the source slice"
        );

        let raw: jarray = jni::objects::JPrimitiveArray::into_raw(byte_array);
        assert!(
            !raw.is_null(),
            "a real Java byte array should unwrap to a non-null raw jarray"
        );

        let primitive_array: JPrimitiveArray<'_, jbyte> =
            unsafe { jni::objects::JPrimitiveArray::from_raw(raw) };

        let wrapped_len = env
            .get_array_length(&primitive_array)
            .expect("re-wrapped primitive array length should be readable");
        assert_eq!(
            wrapped_len, original_len,
            "JPrimitiveArray::from_raw should preserve the usable Java array reference"
        );

        let mut observed = [0_i8; 6];
        env.get_byte_array_region(&primitive_array, 0, &mut observed)
            .expect("contents should be readable through the re-wrapped primitive array");

        let expected: Vec<i8> = original_bytes.iter().map(|byte| *byte as i8).collect();
        assert_eq!(
            observed.as_slice(),
            expected.as_slice(),
            "array contents should survive into_raw/from_raw round-trip"
        );

        let replacement = [8_i8, 6, 7, 5, 3, 0];
        env.set_byte_array_region(&primitive_array, 0, &replacement)
            .expect("contents should be writable through the re-wrapped primitive array");

        let mut reread = [0_i8; 6];
        env.get_byte_array_region(&primitive_array, 0, &mut reread)
            .expect("updated contents should be readable");
        assert_eq!(
            reread, replacement,
            "writes through the re-wrapped JPrimitiveArray should affect the same Java array"
        );

        let final_raw: jarray = jni::objects::JPrimitiveArray::into_raw(primitive_array);
        assert_eq!(
            final_raw, raw,
            "unwrapping the re-wrapped primitive array should return the same raw jarray"
        );
    }
}