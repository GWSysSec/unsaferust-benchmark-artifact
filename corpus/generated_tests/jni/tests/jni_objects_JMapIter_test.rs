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

fn object_to_string(env: &mut JNIEnv<'_>, obj: JObject<'_>) -> String {
    assert!(!obj.as_raw().is_null(), "expected non-null Java string object");
    let java_string = JString::from(obj);
    let java_str = unwrap_jni(env.get_string(&java_string), env);
    String::from(java_str)
}

#[test]
fn jmap_iter_next_visits_all_entries_and_reports_end() {
    let vm = shared_jvm();
    let mut env = vm
        .attach_current_thread()
        .expect("failed to attach current thread");

    let map_object = unwrap_jni(
        env.new_object("java/util/LinkedHashMap", "()V", &[]),
        &env,
    );
    let map = unwrap_jni(JMap::from_env(&mut env, &map_object), &env);

    assert!(
        unwrap_jni(JMap::iter(&map, &mut env), &env)
            .next(&mut env)
            .expect("empty iterator next failed")
            .is_none(),
        "iterator over a newly created map should be exhausted immediately"
    );

    let expected_pairs = [
        ("first-key", "first-value"),
        ("second-key", "second-value"),
        ("third-key", "third-value"),
    ];

    for (key, value) in expected_pairs {
        let key_obj = JObject::from(unwrap_jni(env.new_string(key), &env));
        let value_obj = JObject::from(unwrap_jni(env.new_string(value), &env));

        let previous = unwrap_jni(JMap::put(&map, &mut env, &key_obj, &value_obj), &env);
        assert!(
            previous.is_none(),
            "inserting a new key should not replace an existing value"
        );
    }

    let replacement_key = JObject::from(unwrap_jni(env.new_string("second-key"), &env));
    let replacement_value = JObject::from(unwrap_jni(env.new_string("updated-second-value"), &env));
    let old_value = unwrap_jni(
        JMap::put(&map, &mut env, &replacement_key, &replacement_value),
        &env,
    );
    assert!(
        old_value.is_some(),
        "putting an existing key should return the prior value"
    );
    assert_eq!(
        object_to_string(&mut env, old_value.expect("old value disappeared")),
        "second-value"
    );

    let mut iter = unwrap_jni(JMap::iter(&map, &mut env), &env);
    let mut actual = BTreeMap::new();

    while let Some((key, value)) = unwrap_jni(JMapIter::next(&mut iter, &mut env), &env) {
        let key_string = object_to_string(&mut env, key);
        let value_string = object_to_string(&mut env, value);
        let previous = actual.insert(key_string, value_string);
        assert!(
            previous.is_none(),
            "map iterator should not yield duplicate keys"
        );
    }

    assert_eq!(actual.len(), 3);
    assert_eq!(actual.get("first-key").map(String::as_str), Some("first-value"));
    assert_eq!(
        actual.get("second-key").map(String::as_str),
        Some("updated-second-value")
    );
    assert_eq!(actual.get("third-key").map(String::as_str), Some("third-value"));

    assert!(
        unwrap_jni(JMapIter::next(&mut iter, &mut env), &env).is_none(),
        "calling JMapIter::next after exhaustion should keep returning None"
    );

    let removed = unwrap_jni(JMap::remove(&map, &mut env, &replacement_key), &env);
    assert!(removed.is_some(), "existing key should be removable");
    assert_eq!(
        object_to_string(&mut env, removed.expect("removed value disappeared")),
        "updated-second-value"
    );

    let after_remove = unwrap_jni(JMap::get(&map, &mut env, &replacement_key), &env);
    assert!(
        after_remove.is_none(),
        "removed key should no longer be present in the map"
    );

    let mut count_after_remove = 0;
    let mut iter_after_remove = unwrap_jni(JMap::iter(&map, &mut env), &env);
    while let Some((key, value)) = unwrap_jni(JMapIter::next(&mut iter_after_remove, &mut env), &env)
    {
        assert!(!object_to_string(&mut env, key).is_empty());
        assert!(!object_to_string(&mut env, value).is_empty());
        count_after_remove += 1;
    }
    assert_eq!(count_after_remove, 2);
}