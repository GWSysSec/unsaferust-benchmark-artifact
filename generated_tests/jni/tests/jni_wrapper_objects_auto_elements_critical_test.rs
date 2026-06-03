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

        JavaVM::new(args).expect("a JVM should be created for AutoElementsCritical integration tests")
    })
}

#[test]
fn critical_elements_expose_pointer_metadata_and_copy_back_pointer_mutations() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let initial: [jint; 5] = [1, 2, 3, 4, 5];
    let array = env
        .new_int_array(initial.len() as i32)
        .expect("int array should be allocated");

    env.set_int_array_region(&array, 0, &initial)
        .expect("initial values should be written");

    {
        let elements = unsafe {
            env.get_array_elements_critical(&array, ReleaseMode::CopyBack)
                .expect("critical array elements should be accessible")
        };

        let len = AutoElementsCritical::len(&elements);
        let is_empty = AutoElementsCritical::is_empty(&elements);
        let ptr = AutoElementsCritical::as_ptr(&elements);
        let first_copy_observation = AutoElementsCritical::is_copy(&elements);
        let second_copy_observation = AutoElementsCritical::is_copy(&elements);

        assert_eq!(len, initial.len());
        assert!(!is_empty);
        assert!(!ptr.is_null());
        assert_eq!(first_copy_observation, second_copy_observation);

        unsafe {
            for idx in 0..len {
                *ptr.add(idx) *= 10;
            }
        }
    }

    let mut actual = [0 as jint; 5];
    env.get_int_array_region(&array, 0, &mut actual)
        .expect("mutated values should be readable after critical elements are released");

    assert_eq!(actual, [10, 20, 30, 40, 50]);
}

#[test]
fn critical_elements_discard_prevents_mutations_from_being_copied_back() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let initial: [jint; 4] = [7, 11, 13, 17];
    let array = env
        .new_int_array(initial.len() as i32)
        .expect("int array should be allocated");

    env.set_int_array_region(&array, 0, &initial)
        .expect("initial values should be written");

    {
        let mut elements = unsafe {
            env.get_array_elements_critical(&array, ReleaseMode::CopyBack)
                .expect("critical array elements should be accessible")
        };

        assert_eq!(AutoElementsCritical::len(&elements), initial.len());
        assert!(!AutoElementsCritical::is_empty(&elements));

        let ptr = AutoElementsCritical::as_ptr(&elements);
        assert!(!ptr.is_null());

        unsafe {
            *ptr.add(0) = 700;
            *ptr.add(1) = 1100;
            *ptr.add(2) = 1300;
            *ptr.add(3) = 1700;
        }

        AutoElementsCritical::discard(&mut elements);
    }

    let mut actual = [0 as jint; 4];
    env.get_int_array_region(&array, 0, &mut actual)
        .expect("values should be readable after discarded critical elements are released");

    assert_eq!(actual, initial);
}

#[test]
fn critical_elements_report_empty_metadata_for_zero_length_arrays() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let array = env
        .new_int_array(0)
        .expect("zero-length int array should be allocated");

    {
        let mut elements = unsafe {
            env.get_array_elements_critical(&array, ReleaseMode::NoCopyBack)
                .expect("zero-length critical array elements should be accessible")
        };

        assert_eq!(AutoElementsCritical::len(&elements), 0);
        assert!(AutoElementsCritical::is_empty(&elements));

        let ptr = AutoElementsCritical::as_ptr(&elements);
        assert_eq!(ptr, AutoElementsCritical::as_ptr(&elements));

        let copy_flag = AutoElementsCritical::is_copy(&elements);
        assert_eq!(copy_flag, AutoElementsCritical::is_copy(&elements));

        AutoElementsCritical::discard(&mut elements);
    }

    let length = env
        .get_array_length(&array)
        .expect("array length should remain readable after discard");

    assert_eq!(length, 0);
}