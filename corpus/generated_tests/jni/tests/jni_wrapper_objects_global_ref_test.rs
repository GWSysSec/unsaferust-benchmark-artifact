#![cfg(feature = "invocation")]

use std::sync::OnceLock;

use jni::objects::{GlobalRef, JObject};
use jni::{InitArgsBuilder, JNIVersion, JavaVM};

static JVM: OnceLock<JavaVM> = OnceLock::new();

fn java_vm() -> &'static JavaVM {
    JVM.get_or_init(|| {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .expect("JVM init args should build successfully");

        JavaVM::new(args).expect("a JVM should be created for GlobalRef integration tests")
    })
}

#[test]
fn global_ref_as_obj_exposes_stable_object_reference_for_jni_workflows() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let java_string = env
        .new_string("global-ref as_obj payload")
        .expect("Java string should be created");
    let local_obj = JObject::from(java_string);

    let global = env
        .new_global_ref(&local_obj)
        .expect("creating a GlobalRef from a local object should succeed");

    let global_obj = GlobalRef::as_obj(&global);
    assert!(
        !JObject::as_raw(global_obj).is_null(),
        "GlobalRef::as_obj should expose a non-null object for a live global reference"
    );

    assert!(
        env.is_same_object(global_obj, &local_obj)
            .expect("identity comparison between global and original local should succeed"),
        "the global reference should refer to the same Java object as the original local reference"
    );

    assert!(
        env.is_instance_of(global_obj, "java/lang/String")
            .expect("instance check against java.lang.String should succeed"),
        "the object exposed by GlobalRef::as_obj should remain usable as a Java string"
    );

    assert!(
        env.is_instance_of(global_obj, "java/lang/Object")
            .expect("instance check against java.lang.Object should succeed"),
        "the object exposed by GlobalRef::as_obj should also be a java.lang.Object"
    );

    let second_global = env
        .new_global_ref(global_obj)
        .expect("creating another GlobalRef from GlobalRef::as_obj should succeed");

    assert!(
        env.is_same_object(GlobalRef::as_obj(&global), GlobalRef::as_obj(&second_global))
            .expect("identity comparison between two global refs should succeed"),
        "a GlobalRef created from GlobalRef::as_obj should target the same Java object"
    );

    let other_string = env
        .new_string("global-ref as_obj payload")
        .expect("a distinct Java string with equal contents should be created");
    let other_obj = JObject::from(other_string);

    assert!(
        !env.is_same_object(GlobalRef::as_obj(&global), &other_obj)
            .expect("identity comparison with a distinct object should succeed"),
        "different Java string objects should not be identical even when their contents match"
    );

    env.delete_local_ref(local_obj)
        .expect("deleting the original local reference should succeed");

    assert!(
        env.is_instance_of(GlobalRef::as_obj(&global), "java/lang/String")
            .expect("global object should remain usable after deleting original local ref"),
        "GlobalRef::as_obj should remain valid after the source local reference is deleted"
    );

    assert!(
        !env.exception_check()
            .expect("exception_check should succeed after global reference operations"),
        "the successful GlobalRef::as_obj workflow should not leave a pending Java exception"
    );
}