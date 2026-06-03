#![cfg(feature = "invocation")]

use jni::objects::ReleaseMode;
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
fn auto_elements_copy_back_exposes_pointer_metadata_and_commits_mutations() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let initial: [jint; 5] = [1, 1, 2, 3, 5];
    let array = env
        .new_int_array(initial.len() as i32)
        .expect("int array should be allocated");

    env.set_int_array_region(&array, 0, &initial)
        .expect("initial array values should be written");

    {
        let mut elements = unsafe {
            env.get_array_elements(&array, ReleaseMode::CopyBack)
                .expect("array elements should be accessible")
        };

        assert_eq!(
            jni::objects::AutoElements::len(&elements),
            initial.len(),
            "AutoElements::len should match Java array length"
        );
        assert!(
            !jni::objects::AutoElements::is_empty(&elements),
            "non-empty Java array should not report empty"
        );

        let ptr = jni::objects::AutoElements::as_ptr(&elements);
        assert!(
            !ptr.is_null(),
            "AutoElements::as_ptr should expose a non-null element pointer"
        );

        let copy_flag_before = jni::objects::AutoElements::is_copy(&elements);
        assert_eq!(
            copy_flag_before,
            jni::objects::AutoElements::is_copy(&elements),
            "AutoElements::is_copy should be stable while the elements are held"
        );

        unsafe {
            *ptr.add(0) = 8;
            *ptr.add(1) = 13;
            *ptr.add(2) = 21;
            *ptr.add(3) = 34;
            *ptr.add(4) = 55;
        }

        jni::objects::AutoElements::commit(&mut elements)
            .expect("AutoElements::commit should copy mutations back successfully");
    }

    let mut committed = [0 as jint; 5];
    env.get_int_array_region(&array, 0, &mut committed)
        .expect("committed array values should be readable");

    assert_eq!(
        committed,
        [8, 13, 21, 34, 55],
        "committed pointer writes should be visible from the Java array"
    );
}

#[test]
fn auto_elements_discard_prevents_mutations_from_reaching_java_array() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let initial: [jint; 4] = [10, 20, 30, 40];
    let array = env
        .new_int_array(initial.len() as i32)
        .expect("int array should be allocated");

    env.set_int_array_region(&array, 0, &initial)
        .expect("initial array values should be written");

    {
        let mut elements = unsafe {
            env.get_array_elements(&array, ReleaseMode::CopyBack)
                .expect("array elements should be accessible")
        };

        assert_eq!(
            jni::objects::AutoElements::len(&elements),
            4,
            "AutoElements::len should expose the full primitive array length"
        );
        assert!(
            !jni::objects::AutoElements::is_empty(&elements),
            "AutoElements::is_empty should be false for a four-element array"
        );

        let ptr = jni::objects::AutoElements::as_ptr(&elements);
        assert!(
            !ptr.is_null(),
            "AutoElements::as_ptr should be non-null before discard"
        );

        unsafe {
            *ptr.add(0) = -1;
            *ptr.add(1) = -2;
            *ptr.add(2) = -3;
            *ptr.add(3) = -4;
        }

        jni::objects::AutoElements::discard(&mut elements);
    }

    let mut after_discard = [0 as jint; 4];
    env.get_int_array_region(&array, 0, &mut after_discard)
        .expect("array values should be readable after discard");

    assert_eq!(
        after_discard, initial,
        "discarded pointer writes should not be copied back to the Java array"
    );
}

#[test]
fn auto_elements_reports_empty_array_metadata() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let empty_array = env
        .new_int_array(0)
        .expect("zero-length int array should be allocated");

    {
        let elements = unsafe {
            env.get_array_elements(&empty_array, ReleaseMode::NoCopyBack)
                .expect("empty array elements should still be accessible")
        };

        assert_eq!(
            jni::objects::AutoElements::len(&elements),
            0,
            "AutoElements::len should be zero for an empty Java array"
        );
        assert!(
            jni::objects::AutoElements::is_empty(&elements),
            "AutoElements::is_empty should be true for an empty Java array"
        );

        let ptr = jni::objects::AutoElements::as_ptr(&elements);
        assert_eq!(
            ptr,
            jni::objects::AutoElements::as_ptr(&elements),
            "AutoElements::as_ptr should return a stable pointer value while held"
        );

        let copy_flag = jni::objects::AutoElements::is_copy(&elements);
        assert_eq!(
            copy_flag,
            jni::objects::AutoElements::is_copy(&elements),
            "AutoElements::is_copy should remain stable for empty arrays"
        );
    }

    assert_eq!(
        env.get_array_length(&empty_array)
            .expect("empty array length should be readable"),
        0,
        "Java array should remain zero length after AutoElements is dropped"
    );
}