#![cfg(feature = "invocation")]

use std::sync::OnceLock;

use jni::objects::{AutoLocal, JObject};
use jni::{InitArgsBuilder, JNIVersion, JavaVM};

static JVM: OnceLock<JavaVM> = OnceLock::new();

fn java_vm() -> &'static JavaVM {
    JVM.get_or_init(|| {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .expect("JVM init args should build");

        JavaVM::new(args).expect("JVM should be created")
    })
}

#[test]
fn auto_local_new_wraps_reference_and_forget_returns_original_local_object() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current thread should attach to JVM");

    let local_string = env
        .new_string("auto-local round trip payload")
        .expect("Java string should be created");
    let local_object = JObject::from(local_string);
    let original_raw = local_object.as_raw();

    assert!(
        !original_raw.is_null(),
        "newly-created Java string local reference should be non-null"
    );

    let wrapped = AutoLocal::new(local_object, &env);
    let forgotten = AutoLocal::forget(wrapped);

    assert_eq!(
        forgotten.as_raw(),
        original_raw,
        "forget should return the same JNI local reference that was wrapped"
    );
    assert!(
        !forgotten.as_raw().is_null(),
        "forgotten object should still be a live non-null local reference"
    );

    let string_class = env
        .find_class("java/lang/String")
        .expect("java.lang.String class should be found");
    let object_class = env
        .find_class("java/lang/Object")
        .expect("java.lang.Object class should be found");

    assert!(
        env.is_instance_of(&forgotten, string_class)
            .expect("is_instance_of should work for forgotten AutoLocal object"),
        "forgotten object should still be a java.lang.String"
    );
    assert!(
        env.is_instance_of(&forgotten, object_class)
            .expect("is_instance_of should work for Object superclass"),
        "forgotten string should also be an instance of java.lang.Object"
    );
    assert!(
        env.is_same_object(&forgotten, &forgotten)
            .expect("is_same_object should compare the forgotten reference with itself"),
        "a forgotten local reference should compare identical to itself"
    );

    env.delete_local_ref(forgotten)
        .expect("manually deleting forgotten local reference should succeed");
}

#[test]
fn auto_local_forget_preserves_reference_identity_across_multiple_wrapped_objects() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current thread should attach to JVM");

    let first = JObject::from(
        env.new_string("same text")
            .expect("first Java string should be created"),
    );
    let second = JObject::from(
        env.new_string("same text")
            .expect("second Java string should be created"),
    );

    let first_raw = first.as_raw();
    let second_raw = second.as_raw();

    assert!(
        !first_raw.is_null() && !second_raw.is_null(),
        "both Java string local references should be non-null"
    );
    assert_ne!(
        first_raw, second_raw,
        "separately-created strings should have distinct local reference handles"
    );

    let first_wrapped = AutoLocal::new(first, &env);
    let second_wrapped = AutoLocal::new(second, &env);

    let first_forgotten = AutoLocal::forget(first_wrapped);
    let second_forgotten = AutoLocal::forget(second_wrapped);

    assert_eq!(
        first_forgotten.as_raw(),
        first_raw,
        "forget should preserve the first wrapped reference handle"
    );
    assert_eq!(
        second_forgotten.as_raw(),
        second_raw,
        "forget should preserve the second wrapped reference handle"
    );
    assert!(
        !env.is_same_object(&first_forgotten, &second_forgotten)
            .expect("is_same_object should compare two distinct Java string objects"),
        "distinct Java string objects with equal contents should not be the same reference"
    );

    let first_class = env
        .get_object_class(&first_forgotten)
        .expect("object class of first forgotten reference should be available");
    let second_class = env
        .get_object_class(&second_forgotten)
        .expect("object class of second forgotten reference should be available");

    assert!(
        env.is_assignable_from(first_class, second_class)
            .expect("String class should be assignable from itself"),
        "both forgotten references should have compatible runtime classes"
    );

    env.delete_local_ref(first_forgotten)
        .expect("manually deleting first forgotten reference should succeed");
    env.delete_local_ref(second_forgotten)
        .expect("manually deleting second forgotten reference should succeed");
}

#[test]
fn auto_local_can_wrap_and_forget_null_object_without_deleting_any_reference() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current thread should attach to JVM");

    let null_object = JObject::null();

    assert!(
        null_object.as_raw().is_null(),
        "JObject::null should start as a null JNI reference"
    );

    let wrapped_null = AutoLocal::new(null_object, &env);
    let forgotten_null = AutoLocal::forget(wrapped_null);

    assert!(
        forgotten_null.as_raw().is_null(),
        "forget should preserve a null object as null"
    );
    assert!(
        env.is_same_object(&forgotten_null, JObject::null())
            .expect("is_same_object should support null references"),
        "forgotten null object should compare as the same object as another null reference"
    );
}