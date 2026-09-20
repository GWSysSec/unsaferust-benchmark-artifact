#![cfg(feature = "invocation")]

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use jni::objects::{JMap, JMapIter, JObject, JString};
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

fn make_java_string_object<'local>(env: &JNIEnv<'local>, text: &str) -> JObject<'local> {
    JObject::from(unwrap_jni(env.new_string(text), env))
}

fn java_object_to_string(env: &mut JNIEnv<'_>, obj: JObject<'_>) -> String {
    assert!(!obj.as_raw().is_null(), "expected a non-null Java string");
    let java_string = JString::from(obj);
    let java_str = unwrap_jni(env.get_string(&java_string), env);
    String::from(java_str)
}

#[test]
fn jmap_put_get_replace_remove_and_iterate_workflow() {
    let vm = shared_jvm();
    let mut env = vm
        .attach_current_thread()
        .expect("failed to attach current thread");

    let map_object = unwrap_jni(env.new_object("java/util/HashMap", "()V", &[]), &env);
    let map = unwrap_jni(JMap::from_env(&mut env, &map_object), &env);

    let key_alpha = make_java_string_object(&env, "alpha");
    let key_beta = make_java_string_object(&env, "beta");
    let key_gamma = make_java_string_object(&env, "gamma");
    let key_missing = make_java_string_object(&env, "missing");

    let value_one = make_java_string_object(&env, "one");
    let value_two = make_java_string_object(&env, "two");
    let value_three = make_java_string_object(&env, "three");
    let value_one_replaced = make_java_string_object(&env, "one-replaced");

    let previous_alpha = unwrap_jni(map.put(&mut env, &key_alpha, &value_one), &env);
    assert!(
        previous_alpha.is_none(),
        "first insertion for alpha should not replace an existing value"
    );

    let previous_beta = unwrap_jni(map.put(&mut env, &key_beta, &value_two), &env);
    assert!(
        previous_beta.is_none(),
        "first insertion for beta should not replace an existing value"
    );

    let previous_gamma = unwrap_jni(map.put(&mut env, &key_gamma, &value_three), &env);
    assert!(
        previous_gamma.is_none(),
        "first insertion for gamma should not replace an existing value"
    );

    let fetched_alpha = unwrap_jni(map.get(&mut env, &key_alpha), &env)
        .expect("alpha should be present after insertion");
    assert_eq!(java_object_to_string(&mut env, fetched_alpha), "one");

    let missing_before_insert = unwrap_jni(map.get(&mut env, &key_missing), &env);
    assert!(
        missing_before_insert.is_none(),
        "unknown key should not have a value"
    );

    let replaced_alpha = unwrap_jni(map.put(&mut env, &key_alpha, &value_one_replaced), &env)
        .expect("second insertion for alpha should return the previous value");
    assert_eq!(java_object_to_string(&mut env, replaced_alpha), "one");

    let fetched_replaced_alpha = unwrap_jni(map.get(&mut env, &key_alpha), &env)
        .expect("alpha should remain present after replacement");
    assert_eq!(
        java_object_to_string(&mut env, fetched_replaced_alpha),
        "one-replaced"
    );

    let removed_beta = unwrap_jni(map.remove(&mut env, &key_beta), &env)
        .expect("removing beta should return its previous value");
    assert_eq!(java_object_to_string(&mut env, removed_beta), "two");

    let fetched_removed_beta = unwrap_jni(map.get(&mut env, &key_beta), &env);
    assert!(
        fetched_removed_beta.is_none(),
        "removed key should no longer be present"
    );

    let removed_missing = unwrap_jni(map.remove(&mut env, &key_missing), &env);
    assert!(
        removed_missing.is_none(),
        "removing a missing key should return None"
    );

    let mut iter = unwrap_jni(map.iter(&mut env), &env);
    let mut observed = BTreeMap::new();

    while let Some((key, value)) = unwrap_jni(JMapIter::next(&mut iter, &mut env), &env) {
        observed.insert(
            java_object_to_string(&mut env, key),
            java_object_to_string(&mut env, value),
        );
    }

    let expected = BTreeMap::from([
        ("alpha".to_owned(), "one-replaced".to_owned()),
        ("gamma".to_owned(), "three".to_owned()),
    ]);

    assert_eq!(observed, expected);
    assert_eq!(observed.len(), 2);
    assert!(observed.contains_key("alpha"));
    assert!(!observed.contains_key("beta"));
}