#![cfg(feature = "invocation")]

use jni::objects::JValue;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};

fn new_test_vm() -> JavaVM {
    let args = InitArgsBuilder::new()
        .version(JNIVersion::V8)
        .option("-Xcheck:jni")
        .build()
        .expect("valid JVM init args");

    JavaVM::new(args).expect("JVM should be created")
}

#[test]
fn raw_vm_roundtrip_and_thread_attachment_workflow() {
    let vm = new_test_vm();

    unsafe {
        JavaVM::detach_current_thread(&vm);
    }

    assert_eq!(JavaVM::threads_attached(&vm), 0);
    assert!(
        JavaVM::get_env(&vm).is_err(),
        "newly normalized test thread should be detached"
    );

    let raw_vm = JavaVM::get_java_vm_pointer(&vm);
    assert!(!raw_vm.is_null(), "JavaVM raw pointer must be non-null");

    let vm_from_raw = unsafe { JavaVM::from_raw(raw_vm) }.expect("raw JavaVM pointer should wrap");
    assert_eq!(
        JavaVM::get_java_vm_pointer(&vm_from_raw),
        raw_vm,
        "wrapping from raw should preserve the exact VM pointer"
    );

    {
        let mut env = JavaVM::attach_current_thread_permanently(&vm_from_raw)
            .expect("permanent attach should succeed");
        assert!(
            !env.get_raw().is_null(),
            "attached JNIEnv pointer must be non-null"
        );
        assert!(
            JavaVM::get_env(&vm_from_raw).is_ok(),
            "get_env should succeed while the thread is attached"
        );
        assert_eq!(
            JavaVM::threads_attached(&vm_from_raw),
            1,
            "one thread should be attached after permanent attach"
        );

        let abs_value = env
            .call_static_method(
                "java/lang/Math",
                "abs",
                "(I)I",
                &[JValue::Int(-42)],
            )
            .expect("Math.abs call should succeed")
            .i()
            .expect("Math.abs should return int");
        assert_eq!(abs_value, 42);

        {
            let mut scoped_env = JavaVM::attach_current_thread(&vm_from_raw)
                .expect("scoped attach on already-attached thread should succeed");
            assert!(
                !scoped_env.get_raw().is_null(),
                "nested scoped JNIEnv pointer must be non-null"
            );
            assert_eq!(
                JavaVM::threads_attached(&vm_from_raw),
                1,
                "nested attach must not increase attached thread count"
            );

            let nested_abs_value = scoped_env
                .call_static_method(
                    "java/lang/Math",
                    "abs",
                    "(I)I",
                    &[JValue::Int(-7)],
                )
                .expect("nested Math.abs call should succeed")
                .i()
                .expect("Math.abs should return int");
            assert_eq!(nested_abs_value, 7);
        }

        assert!(
            JavaVM::get_env(&vm_from_raw).is_ok(),
            "dropping nested scoped attach must not detach permanent thread"
        );
        assert_eq!(
            JavaVM::threads_attached(&vm_from_raw),
            1,
            "permanent attachment should still be counted after nested guard drop"
        );
    }

    unsafe {
        JavaVM::detach_current_thread(&vm_from_raw);
    }

    assert_eq!(
        JavaVM::threads_attached(&vm_from_raw),
        0,
        "explicit detach should remove the permanent attachment"
    );
    assert!(
        JavaVM::get_env(&vm_from_raw).is_err(),
        "get_env should fail after explicit detach"
    );

    {
        let mut daemon_env = JavaVM::attach_current_thread_as_daemon(&vm_from_raw)
            .expect("daemon attach should succeed");
        assert!(
            !daemon_env.get_raw().is_null(),
            "daemon-attached JNIEnv pointer must be non-null"
        );
        assert_eq!(
            JavaVM::threads_attached(&vm_from_raw),
            1,
            "daemon attach should be counted diagnostically"
        );

        let daemon_abs_value = daemon_env
            .call_static_method(
                "java/lang/Math",
                "abs",
                "(I)I",
                &[JValue::Int(-99)],
            )
            .expect("daemon-attached Math.abs call should succeed")
            .i()
            .expect("Math.abs should return int");
        assert_eq!(daemon_abs_value, 99);
    }

    unsafe {
        JavaVM::detach_current_thread(&vm_from_raw);
    }

    assert_eq!(
        JavaVM::threads_attached(&vm_from_raw),
        0,
        "explicit detach should remove daemon attachment"
    );
    assert!(
        JavaVM::get_env(&vm_from_raw).is_err(),
        "daemon-detached thread should no longer have an environment"
    );
}

#[test]
#[ignore = "Destroying the process-wide JVM is only safe when run in isolation."]
fn destroy_vm_when_no_other_threads_are_attached() {
    let vm = new_test_vm();

    unsafe {
        JavaVM::detach_current_thread(&vm);
    }

    assert_eq!(JavaVM::threads_attached(&vm), 0);

    unsafe {
        JavaVM::destroy(&vm).expect("destroying an otherwise unused JVM should succeed");
    }
}