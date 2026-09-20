#![cfg(feature = "invocation")]

use jni::objects::JStaticMethodID;
use jni::signature::{Primitive, ReturnType};
use jni::sys::jmethodID;
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

        JavaVM::new(args).expect("a JVM should be created for JStaticMethodID integration tests")
    })
}

#[test]
fn jstatic_method_id_from_raw_round_trips_and_remains_usable_for_static_calls() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let system_class = env
        .find_class("java/lang/System")
        .expect("java.lang.System should be available");

    let original_id = env
        .get_static_method_id(&system_class, "currentTimeMillis", "()J")
        .expect("System.currentTimeMillis static method ID should be found");

    let raw_id: jmethodID = JStaticMethodID::into_raw(original_id);
    assert!(
        !raw_id.is_null(),
        "a method ID obtained from the JVM should be non-null"
    );

    let round_tripped_once: JStaticMethodID = unsafe { JStaticMethodID::from_raw(raw_id) };
    let raw_id_again: jmethodID = JStaticMethodID::into_raw(round_tripped_once);
    assert_eq!(
        raw_id_again, raw_id,
        "JStaticMethodID::from_raw followed by into_raw should preserve the exact pointer"
    );
    assert!(
        !raw_id_again.is_null(),
        "round-tripping through JStaticMethodID should not turn a valid method ID into null"
    );

    let round_tripped_for_call: JStaticMethodID = unsafe { JStaticMethodID::from_raw(raw_id_again) };
    let first_call = unsafe {
        env.call_static_method_unchecked(
            &system_class,
            round_tripped_for_call,
            ReturnType::Primitive(Primitive::Long),
            &[],
        )
    }
    .expect("round-tripped System.currentTimeMillis method ID should be callable")
    .j()
    .expect("System.currentTimeMillis should return a Java long");

    assert!(
        first_call > 0,
        "System.currentTimeMillis should return a positive timestamp"
    );

    let fresh_id = env
        .get_static_method_id(&system_class, "nanoTime", "()J")
        .expect("System.nanoTime static method ID should be found");
    let fresh_raw: jmethodID = JStaticMethodID::into_raw(fresh_id);
    assert!(
        !fresh_raw.is_null(),
        "a second static method ID obtained from the JVM should also be non-null"
    );
    assert_ne!(
        fresh_raw, raw_id,
        "different static methods on the same class should have distinct method IDs"
    );

    let nano_id: JStaticMethodID = unsafe { JStaticMethodID::from_raw(fresh_raw) };
    let nano_value = unsafe {
        env.call_static_method_unchecked(
            &system_class,
            nano_id,
            ReturnType::Primitive(Primitive::Long),
            &[],
        )
    }
    .expect("round-tripped System.nanoTime method ID should be callable")
    .j()
    .expect("System.nanoTime should return a Java long");

    assert!(
        nano_value != 0,
        "System.nanoTime should return a non-zero monotonic time reading"
    );
}