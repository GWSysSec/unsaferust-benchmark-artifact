#![cfg(feature = "invocation")]

use jni::objects::{JFieldID, JValue};
use jni::signature::{Primitive, ReturnType};
use jni::sys::jfieldID;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use std::ptr;
use std::sync::OnceLock;

static JVM: OnceLock<JavaVM> = OnceLock::new();

fn java_vm() -> &'static JavaVM {
    JVM.get_or_init(|| {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .expect("JVM init args should build successfully");

        JavaVM::new(args).expect("a JVM should be created for JFieldID integration tests")
    })
}

#[test]
fn jfield_id_from_raw_round_trips_and_remains_usable_for_field_access() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let atomic_integer_class = env
        .find_class("java/util/concurrent/atomic/AtomicInteger")
        .expect("AtomicInteger class should be available");

    let first = env
        .new_object(
            &atomic_integer_class,
            "(I)V",
            &[JValue::Int(321)],
        )
        .expect("first AtomicInteger should be constructed");

    let second = env
        .new_object(
            &atomic_integer_class,
            "(I)V",
            &[JValue::Int(-17)],
        )
        .expect("second AtomicInteger should be constructed");

    let field_id = env
        .get_field_id(&atomic_integer_class, "value", "I")
        .expect("AtomicInteger.value field ID should be found");

    let raw: jfieldID = JFieldID::into_raw(field_id);
    assert_ne!(
        raw,
        ptr::null_mut(),
        "a field ID obtained from the JVM must be a non-null raw jfieldID"
    );

    let round_tripped_once = unsafe { JFieldID::from_raw(raw) };
    let raw_after_first_round_trip = JFieldID::into_raw(round_tripped_once);
    assert_eq!(
        raw_after_first_round_trip, raw,
        "JFieldID::from_raw followed by JFieldID::into_raw should preserve the exact raw field ID"
    );

    let first_value = unsafe {
        env.get_field_unchecked(
            &first,
            JFieldID::from_raw(raw_after_first_round_trip),
            ReturnType::Primitive(Primitive::Int),
        )
        .expect("field access through a rewrapped JFieldID should succeed")
    }
    .i()
    .expect("AtomicInteger.value should be returned as an int");

    assert_eq!(
        first_value, 321,
        "the rewrapped raw field ID should read the expected value from the first object"
    );

    let second_value = unsafe {
        env.get_field_unchecked(
            &second,
            JFieldID::from_raw(raw),
            ReturnType::Primitive(Primitive::Int),
        )
        .expect("the same raw field ID should be reusable for another object of the same class")
    }
    .i()
    .expect("AtomicInteger.value should be returned as an int");

    assert_eq!(
        second_value, -17,
        "the same rewrapped field ID should read the expected value from a second object"
    );

    let round_tripped_twice = unsafe { JFieldID::from_raw(raw) };
    let raw_after_second_round_trip = JFieldID::into_raw(round_tripped_twice);
    assert_eq!(
        raw_after_second_round_trip, raw,
        "repeated from_raw/into_raw conversions should continue to preserve pointer identity"
    );

    assert_eq!(
        raw_after_second_round_trip, raw_after_first_round_trip,
        "all round-tripped raw field IDs should refer to the same JVM field"
    );
}