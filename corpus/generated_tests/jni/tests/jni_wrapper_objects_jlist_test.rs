#![cfg(feature = "invocation")]

use std::sync::{Arc, OnceLock};

use jni::objects::{JList, JListIter, JObject, JString};
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};

static JVM: OnceLock<Arc<JavaVM>> = OnceLock::new();

fn shared_jvm() -> Arc<JavaVM> {
    JVM.get_or_init(|| {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .expect("failed to build JVM init arguments");
        Arc::new(JavaVM::new(args).expect("failed to create JVM"))
    })
    .clone()
}

fn unwrap_jni<T>(result: jni::errors::Result<T>, env: &JNIEnv<'_>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
            }
            panic!("unexpected JNI error: {error:?}");
        }
    }
}

fn java_object_to_string(env: &mut JNIEnv<'_>, obj: JObject<'_>) -> String {
    assert!(!obj.as_raw().is_null(), "expected a non-null Java string");
    let java_string = JString::from(obj);
    let java_str = unwrap_jni(env.get_string(&java_string), env);
    String::from(java_str)
}

#[test]
fn jlist_supports_indexed_mutation_iteration_and_empty_edges() {
    let vm = shared_jvm();
    let mut env = vm
        .attach_current_thread()
        .expect("failed to attach current thread");

    let list_object = unwrap_jni(env.new_object("java/util/ArrayList", "()V", &[]), &env);
    let list = unwrap_jni(JList::from_env(&mut env, &list_object), &env);

    assert_eq!(unwrap_jni(JList::size(&list, &mut env), &env), 0);
    assert!(
        unwrap_jni(JList::get(&list, &mut env, 0), &env).is_none(),
        "getting from an empty list should return None"
    );
    assert!(
        unwrap_jni(JList::pop(&list, &mut env), &env).is_none(),
        "popping an empty list should return None"
    );

    let alpha = JObject::from(unwrap_jni(env.new_string("alpha"), &env));
    let gamma = JObject::from(unwrap_jni(env.new_string("gamma"), &env));
    let beta = JObject::from(unwrap_jni(env.new_string("beta"), &env));
    let delta = JObject::from(unwrap_jni(env.new_string("delta"), &env));

    unwrap_jni(JList::add(&list, &mut env, &alpha), &env);
    unwrap_jni(JList::add(&list, &mut env, &gamma), &env);
    assert_eq!(unwrap_jni(JList::size(&list, &mut env), &env), 2);

    unwrap_jni(JList::insert(&list, &mut env, 1, &beta), &env);
    unwrap_jni(JList::add(&list, &mut env, &delta), &env);
    assert_eq!(unwrap_jni(JList::size(&list, &mut env), &env), 4);

    let first = unwrap_jni(JList::get(&list, &mut env, 0), &env).expect("missing first item");
    assert_eq!(java_object_to_string(&mut env, first), "alpha");

    let second = unwrap_jni(JList::get(&list, &mut env, 1), &env).expect("missing second item");
    assert_eq!(java_object_to_string(&mut env, second), "beta");

    let removed = unwrap_jni(JList::remove(&list, &mut env, 2), &env).expect("missing removed item");
    assert_eq!(java_object_to_string(&mut env, removed), "gamma");
    assert_eq!(unwrap_jni(JList::size(&list, &mut env), &env), 3);

    let mut iter = unwrap_jni(JList::iter(&list, &mut env), &env);
    let mut iterated = Vec::new();
    while let Some(obj) = unwrap_jni(JListIter::next(&mut iter, &mut env), &env) {
        iterated.push(java_object_to_string(&mut env, obj));
    }

    assert_eq!(iterated, vec!["alpha", "beta", "delta"]);
    assert!(
        unwrap_jni(JListIter::next(&mut iter, &mut env), &env).is_none(),
        "iterator should keep returning None after exhaustion"
    );

    let popped = unwrap_jni(JList::pop(&list, &mut env), &env).expect("missing popped item");
    assert_eq!(java_object_to_string(&mut env, popped), "delta");
    assert_eq!(unwrap_jni(JList::size(&list, &mut env), &env), 2);

    let remaining_first =
        unwrap_jni(JList::get(&list, &mut env, 0), &env).expect("missing remaining first item");
    let remaining_second =
        unwrap_jni(JList::get(&list, &mut env, 1), &env).expect("missing remaining second item");

    assert_eq!(java_object_to_string(&mut env, remaining_first), "alpha");
    assert_eq!(java_object_to_string(&mut env, remaining_second), "beta");
    assert!(
        unwrap_jni(JList::get(&list, &mut env, 2), &env).is_none(),
        "index equal to list size should return None"
    );
}