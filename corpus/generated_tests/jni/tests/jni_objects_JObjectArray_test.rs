use jni::objects::{JObject, JObjectArray};
use jni::sys::jobjectArray;
use std::ptr;

#[test]
fn null_jobject_array_from_raw_round_trips_through_into_raw() {
    let raw: jobjectArray = ptr::null_mut();

    let array: JObjectArray<'static> = unsafe { jni::objects::JObjectArray::from_raw(raw) };
    let unwrapped: jobjectArray = jni::objects::JObjectArray::into_raw(array);

    assert!(
        unwrapped.is_null(),
        "JObjectArray::into_raw should preserve a null jobjectArray"
    );
    assert_eq!(
        unwrapped, raw,
        "a null jobjectArray should round-trip without pointer changes"
    );

    let array_again: JObjectArray<'static> =
        unsafe { jni::objects::JObjectArray::from_raw(unwrapped) };
    let unwrapped_again: jobjectArray = jni::objects::JObjectArray::into_raw(array_again);

    assert!(
        unwrapped_again.is_null(),
        "JObjectArray::from_raw should accept null repeatedly"
    );
    assert_eq!(
        unwrapped_again,
        ptr::null_mut(),
        "the platform null jobjectArray value should remain exact"
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

            JavaVM::new(args).expect("a JVM should be created for JObjectArray tests")
        })
    }

    #[test]
    fn jobject_array_raw_round_trip_preserves_usable_local_reference() {
        let vm = java_vm();
        let mut env = vm
            .attach_current_thread()
            .expect("current test thread should attach to the JVM");

        let string_class = env
            .find_class("java/lang/String")
            .expect("java.lang.String should be available");

        let array = env
            .new_object_array(3, &string_class, JObject::null())
            .expect("String[] should be created with null initial elements");

        assert_eq!(
            env.get_array_length(&array)
                .expect("new object array length should be readable"),
            3,
            "new_object_array should create the requested number of elements"
        );

        let raw: jobjectArray = jni::objects::JObjectArray::into_raw(array);
        assert!(
            !raw.is_null(),
            "a real Java object array local reference should not unwrap to null"
        );

        let array = unsafe { jni::objects::JObjectArray::from_raw(raw) };

        assert_eq!(
            env.get_array_length(&array)
                .expect("array should remain valid after from_raw"),
            3,
            "from_raw should wrap the same usable array reference"
        );

        let initial_first = env
            .get_object_array_element(&array, 0)
            .expect("first element should be readable");
        assert!(
            env.is_same_object(&initial_first, JObject::null())
                .expect("null comparison should succeed"),
            "initial array elements should be null"
        );

        let alpha = env
            .new_string("alpha")
            .expect("first Java string should be created");
        let beta = env
            .new_string("beta")
            .expect("second Java string should be created");

        env.set_object_array_element(&array, 0, &alpha)
            .expect("first array slot should accept alpha");
        env.set_object_array_element(&array, 2, &beta)
            .expect("last array slot should accept beta");

        let first = env
            .get_object_array_element(&array, 0)
            .expect("first populated element should be readable");
        let middle = env
            .get_object_array_element(&array, 1)
            .expect("middle element should be readable");
        let last = env
            .get_object_array_element(&array, 2)
            .expect("last populated element should be readable");

        assert!(
            env.is_same_object(&first, &alpha)
                .expect("first element should be comparable with alpha"),
            "slot 0 should contain the alpha string reference"
        );
        assert!(
            env.is_same_object(&middle, JObject::null())
                .expect("middle element should be comparable with null"),
            "untouched slot 1 should still be null"
        );
        assert!(
            env.is_same_object(&last, &beta)
                .expect("last element should be comparable with beta"),
            "slot 2 should contain the beta string reference"
        );
        assert!(
            !env.is_same_object(&first, &last)
                .expect("different string objects should be comparable"),
            "distinct elements should refer to distinct Java String objects"
        );

        let raw_again: jobjectArray = jni::objects::JObjectArray::into_raw(array);
        assert_eq!(
            raw_again, raw,
            "into_raw after re-wrapping should return the exact same local reference"
        );

        let array_again = unsafe { jni::objects::JObjectArray::from_raw(raw_again) };
        assert_eq!(
            env.get_array_length(&array_again)
                .expect("array should remain readable after a second raw round-trip"),
            3,
            "array length should remain stable after repeated raw wrapping"
        );
    }
}