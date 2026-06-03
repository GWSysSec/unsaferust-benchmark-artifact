#![cfg(feature = "invocation")]

use std::sync::OnceLock;

use jni::objects::{GlobalRef, JObject, WeakRef};
use jni::{InitArgsBuilder, JNIVersion, JavaVM};

static JVM: OnceLock<JavaVM> = OnceLock::new();

fn java_vm() -> &'static JavaVM {
    JVM.get_or_init(|| {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .expect("JVM init args should build successfully");

        JavaVM::new(args).expect("a JVM should be created for WeakRef integration tests")
    })
}

#[test]
fn weak_ref_upgrades_and_identity_checks_track_live_object() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let java_string = env
        .new_string("weak-ref live object payload")
        .expect("Java string should be created");
    let local_obj = JObject::from(java_string);

    let weak = env
        .new_weak_ref(&local_obj)
        .expect("creating a weak reference should not fail")
        .expect("a live object should produce a WeakRef");

    assert!(
        !WeakRef::as_raw(&weak).is_null(),
        "WeakRef::as_raw should expose a non-null JNI weak reference for a live object"
    );

    assert!(
        !WeakRef::is_garbage_collected(&weak, &env).expect("GC status check should succeed"),
        "a weak reference should not be reported as garbage collected while a strong local reference is live"
    );

    assert!(
        WeakRef::is_same_object(&weak, &env, &local_obj).expect("weak/local identity check should succeed"),
        "the weak reference should identify the object it was created from"
    );

    assert!(
        !WeakRef::is_same_object(&weak, &env, JObject::null())
            .expect("weak/null identity check should succeed"),
        "a live weak reference should not compare equal to null"
    );

    let upgraded_local = WeakRef::upgrade_local(&weak, &env)
        .expect("upgrading weak reference to local reference should not fail")
        .expect("live weak reference should upgrade to a local reference");

    assert!(
        !upgraded_local.as_raw().is_null(),
        "WeakRef::upgrade_local should return a non-null local reference for a live object"
    );

    assert!(
        env.is_same_object(&upgraded_local, &local_obj)
            .expect("local/local identity check should succeed"),
        "the upgraded local reference should refer to the original object"
    );

    let upgraded_global = WeakRef::upgrade_global(&weak, &env)
        .expect("upgrading weak reference to global reference should not fail")
        .expect("live weak reference should upgrade to a global reference");

    let global_obj: &JObject<'static> = GlobalRef::as_obj(&upgraded_global);
    assert!(
        !global_obj.as_raw().is_null(),
        "WeakRef::upgrade_global should return a non-null global reference for a live object"
    );

    assert!(
        env.is_same_object(global_obj, &local_obj)
            .expect("global/local identity check should succeed"),
        "the upgraded global reference should refer to the original object"
    );
}

#[test]
fn weak_ref_clone_in_jvm_creates_equivalent_independent_weak_reference() {
    let vm = java_vm();
    let env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let first_string = env
        .new_string("first weak-ref target")
        .expect("first Java string should be created");
    let first_obj = JObject::from(first_string);

    let second_string = env
        .new_string("second weak-ref target")
        .expect("second Java string should be created");
    let second_obj = JObject::from(second_string);

    let first_weak = env
        .new_weak_ref(&first_obj)
        .expect("creating first weak reference should not fail")
        .expect("first live object should produce a WeakRef");

    let second_weak = env
        .new_weak_ref(&second_obj)
        .expect("creating second weak reference should not fail")
        .expect("second live object should produce a WeakRef");

    assert!(
        !WeakRef::is_weak_ref_to_same_object(&first_weak, &env, &second_weak)
            .expect("different weak-reference identity check should succeed"),
        "weak references created from different Java objects should not compare as the same object"
    );

    let cloned_in_jvm = WeakRef::clone_in_jvm(&first_weak, &env)
        .expect("JVM-level weak reference clone should not fail")
        .expect("live weak reference should be cloneable inside the JVM");

    assert!(
        !WeakRef::as_raw(&cloned_in_jvm).is_null(),
        "WeakRef::clone_in_jvm should return a WeakRef with a non-null raw JNI weak reference"
    );

    assert!(
        WeakRef::is_weak_ref_to_same_object(&first_weak, &env, &cloned_in_jvm)
            .expect("same-target weak-reference identity check should succeed"),
        "the JVM-level clone should refer to the same Java object as the original weak reference"
    );

    assert!(
        WeakRef::is_same_object(&cloned_in_jvm, &env, &first_obj)
            .expect("cloned weak/local identity check should succeed"),
        "the cloned weak reference should identify the original first object"
    );

    assert!(
        !WeakRef::is_same_object(&cloned_in_jvm, &env, &second_obj)
            .expect("cloned weak/different-local identity check should succeed"),
        "the cloned weak reference should not identify an unrelated object"
    );

    let upgraded_from_clone = WeakRef::upgrade_local(&cloned_in_jvm, &env)
        .expect("upgrading cloned weak reference to local reference should not fail")
        .expect("cloned live weak reference should upgrade to a local reference");

    assert!(
        env.is_same_object(&upgraded_from_clone, &first_obj)
            .expect("upgraded clone/local identity check should succeed"),
        "a local reference upgraded from the cloned weak reference should refer to the first object"
    );
}