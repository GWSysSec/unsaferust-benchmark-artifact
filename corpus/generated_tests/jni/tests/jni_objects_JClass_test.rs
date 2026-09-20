use jni::objects::JClass;
use jni::sys::jclass;
use std::ptr;

#[test]
fn jclass_null_raw_round_trip_is_preserved() {
    let raw: jclass = ptr::null_mut();

    let class = unsafe { JClass::from_raw(raw) };
    assert!(
        JClass::as_raw(&class).is_null(),
        "a JClass created from a null raw jclass should report a null raw pointer"
    );

    let unwrapped = JClass::into_raw(class);
    assert!(
        unwrapped.is_null(),
        "into_raw should preserve a null raw jclass exactly"
    );
    assert_eq!(
        unwrapped, raw,
        "null raw jclass should round-trip without changing pointer value"
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

            JavaVM::new(args).expect("a JVM should be created for JClass integration tests")
        })
    }

    #[test]
    fn jclass_raw_round_trip_preserves_live_class_reference_and_semantics() {
        let vm = java_vm();
        let mut env = vm
            .attach_current_thread()
            .expect("current test thread should attach to JVM");

        let string_class = env
            .find_class("java/lang/String")
            .expect("java.lang.String class should be found");
        let first_raw: jclass = JClass::as_raw(&string_class);

        assert!(
            !first_raw.is_null(),
            "find_class should return a non-null local class reference"
        );

        let unwrapped_raw: jclass = JClass::into_raw(string_class);
        assert_eq!(
            unwrapped_raw, first_raw,
            "as_raw and into_raw should expose the same underlying jclass"
        );

        let rewrapped_string_class = unsafe { JClass::from_raw(unwrapped_raw) };
        assert_eq!(
            JClass::as_raw(&rewrapped_string_class),
            first_raw,
            "from_raw should wrap the exact raw class reference it is given"
        );
        assert!(
            !JClass::as_raw(&rewrapped_string_class).is_null(),
            "rewrapped java.lang.String class reference should remain non-null"
        );

        let superclass = env
            .get_superclass(&rewrapped_string_class)
            .expect("getting superclass of java.lang.String should succeed")
            .expect("java.lang.String should have java.lang.Object as superclass");
        assert!(
            !JClass::as_raw(&superclass).is_null(),
            "superclass local reference should be non-null"
        );

        let object_class = env
            .find_class("java/lang/Object")
            .expect("java.lang.Object class should be found");
        assert!(
            env.is_same_object(&superclass, &object_class)
                .expect("class references should be comparable"),
            "java.lang.String superclass should be java.lang.Object"
        );

        assert!(
            env.is_assignable_from(&object_class, &rewrapped_string_class)
                .expect("assignability check Object <- String should succeed"),
            "java.lang.String should be assignable to java.lang.Object"
        );

        assert!(
            !env.is_assignable_from(&rewrapped_string_class, &object_class)
                .expect("assignability check String <- Object should succeed"),
            "java.lang.Object should not be assignable to java.lang.String"
        );

        let final_raw: jclass = JClass::into_raw(rewrapped_string_class);
        assert_eq!(
            final_raw, first_raw,
            "rewrapped JClass should unwrap back to the original raw class reference"
        );
    }
}