#![cfg(feature = "invocation")]

use jni::objects::{AutoElements, ReleaseMode};
use jni::sys::jint;
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

        JavaVM::new(args).expect("a JVM should be created for AutoElements integration tests")
    })
}

#[test]
fn auto_elements_exposes_length_pointer_and_commits_mutations() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let initial: [jint; 4] = [10, 20, 30, 40];
    let array = env
        .new_int_array(initial.len() as i32)
        .expect("int array should be allocated");

    env.set_int_array_region(&array, 0, &initial)
        .expect("initial values should be written");

    {
        let mut elements = unsafe {
            env.get_array_elements(&array, ReleaseMode::CopyBack)
                .expect("array elements should be accessible")
        };

        assert_eq!(
            AutoElements::len(&elements),
            initial.len(),
            "AutoElements should report the Java array length"
        );
        assert!(
            !AutoElements::is_empty(&elements),
            "non-empty Java array should not be reported as empty"
        );

        let ptr = AutoElements::as_ptr(&elements);
        assert!(!ptr.is_null(), "AutoElements pointer should be non-null");

        unsafe {
            let slice = std::slice::from_raw_parts_mut(ptr, AutoElements::len(&elements));
            assert_eq!(slice, initial);
            slice[1] = 200;
            slice[3] = 400;
        }

        AutoElements::commit(&mut elements).expect("committing copied elements should succeed");

        assert!(
            AutoElements::is_copy(&elements) || !AutoElements::is_copy(&elements),
            "is_copy should be callable and return a boolean"
        );
    }

    let mut after_commit = [0 as jint; 4];
    env.get_int_array_region(&array, 0, &mut after_commit)
        .expect("committed values should be readable from Java array");

    assert_eq!(after_commit, [10, 200, 30, 400]);
}

#[test]
fn auto_elements_discard_prevents_later_copyback_changes() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let original: [jint; 3] = [7, 8, 9];
    let array = env
        .new_int_array(original.len() as i32)
        .expect("int array should be allocated");

    env.set_int_array_region(&array, 0, &original)
        .expect("original values should be written");

    {
        let mut elements = unsafe {
            env.get_array_elements(&array, ReleaseMode::CopyBack)
                .expect("array elements should be accessible")
        };

        assert_eq!(AutoElements::len(&elements), 3);
        assert!(!AutoElements::is_empty(&elements));
        assert!(!AutoElements::as_ptr(&elements).is_null());

        unsafe {
            let slice = std::slice::from_raw_parts_mut(
                AutoElements::as_ptr(&elements),
                AutoElements::len(&elements),
            );
            slice[0] = 70;
            slice[1] = 80;
            slice[2] = 90;
        }

        AutoElements::discard(&mut elements);
    }

    let mut after_discard = [0 as jint; 3];
    env.get_int_array_region(&array, 0, &mut after_discard)
        .expect("array values should be readable after AutoElements drop");

    assert_eq!(
        after_discard, original,
        "discard should prevent modified copied elements from being copied back"
    );
}

#[test]
fn auto_elements_empty_array_reports_empty_and_has_zero_length() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let array = env
        .new_int_array(0)
        .expect("empty int array should be allocated");

    {
        let mut elements = unsafe {
            env.get_array_elements(&array, ReleaseMode::NoCopyBack)
                .expect("empty array elements should be accessible")
        };

        assert_eq!(AutoElements::len(&elements), 0);
        assert!(AutoElements::is_empty(&elements));

        AutoElements::discard(&mut elements);
    }

    let length = env
        .get_array_length(&array)
        .expect("empty Java array length should be readable");

    assert_eq!(length, 0);
}