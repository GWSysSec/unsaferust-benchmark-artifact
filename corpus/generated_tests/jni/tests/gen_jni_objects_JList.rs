#![cfg(feature = "invocation")]

use jni::{
    objects::{JList, JObject, JString},
    sys::jint,
    JNIEnv,
};

mod util;
use util::jvm;

/// Helper to get the size of a Java List via direct JNI method call.
fn list_size(env: &mut JNIEnv, list_obj: &JObject) -> jint {
    env.call_method(list_obj, "size", "()I", &[])
        .unwrap()
        .i()
        .unwrap()
}

/// Helper to convert a JObject (known to be a java.lang.String) to a Rust String.
fn jobject_to_string<'local>(env: &mut JNIEnv<'local>, obj: JObject<'local>) -> String {
    let jstr: JString<'local> = obj.into();
    let java_str = env.get_string(&jstr).unwrap();
    java_str.to_str().unwrap().to_owned()
}

#[test]
fn test_insert_grows_list_and_preserves_order() {
    let mut env = jvm().attach_current_thread().unwrap();

    let list_obj = env
        .new_object("java/util/ArrayList", "()V", &[])
        .unwrap();
    let list = JList::from_env(&mut env, &list_obj).unwrap();

    // Pre-state: empty list.
    assert_eq!(list_size(&mut env, &list_obj), 0);

    let s_alpha = env.new_string("alpha").unwrap();
    let s_beta = env.new_string("beta").unwrap();
    let s_gamma = env.new_string("gamma").unwrap();
    let s_delta = env.new_string("delta").unwrap();

    // Append-style inserts at the tail.
    list.insert(&mut env, 0, &s_alpha).unwrap();
    assert_eq!(list_size(&mut env, &list_obj), 1);

    list.insert(&mut env, 1, &s_beta).unwrap();
    assert_eq!(list_size(&mut env, &list_obj), 2);

    list.insert(&mut env, 2, &s_gamma).unwrap();
    assert_eq!(list_size(&mut env, &list_obj), 3);

    // Insert at the front; this should shift existing elements right.
    list.insert(&mut env, 0, &s_delta).unwrap();
    assert_eq!(list_size(&mut env, &list_obj), 4);

    // Expected order is now: [delta, alpha, beta, gamma].
    // Use pop to drain from the tail and verify the reversed order.
    let pop_gamma = list.pop(&mut env).unwrap().expect("expected gamma");
    assert_eq!(jobject_to_string(&mut env, pop_gamma), "gamma");
    assert_eq!(list_size(&mut env, &list_obj), 3);

    let pop_beta = list.pop(&mut env).unwrap().expect("expected beta");
    assert_eq!(jobject_to_string(&mut env, pop_beta), "beta");
    assert_eq!(list_size(&mut env, &list_obj), 2);

    let pop_alpha = list.pop(&mut env).unwrap().expect("expected alpha");
    assert_eq!(jobject_to_string(&mut env, pop_alpha), "alpha");
    assert_eq!(list_size(&mut env, &list_obj), 1);

    let pop_delta = list.pop(&mut env).unwrap().expect("expected delta");
    assert_eq!(jobject_to_string(&mut env, pop_delta), "delta");
    assert_eq!(list_size(&mut env, &list_obj), 0);
}

#[test]
fn test_pop_on_empty_returns_none() {
    let mut env = jvm().attach_current_thread().unwrap();

    let list_obj = env
        .new_object("java/util/ArrayList", "()V", &[])
        .unwrap();
    let list = JList::from_env(&mut env, &list_obj).unwrap();

    // Pre-state: empty.
    assert_eq!(list_size(&mut env, &list_obj), 0);

    // Pop on empty list should yield None, not an error.
    let result = list.pop(&mut env).unwrap();
    assert!(result.is_none());

    // Size unchanged.
    assert_eq!(list_size(&mut env, &list_obj), 0);

    // Push one, then pop twice: first Some, second None.
    let only = env.new_string("only").unwrap();
    list.insert(&mut env, 0, &only).unwrap();
    assert_eq!(list_size(&mut env, &list_obj), 1);

    let first = list.pop(&mut env).unwrap();
    assert!(first.is_some());
    assert_eq!(jobject_to_string(&mut env, first.unwrap()), "only");
    assert_eq!(list_size(&mut env, &list_obj), 0);

    let second = list.pop(&mut env).unwrap();
    assert!(second.is_none());
    assert_eq!(list_size(&mut env, &list_obj), 0);
}

#[test]
fn test_remove_returns_element_and_shrinks() {
    let mut env = jvm().attach_current_thread().unwrap();

    let list_obj = env
        .new_object("java/util/ArrayList", "()V", &[])
        .unwrap();
    let list = JList::from_env(&mut env, &list_obj).unwrap();

    assert_eq!(list_size(&mut env, &list_obj), 0);

    // Build: [a, b, c, d, e]
    let labels = ["a", "b", "c", "d", "e"];
    for (i, label) in labels.iter().enumerate() {
        let s = env.new_string(*label).unwrap();
        list.insert(&mut env, i as jint, &s).unwrap();
    }
    assert_eq!(list_size(&mut env, &list_obj), 5);

    // Remove from middle (index 2 -> "c"). List becomes [a, b, d, e].
    let removed_c = list.remove(&mut env, 2).unwrap();
    assert!(removed_c.is_some());
    assert_eq!(jobject_to_string(&mut env, removed_c.unwrap()), "c");
    assert_eq!(list_size(&mut env, &list_obj), 4);

    // Remove from head (index 0 -> "a"). List becomes [b, d, e].
    let removed_a = list.remove(&mut env, 0).unwrap();
    assert!(removed_a.is_some());
    assert_eq!(jobject_to_string(&mut env, removed_a.unwrap()), "a");
    assert_eq!(list_size(&mut env, &list_obj), 3);

    // Remove from tail (index size-1 = 2 -> "e"). List becomes [b, d].
    let removed_e = list.remove(&mut env, 2).unwrap();
    assert!(removed_e.is_some());
    assert_eq!(jobject_to_string(&mut env, removed_e.unwrap()), "e");
    assert_eq!(list_size(&mut env, &list_obj), 2);

    // Drain the rest with pop to confirm the remaining order is [b, d].
    let popped_d = list.pop(&mut env).unwrap().expect("expected d");
    assert_eq!(jobject_to_string(&mut env, popped_d), "d");
    assert_eq!(list_size(&mut env, &list_obj), 1);

    let popped_b = list.pop(&mut env).unwrap().expect("expected b");
    assert_eq!(jobject_to_string(&mut env, popped_b), "b");
    assert_eq!(list_size(&mut env, &list_obj), 0);
}

#[test]
fn test_insert_front_reverse_build_then_remove_front() {
    let mut env = jvm().attach_current_thread().unwrap();

    let list_obj = env
        .new_object("java/util/ArrayList", "()V", &[])
        .unwrap();
    let list = JList::from_env(&mut env, &list_obj).unwrap();

    assert_eq!(list_size(&mut env, &list_obj), 0);

    // Repeatedly inserting at index 0 reverses the logical insertion order.
    let inputs = ["one", "two", "three", "four"];
    for (i, s) in inputs.iter().enumerate() {
        let js = env.new_string(*s).unwrap();
        list.insert(&mut env, 0, &js).unwrap();
        assert_eq!(list_size(&mut env, &list_obj), (i as jint) + 1);
    }

    // Logical order is now: [four, three, two, one]
    // Remove from index 0 four times, asserting order.
    let expected = ["four", "three", "two", "one"];
    for (step, want) in expected.iter().enumerate() {
        let pre_size = list_size(&mut env, &list_obj);
        assert_eq!(pre_size, (expected.len() - step) as jint);

        let got = list.remove(&mut env, 0).unwrap();
        assert!(got.is_some(), "expected Some at step {}", step);
        assert_eq!(jobject_to_string(&mut env, got.unwrap()), *want);

        let post_size = list_size(&mut env, &list_obj);
        assert_eq!(post_size, pre_size - 1);
    }

    assert_eq!(list_size(&mut env, &list_obj), 0);

    // Final pop on drained list -> None.
    let tail = list.pop(&mut env).unwrap();
    assert!(tail.is_none());
}