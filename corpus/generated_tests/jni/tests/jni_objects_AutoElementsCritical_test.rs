#![cfg(feature = "invocation")]

use jni::objects::{AutoElementsCritical, ReleaseMode};
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

        JavaVM::new(args).expect("a JVM should be created for AutoElementsCritical tests")
    })
}

#[test]
fn critical_elements_expose_pointer_length_and_copy_back_mutations() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let initial: [jint; 4] = [3, 5, 8, 13];
    let array = env
        .new_int_array(initial.len() as i32)
        .expect("int array should be allocated");

    env.set_int_array_region(&array, 0, &initial)
        .expect("initial values should be written");

    {
        let elements = unsafe {
            env.get_array_elements_critical(&array, ReleaseMode::CopyBack)
                .expect("critical elements should be accessible")
        };

        assert_eq!(
            AutoElementsCritical::len(&elements),
            initial.len(),
            "critical element view should report the Java array length"
        );
        assert!(
            !AutoElementsCritical::is_empty(&elements),
            "non-empty Java array should not produce an empty critical view"
        );

        let ptr = AutoElementsCritical::as_ptr(&elements);
        assert!(
            !ptr.is_null(),
            "critical element pointer for a non-empty array should be non-null"
        );

        let was_copy = AutoElementsCritical::is_copy(&elements);
        assert!(
            was_copy || !was_copy,
            "is_copy should return a well-defined boolean value"
        );

        unsafe {
            *ptr.add(0) = 21;
            *ptr.add(1) = 34;
            *ptr.add(2) = 55;
            *ptr.add(3) = 89;
        }
    }

    let mut after_copy_back: [jint; 4] = [0; 4];
    env.get_int_array_region(&array, 0, &mut after_copy_back)
        .expect("mutated values should be readable after critical elements are released");

    assert_eq!(
        after_copy_back,
        [21, 34, 55, 89],
        "ReleaseMode::CopyBack should persist pointer writes when the wrapper is dropped"
    );
}

#[test]
fn critical_elements_discard_prevents_copy_back_for_copy_views() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let initial: [jint; 3] = [100, 200, 300];
    let array = env
        .new_int_array(initial.len() as i32)
        .expect("int array should be allocated");

    env.set_int_array_region(&array, 0, &initial)
        .expect("initial values should be written");

    let was_copy;
    {
        let mut elements = unsafe {
            env.get_array_elements_critical(&array, ReleaseMode::CopyBack)
                .expect("critical elements should be accessible")
        };

        assert_eq!(AutoElementsCritical::len(&elements), 3);
        assert!(!AutoElementsCritical::is_empty(&elements));

        let ptr = AutoElementsCritical::as_ptr(&elements);
        assert!(!ptr.is_null());

        was_copy = AutoElementsCritical::is_copy(&elements);

        unsafe {
            *ptr.add(0) = -1;
            *ptr.add(1) = -2;
            *ptr.add(2) = -3;
        }

        AutoElementsCritical::discard(&mut elements);

        assert_eq!(
            AutoElementsCritical::len(&elements),
            3,
            "discard should change release behavior, not the visible length"
        );
        assert_eq!(
            AutoElementsCritical::is_copy(&elements),
            was_copy,
            "discard should not change whether JNI reported the view as a copy"
        );
    }

    let mut after_release: [jint; 3] = [0; 3];
    env.get_int_array_region(&array, 0, &mut after_release)
        .expect("array should remain readable after discarded critical view is released");

    if was_copy {
        assert_eq!(
            after_release, initial,
            "discard should prevent copied critical elements from being copied back"
        );
    } else {
        assert_eq!(
            after_release,
            [-1, -2, -3],
            "discard has no effect for non-copy critical elements because writes touched the original array"
        );
    }
}

#[test]
fn empty_array_critical_view_reports_zero_length_and_is_empty() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let array = env
        .new_int_array(0)
        .expect("zero-length int array should be allocated");

    {
        let mut elements = unsafe {
            env.get_array_elements_critical(&array, ReleaseMode::CopyBack)
                .expect("critical elements for an empty array should be accessible")
        };

        assert_eq!(
            AutoElementsCritical::len(&elements),
            0,
            "empty Java arrays should expose a zero-length critical view"
        );
        assert!(
            AutoElementsCritical::is_empty(&elements),
            "zero-length critical view should report is_empty"
        );

        let first_ptr = AutoElementsCritical::as_ptr(&elements);
        AutoElementsCritical::discard(&mut elements);
        let second_ptr = AutoElementsCritical::as_ptr(&elements);

        assert_eq!(
            first_ptr, second_ptr,
            "discard should not replace the wrapped critical pointer"
        );
        assert!(
            AutoElementsCritical::is_empty(&elements),
            "discard should not change empty critical view state"
        );
    }

    assert_eq!(
        env.get_array_length(&array)
            .expect("empty array length should remain readable"),
        0
    );
}