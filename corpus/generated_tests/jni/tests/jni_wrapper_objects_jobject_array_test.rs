use jni::objects::JObjectArray;
use jni::sys::jobjectArray;
use std::ptr;

#[test]
fn null_jobject_array_round_trips_through_from_raw_and_into_raw() {
    let raw: jobjectArray = ptr::null_mut();

    let array: JObjectArray<'static> = unsafe { JObjectArray::from_raw(raw) };
    let round_tripped: jobjectArray = JObjectArray::into_raw(array);

    assert!(
        round_tripped.is_null(),
        "JObjectArray::from_raw(null) followed by into_raw should preserve null"
    );
    assert_eq!(
        round_tripped, raw,
        "JObjectArray::into_raw should return exactly the raw pointer supplied to from_raw"
    );
}

#[test]
fn non_null_jobject_array_pointer_identity_is_preserved_without_dereferencing() {
    let raw: jobjectArray = 0x1usize as jobjectArray;

    let array: JObjectArray<'static> = unsafe { JObjectArray::from_raw(raw) };
    let round_tripped: jobjectArray = JObjectArray::into_raw(array);

    assert!(
        !round_tripped.is_null(),
        "a non-null raw jobjectArray should remain non-null after wrapping"
    );
    assert_eq!(
        round_tripped, raw,
        "JObjectArray::from_raw and JObjectArray::into_raw should preserve pointer identity"
    );
}

#[test]
fn multiple_raw_array_wrappers_preserve_distinct_pointer_values() {
    let first_raw: jobjectArray = 0x1usize as jobjectArray;
    let second_raw: jobjectArray = 0x2usize as jobjectArray;

    let first_array: JObjectArray<'static> = unsafe { JObjectArray::from_raw(first_raw) };
    let second_array: JObjectArray<'static> = unsafe { JObjectArray::from_raw(second_raw) };

    let first_round_tripped: jobjectArray = JObjectArray::into_raw(first_array);
    let second_round_tripped: jobjectArray = JObjectArray::into_raw(second_array);

    assert_eq!(
        first_round_tripped, first_raw,
        "the first JObjectArray wrapper should return its own original raw pointer"
    );
    assert_eq!(
        second_round_tripped, second_raw,
        "the second JObjectArray wrapper should return its own original raw pointer"
    );
    assert_ne!(
        first_round_tripped, second_round_tripped,
        "distinct raw jobjectArray pointers should remain distinct after round-tripping"
    );
}

#[cfg(feature = "invocation")]
mod invocation_tests {
    use jni::objects::{JObject, JObjectArray, JString};
    use jni::sys::{jobjectArray, jsize};
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

            JavaVM::new(args).expect("a JVM should be created for JObjectArray integration tests")
        })
    }

    #[test]
    fn real_java_object_array_survives_raw_round_trip_and_remains_usable() {
        let vm = java_vm();
        let mut env = vm
            .attach_current_thread()
            .expect("current test thread should attach to JVM");

        let initial: JObject<'static> = JObject::null();
        let array: JObjectArray<'_> = env
            .new_object_array(3, "java/lang/String", initial)
            .expect("String[] should be created successfully");

        let original_len: jsize = env
            .get_array_length(&array)
            .expect("newly-created object array length should be readable");
        assert_eq!(original_len, 3, "created object array should have requested length");

        let raw: jobjectArray = JObjectArray::into_raw(array);
        assert!(
            !raw.is_null(),
            "a real JVM-created object array should expose a non-null raw jobjectArray"
        );

        let array: JObjectArray<'_> = unsafe { JObjectArray::from_raw(raw) };

        let len_after_round_trip: jsize = env
            .get_array_length(&array)
            .expect("object array should remain valid after from_raw/into_raw round-trip");
        assert_eq!(
            len_after_round_trip, original_len,
            "object array length should be unchanged after raw round-trip"
        );

        let hello: JString<'_> = env
            .new_string("hello from raw array")
            .expect("test string should be created");
        let hello_obj: JObject<'_> = JObject::from(hello);

        env.set_object_array_element(&array, 1, &hello_obj)
            .expect("setting an element through the round-tripped array should succeed");

        let fetched: JObject<'_> = env
            .get_object_array_element(&array, 1)
            .expect("getting an element through the round-tripped array should succeed");

        assert!(
            !JObject::as_raw(&fetched).is_null(),
            "the element set at index 1 should be a non-null Java string object"
        );
        assert!(
            env.is_same_object(&hello_obj, &fetched)
                .expect("same-object comparison should succeed"),
            "the fetched array element should be the exact Java object that was inserted"
        );
    }
}