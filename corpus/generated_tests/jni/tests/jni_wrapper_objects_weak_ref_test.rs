#![cfg(feature = "invocation")]

use std::sync::OnceLock;

use jni::objects::{JObject, WeakRef};
use jni::{InitArgsBuilder, JNIVersion, JavaVM};

static JVM: OnceLock<JavaVM> = OnceLock::new();

fn java_vm() -> &'static JavaVM {
    JVM.get_or_init(|| {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .expect("JVM init args should build successfully");

        JavaVM::new(args).expect("JVM should be created for WeakRef integration tests")
    })
}

#[test]
fn weak_ref_live_object_upgrades_identity_and_clone_workflow() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let java_string = env
        .new_string("weak-ref public API live object")
        .expect("test Java string should be created");
    let local_obj = JObject::from(java_string);

    let weak = env
        .new_weak_ref(&local_obj)
        .expect("creating weak ref should succeed")
        .expect("live non-null object should produce a WeakRef");

    assert!(
        !WeakRef::as_raw(&weak).is_null(),
        "WeakRef::as_raw should expose a non-null JNI weak global reference"
    );

    assert!(
        !WeakRef::is_garbage_collected(&weak, &env)
            .expect("is_garbage_collected should succeed for a live referent"),
        "weak referent should not be reported as garbage collected while a strong local exists"
    );

    assert!(
        WeakRef::is_same_object(&weak, &env, &local_obj)
            .expect("is_same_object should succeed for its original referent"),
        "weak reference should identify the same object used to create it"
    );

    let upgraded_local = WeakRef::upgrade_local(&weak, &env)
        .expect("upgrade_local should not fail for a live referent")
        .expect("live weak referent should upgrade to a local reference");

    assert!(
        env.is_same_object(&local_obj, &upgraded_local)
            .expect("JNIEnv::is_same_object should compare original and upgraded local"),
        "upgraded local reference should point to the original Java object"
    );

    let upgraded_global = WeakRef::upgrade_global(&weak, &env)
        .expect("upgrade_global should not fail for a live referent")
        .expect("live weak referent should upgrade to a global reference");

    assert!(
        env.is_same_object(&local_obj, upgraded_global.as_obj())
            .expect("JNIEnv::is_same_object should compare original and upgraded global"),
        "upgraded global reference should point to the original Java object"
    );

    let cloned_weak = WeakRef::clone_in_jvm(&weak, &env)
        .expect("clone_in_jvm should not fail for a live weak reference")
        .expect("live weak reference should clone to another WeakRef");

    assert!(
        !WeakRef::as_raw(&cloned_weak).is_null(),
        "cloned WeakRef should also have a non-null raw weak reference"
    );

    assert!(
        WeakRef::is_weak_ref_to_same_object(&weak, &env, &cloned_weak)
            .expect("is_weak_ref_to_same_object should compare two live weak refs"),
        "cloned weak reference should refer to the same Java object"
    );

    assert!(
        WeakRef::is_same_object(&cloned_weak, &env, &local_obj)
            .expect("cloned weak ref identity check should succeed"),
        "cloned weak reference should identify the original object"
    );
}

#[test]
fn weak_refs_to_distinct_objects_are_not_confused() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let first_string = env
        .new_string("first weak-ref referent")
        .expect("first Java string should be created");
    let second_string = env
        .new_string("second weak-ref referent")
        .expect("second Java string should be created");

    let first_obj = JObject::from(first_string);
    let second_obj = JObject::from(second_string);

    let first_weak = env
        .new_weak_ref(&first_obj)
        .expect("creating first weak ref should succeed")
        .expect("first live object should produce a WeakRef");
    let second_weak = env
        .new_weak_ref(&second_obj)
        .expect("creating second weak ref should succeed")
        .expect("second live object should produce a WeakRef");

    assert!(
        !WeakRef::as_raw(&first_weak).is_null(),
        "first weak reference raw handle should be non-null"
    );
    assert!(
        !WeakRef::as_raw(&second_weak).is_null(),
        "second weak reference raw handle should be non-null"
    );

    assert!(
        !env.is_same_object(&first_obj, &second_obj)
            .expect("two separately-created strings should be comparable"),
        "distinct Java string instances should not be the same object"
    );

    assert!(
        WeakRef::is_same_object(&first_weak, &env, &first_obj)
            .expect("first weak ref should compare with first object"),
        "first weak ref should match its own referent"
    );
    assert!(
        WeakRef::is_same_object(&second_weak, &env, &second_obj)
            .expect("second weak ref should compare with second object"),
        "second weak ref should match its own referent"
    );

    assert!(
        !WeakRef::is_same_object(&first_weak, &env, &second_obj)
            .expect("first weak ref should compare with second object"),
        "first weak ref should not match the second referent"
    );
    assert!(
        !WeakRef::is_same_object(&second_weak, &env, &first_obj)
            .expect("second weak ref should compare with first object"),
        "second weak ref should not match the first referent"
    );

    assert!(
        !WeakRef::is_weak_ref_to_same_object(&first_weak, &env, &second_weak)
            .expect("distinct weak refs should be comparable"),
        "weak refs created from distinct Java objects should not be treated as the same referent"
    );

    let first_clone = WeakRef::clone_in_jvm(&first_weak, &env)
        .expect("cloning first weak ref should succeed")
        .expect("first live weak ref should clone");

    assert!(
        WeakRef::is_weak_ref_to_same_object(&first_weak, &env, &first_clone)
            .expect("original and clone should be comparable"),
        "cloned weak ref should preserve identity with original weak ref"
    );
    assert!(
        !WeakRef::is_weak_ref_to_same_object(&first_clone, &env, &second_weak)
            .expect("clone and unrelated weak ref should be comparable"),
        "cloned first weak ref should remain distinct from second weak ref"
    );

    let first_upgraded = WeakRef::upgrade_local(&first_clone, &env)
        .expect("upgrading cloned first weak ref should succeed")
        .expect("cloned first weak ref should still point to a live object");

    assert!(
        env.is_same_object(&first_obj, &first_upgraded)
            .expect("upgraded clone should compare with original first object"),
        "upgraded clone should point to the first object"
    );
}