#![cfg(feature = "invocation")]

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use jni::objects::{JMap, JObject, JString};
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

fn object_to_string(env: &mut JNIEnv<'_>, obj: JObject<'_>) -> String {
    let java_string = JString::from(obj);
    let java_str = env
        .get_string(&java_string)
        .expect("failed to read Java string");
    String::from(java_str)
}

#[test]
fn jmap_full_put_get_replace_remove_and_iter_workflow() {
    let vm = shared_jvm();
    let mut env = vm
        .attach_current_thread()
        .expect("failed to attach current thread");

    let map_object = env
        .new_object("java/util/HashMap", "()V", &[])
        .expect("failed to construct HashMap");
    let map = jni::objects::JMap::from_env(&mut env, &map_object).expect("failed to wrap HashMap");

    let missing_key = JObject::from(
        env.new_string("missing")
            .expect("failed to create missing key"),
    );
    assert!(
        jni::objects::JMap::get(&map, &mut env, &missing_key)
            .expect("failed to get missing key")
            .is_none(),
        "getting an absent key should return None"
    );

    let key_alpha = JObject::from(env.new_string("alpha").expect("failed to create alpha key"));
    let value_one = JObject::from(env.new_string("one").expect("failed to create one value"));
    let value_replaced = JObject::from(
        env.new_string("one-replaced")
            .expect("failed to create replacement value"),
    );

    let key_beta = JObject::from(env.new_string("beta").expect("failed to create beta key"));
    let value_two = JObject::from(env.new_string("two").expect("failed to create two value"));

    let key_gamma = JObject::from(env.new_string("gamma").expect("failed to create gamma key"));
    let value_three = JObject::from(
        env.new_string("three")
            .expect("failed to create three value"),
    );

    assert!(
        jni::objects::JMap::put(&map, &mut env, &key_alpha, &value_one)
            .expect("failed to insert alpha")
            .is_none(),
        "first insert for alpha should not replace an existing value"
    );
    assert!(
        jni::objects::JMap::put(&map, &mut env, &key_beta, &value_two)
            .expect("failed to insert beta")
            .is_none(),
        "first insert for beta should not replace an existing value"
    );
    assert!(
        jni::objects::JMap::put(&map, &mut env, &key_gamma, &value_three)
            .expect("failed to insert gamma")
            .is_none(),
        "first insert for gamma should not replace an existing value"
    );

    let fetched_alpha = jni::objects::JMap::get(&map, &mut env, &key_alpha)
        .expect("failed to fetch alpha")
        .expect("alpha should be present after insert");
    assert_eq!(object_to_string(&mut env, fetched_alpha), "one");

    let replaced = jni::objects::JMap::put(&map, &mut env, &key_alpha, &value_replaced)
        .expect("failed to replace alpha")
        .expect("replacing alpha should return the old value");
    assert_eq!(object_to_string(&mut env, replaced), "one");

    let fetched_replaced_alpha = jni::objects::JMap::get(&map, &mut env, &key_alpha)
        .expect("failed to fetch replaced alpha")
        .expect("alpha should still be present after replacement");
    assert_eq!(
        object_to_string(&mut env, fetched_replaced_alpha),
        "one-replaced"
    );

    let removed_beta = jni::objects::JMap::remove(&map, &mut env, &key_beta)
        .expect("failed to remove beta")
        .expect("beta should be removed with its previous value");
    assert_eq!(object_to_string(&mut env, removed_beta), "two");

    assert!(
        jni::objects::JMap::get(&map, &mut env, &key_beta)
            .expect("failed to get removed beta")
            .is_none(),
        "removed beta key should no longer be present"
    );
    assert!(
        jni::objects::JMap::remove(&map, &mut env, &key_beta)
            .expect("failed to remove beta a second time")
            .is_none(),
        "removing the same key twice should return None on the second removal"
    );

    let mut observed = BTreeMap::new();
    let mut iterator =
        jni::objects::JMap::iter(&map, &mut env).expect("failed to create map iterator");
    while let Some((key, value)) =
        jni::objects::JMapIter::next(&mut iterator, &mut env).expect("iterator next failed")
    {
        let key = object_to_string(&mut env, key);
        let value = object_to_string(&mut env, value);
        observed.insert(key, value);
    }

    let expected = BTreeMap::from([
        ("alpha".to_string(), "one-replaced".to_string()),
        ("gamma".to_string(), "three".to_string()),
    ]);

    assert_eq!(observed.len(), 2);
    assert_eq!(observed, expected);
}