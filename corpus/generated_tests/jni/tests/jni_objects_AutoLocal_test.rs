#![cfg(feature = "invocation")]

use std::ptr;
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
            .expect("JVM init args should build successfully");

        JavaVM::new(args).expect("a JVM should be created for AutoLocal integration tests")
    })
}

#[test]
fn auto_local_wraps_local_ref_and_forget_returns_the_same_object() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let string = env
        .new_string("AutoLocal keeps this local reference alive while wrapped")
        .expect("Java string should be created");
    let obj = JObject::from(string);
    let original_raw = obj.as_raw();

    assert!(
        !original_raw.is_null(),
        "new_string must create a non-null local reference"
    );

    let auto = jni::objects::AutoLocal::new(obj, &env);

    assert_eq!(
        auto.as_raw(),
        original_raw,
        "AutoLocal should dereference to the exact wrapped local reference"
    );

    let class = env
        .get_object_class(&*auto)
        .expect("wrapped object should still be usable through Deref");
    assert!(
        !class.as_raw().is_null(),
        "wrapped object should have a non-null runtime class"
    );

    assert!(
        env.is_same_object(&*auto, &*auto)
            .expect("JNI should compare the wrapped object with itself"),
        "wrapped object must be the same JNI object as itself"
    );

    let forgotten = jni::objects::AutoLocal::forget(auto);

    assert_eq!(
        forgotten.as_raw(),
        original_raw,
        "forget should return the original object without replacing the reference"
    );

    assert!(
        env.is_same_object(&forgotten, &forgotten)
            .expect("forgotten object should remain a valid local reference"),
        "forgotten object should remain usable after AutoLocal is consumed"
    );

    env.delete_local_ref(forgotten)
        .expect("forgotten local reference should be deleted manually");
}

#[test]
fn auto_local_forget_preserves_null_reference_without_deleting_it() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let null_obj = JObject::null();
    assert!(
        null_obj.as_raw().is_null(),
        "JObject::null should start as a null JNI reference"
    );

    let auto_null = AutoLocal::new(null_obj, &env);

    assert!(
        auto_null.as_raw().is_null(),
        "AutoLocal should preserve a wrapped null reference"
    );

    assert!(
        env.is_same_object(&*auto_null, JObject::null())
            .expect("JNI should be able to compare null references"),
        "wrapped null reference should compare equal to another null reference"
    );

    let forgotten_null = AutoLocal::forget(auto_null);

    assert_eq!(
        forgotten_null.as_raw(),
        ptr::null_mut(),
        "forget should return the same null raw reference"
    );

    assert!(
        env.is_same_object(&forgotten_null, JObject::null())
            .expect("forgotten null reference should still compare as null"),
        "forgotten null reference should remain semantically null"
    );
}