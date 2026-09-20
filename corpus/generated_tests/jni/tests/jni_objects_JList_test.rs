#![cfg(feature = "invocation")]

use jni::objects::{JList, JListIter, JObject, JString};

mod util;
use util::{attach_current_thread, unwrap};

fn java_object_to_rust_string(env: &mut jni::JNIEnv<'_>, obj: JObject<'_>) -> String {
    let java_string = JString::from(obj);
    let java_str = unwrap(env.get_string(&java_string), env);
    String::from(java_str)
}

#[test]
fn jlist_array_list_full_workflow_with_null_edge_case() {
    let mut env = attach_current_thread();

    let list_object = unwrap(env.new_object("java/util/ArrayList", "()V", &[]), &env);
    let list = unwrap(JList::from_env(&mut env, &list_object), &env);

    assert_eq!(unwrap(JList::size(&list, &mut env), &env), 0);

    let alpha = JObject::from(unwrap(env.new_string("alpha"), &env));
    let gamma = JObject::from(unwrap(env.new_string("gamma"), &env));
    let beta = JObject::from(unwrap(env.new_string("beta"), &env));
    let null_object = JObject::null();

    unwrap(JList::add(&list, &mut env, &alpha), &env);
    unwrap(JList::add(&list, &mut env, &gamma), &env);
    unwrap(JList::add(&list, &mut env, &null_object), &env);

    assert_eq!(unwrap(JList::size(&list, &mut env), &env), 3);

    unwrap(JList::insert(&list, &mut env, 1, &beta), &env);

    assert_eq!(unwrap(JList::size(&list, &mut env), &env), 4);

    let first = unwrap(JList::get(&list, &mut env, 0), &env);
    assert!(first.is_some());
    assert_eq!(
        java_object_to_rust_string(&mut env, first.unwrap()),
        "alpha"
    );

    let inserted = unwrap(JList::get(&list, &mut env, 1), &env);
    assert!(inserted.is_some());
    assert_eq!(
        java_object_to_rust_string(&mut env, inserted.unwrap()),
        "beta"
    );

    let null_entry = unwrap(JList::get(&list, &mut env, 3), &env);
    assert!(
        null_entry.is_none(),
        "JList::get should translate a Java null element into None"
    );

    let removed = unwrap(JList::remove(&list, &mut env, 2), &env);
    assert!(removed.is_some());
    assert_eq!(
        java_object_to_rust_string(&mut env, removed.unwrap()),
        "gamma"
    );

    assert_eq!(unwrap(JList::size(&list, &mut env), &env), 3);

    let popped_null = unwrap(JList::pop(&list, &mut env), &env);
    assert!(
        popped_null.is_none(),
        "JList::pop should translate a popped Java null element into None"
    );

    assert_eq!(unwrap(JList::size(&list, &mut env), &env), 2);

    let mut iterated = Vec::new();
    let mut iterator = unwrap(JList::iter(&list, &mut env), &env);
    loop {
        let next = unwrap(JListIter::next(&mut iterator, &mut env), &env);
        match next {
            Some(obj) => iterated.push(java_object_to_rust_string(&mut env, obj)),
            None => break,
        }
    }

    assert_eq!(iterated, vec!["alpha".to_owned(), "beta".to_owned()]);

    let popped_beta = unwrap(JList::pop(&list, &mut env), &env);
    assert!(popped_beta.is_some());
    assert_eq!(
        java_object_to_rust_string(&mut env, popped_beta.unwrap()),
        "beta"
    );

    assert_eq!(unwrap(JList::size(&list, &mut env), &env), 1);

    let remaining = unwrap(JList::get(&list, &mut env, 0), &env);
    assert!(remaining.is_some());
    assert_eq!(
        java_object_to_rust_string(&mut env, remaining.unwrap()),
        "alpha"
    );
}