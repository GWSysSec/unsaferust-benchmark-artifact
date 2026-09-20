#![cfg(feature = "invocation")]

use jni::objects::{JMethodID, JValue};
use jni::signature::{Primitive, ReturnType};
use jni::sys::{jmethodID, jvalue};
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

        JavaVM::new(args).expect("a JVM should be created for JMethodID integration tests")
    })
}

#[test]
fn jmethod_id_from_raw_round_trips_and_remains_usable_for_instance_calls() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let atomic_integer_class = env
        .find_class("java/util/concurrent/atomic/AtomicInteger")
        .expect("AtomicInteger class should be available");

    let counter = env
        .new_object(&atomic_integer_class, "(I)V", &[JValue::Int(10)])
        .expect("AtomicInteger instance should be constructed");

    let add_and_get_id = env
        .get_method_id(&atomic_integer_class, "addAndGet", "(I)I")
        .expect("addAndGet method ID should be found");

    let raw_add_and_get: jmethodID = jni::objects::JMethodID::into_raw(add_and_get_id);
    assert!(
        !raw_add_and_get.is_null(),
        "method ID obtained from the JVM must be non-null"
    );

    let wrapped_add_and_get = unsafe { jni::objects::JMethodID::from_raw(raw_add_and_get) };
    let raw_after_wrap = jni::objects::JMethodID::into_raw(wrapped_add_and_get);
    assert_eq!(
        raw_after_wrap, raw_add_and_get,
        "from_raw followed by into_raw should preserve the exact jmethodID pointer"
    );

    let callable_add_and_get = unsafe { JMethodID::from_raw(raw_after_wrap) };
    let added = unsafe {
        env.call_method_unchecked(
            &counter,
            callable_add_and_get,
            ReturnType::Primitive(Primitive::Int),
            &[jvalue { i: 9 }],
        )
        .expect("unchecked addAndGet call through rewrapped JMethodID should succeed")
    };
    assert_eq!(
        added.i().expect("addAndGet should return an int"),
        19,
        "addAndGet should add the argument to the existing value"
    );

    let get_id = env
        .get_method_id(&atomic_integer_class, "get", "()I")
        .expect("get method ID should be found");

    let raw_get: jmethodID = jni::objects::JMethodID::into_raw(get_id);
    assert!(
        !raw_get.is_null(),
        "second method ID obtained from the JVM must also be non-null"
    );
    assert_ne!(
        raw_get, raw_after_wrap,
        "different Java methods should have distinct raw method IDs"
    );

    let callable_get = unsafe { jni::objects::JMethodID::from_raw(raw_get) };
    let current = unsafe {
        env.call_method_unchecked(
            &counter,
            callable_get,
            ReturnType::Primitive(Primitive::Int),
            &[],
        )
        .expect("unchecked get call through JMethodID created with from_raw should succeed")
    };
    assert_eq!(
        current.i().expect("get should return an int"),
        19,
        "counter value observed through the rewrapped get method ID should reflect the prior call"
    );

    let normal_get = env
        .call_method(&counter, "get", "()I", &[])
        .expect("checked get call should also succeed");
    assert_eq!(
        normal_get.i().expect("checked get should return an int"),
        19,
        "checked and unchecked calls should observe the same object state"
    );
}