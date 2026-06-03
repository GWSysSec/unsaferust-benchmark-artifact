#![cfg(feature = "invocation")]

use jni::objects::JStaticFieldID;
use jni::signature::{JavaType, Primitive};
use jni::sys::jfieldID;
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

        JavaVM::new(args).expect("a JVM should be created for JStaticFieldID integration tests")
    })
}

#[test]
fn jstatic_field_id_from_raw_round_trips_and_remains_usable_for_static_field_access() {
    let vm = java_vm();
    let mut env = vm
        .attach_current_thread()
        .expect("current test thread should attach to JVM");

    let integer_class = env
        .find_class("java/lang/Integer")
        .expect("java.lang.Integer should be available");

    let checked_value = env
        .get_static_field(&integer_class, "MAX_VALUE", "I")
        .expect("Integer.MAX_VALUE should be readable through checked static field access")
        .i()
        .expect("Integer.MAX_VALUE should be an int");

    assert_eq!(
        checked_value, i32::MAX,
        "checked static field access should read Java Integer.MAX_VALUE correctly"
    );

    let field_id = env
        .get_static_field_id(&integer_class, "MAX_VALUE", "I")
        .expect("static field ID for Integer.MAX_VALUE should be found");

    let raw: jfieldID = JStaticFieldID::into_raw(field_id);

    assert!(
        !raw.is_null(),
        "a real static field ID obtained from the JVM should be non-null"
    );

    let wrapped_again: JStaticFieldID = unsafe { JStaticFieldID::from_raw(raw) };
    let raw_after_first_wrap: jfieldID = JStaticFieldID::into_raw(wrapped_again);

    assert_eq!(
        raw_after_first_wrap, raw,
        "JStaticFieldID::from_raw followed by into_raw should preserve the exact raw ID"
    );
    assert!(
        !raw_after_first_wrap.is_null(),
        "round-tripping through JStaticFieldID should not turn a valid ID into null"
    );

    let wrapped_for_unchecked_access: JStaticFieldID =
        unsafe { JStaticFieldID::from_raw(raw_after_first_wrap) };

    let unchecked_value = unsafe {
        env.get_static_field_unchecked(
            &integer_class,
            wrapped_for_unchecked_access,
            JavaType::Primitive(Primitive::Int),
        )
    }
    .expect("wrapped static field ID should remain usable for unchecked static field access")
    .i()
    .expect("Integer.MAX_VALUE should still be returned as an int");

    assert_eq!(
        unchecked_value, checked_value,
        "unchecked access through the re-wrapped raw static field ID should match checked access"
    );
    assert_eq!(
        unchecked_value, i32::MAX,
        "unchecked access through JStaticFieldID::from_raw should read Integer.MAX_VALUE"
    );

    let wrapped_final: JStaticFieldID = unsafe { JStaticFieldID::from_raw(raw_after_first_wrap) };
    let raw_final: jfieldID = JStaticFieldID::into_raw(wrapped_final);

    assert_eq!(
        raw_final, raw,
        "repeated from_raw/into_raw cycles should continue to preserve the same raw static field ID"
    );
}