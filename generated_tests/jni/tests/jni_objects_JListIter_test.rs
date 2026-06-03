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

fn object_to_string(env: &mut JNIEnv<'_>, obj: JObject<'_>) -> String {
    let java_string = JString::from(obj);
    let java_str = env
        .get_string(&java_string)
        .expect("failed to read Java string");
    String::from(java_str)
}

#[test]
fn jlist_iterator_next_walks_mutated_list_and_reports_end() {
    let vm = shared_jvm();
    let mut env = vm
        .attach_current_thread()
        .expect("failed to attach current thread");

    let list_object = env
        .new_object("java/util/ArrayList", "()V", &[])
        .expect("failed to construct ArrayList");
    let list = JList::from_env(&mut env, &list_object).expect("failed to wrap ArrayList as JList");

    assert_eq!(JList::size(&list, &mut env).expect("size failed"), 0);

    let alpha = JObject::from(env.new_string("alpha").expect("failed to create alpha"));
    let gamma = JObject::from(env.new_string("gamma").expect("failed to create gamma"));
    let beta = JObject::from(env.new_string("beta").expect("failed to create beta"));
    let delta = JObject::from(env.new_string("delta").expect("failed to create delta"));

    JList::add(&list, &mut env, &alpha).expect("failed to add alpha");
    JList::add(&list, &mut env, &gamma).expect("failed to add gamma");
    JList::insert(&list, &mut env, 1, &beta).expect("failed to insert beta");
    JList::add(&list, &mut env, &delta).expect("failed to add delta");

    assert_eq!(JList::size(&list, &mut env).expect("size failed"), 4);

    let removed = JList::remove(&list, &mut env, 2).expect("failed to remove element");
    assert!(removed.is_some());
    assert_eq!(
        object_to_string(&mut env, removed.expect("removed element was unexpectedly absent")),
        "gamma"
    );
    assert_eq!(JList::size(&list, &mut env).expect("size failed"), 3);

    let mut iter = JList::iter(&list, &mut env).expect("failed to create JList iterator");
    let mut collected = Vec::new();

    while let Some(obj) =
        jni::objects::JListIter::next(&mut iter, &mut env).expect("iterator next failed")
    {
        collected.push(object_to_string(&mut env, obj));
    }

    assert_eq!(collected, vec!["alpha", "beta", "delta"]);

    let after_end = JListIter::next(&mut iter, &mut env).expect("iterator next after end failed");
    assert!(after_end.is_none());

    let popped = JList::pop(&list, &mut env).expect("failed to pop list");
    assert!(popped.is_some());
    assert_eq!(
        object_to_string(&mut env, popped.expect("popped element was unexpectedly absent")),
        "delta"
    );
    assert_eq!(JList::size(&list, &mut env).expect("size failed"), 2);
}