use jni::objects::JObject;
use jni::sys::jobject;
use std::ptr;
use std::ptr::NonNull;

#[test]
fn null_jobject_workflow_preserves_null_pointer_identity() {
    let obj: JObject<'static> = JObject::null();

    let first_borrow: jobject = JObject::as_raw(&obj);
    let second_borrow: jobject = JObject::as_raw(&obj);

    assert!(
        first_borrow.is_null(),
        "JObject::null must expose a null raw jobject"
    );
    assert_eq!(
        first_borrow,
        ptr::null_mut(),
        "JObject::null must use the platform null pointer value"
    );
    assert_eq!(
        second_borrow, first_borrow,
        "calling JObject::as_raw repeatedly must be stable and non-mutating"
    );

    let consumed_raw: jobject = JObject::into_raw(obj);

    assert!(
        consumed_raw.is_null(),
        "JObject::into_raw must preserve nullness for JObject::null"
    );
    assert_eq!(
        consumed_raw, first_borrow,
        "JObject::into_raw must return the same raw pointer previously observed through as_raw"
    );
}

#[test]
fn from_raw_null_round_trips_through_as_raw_and_into_raw() {
    let raw: jobject = ptr::null_mut();

    let obj: JObject<'static> = unsafe { JObject::from_raw(raw) };
    let borrowed: jobject = JObject::as_raw(&obj);

    assert!(
        borrowed.is_null(),
        "JObject::from_raw(null) must create a wrapper around a null raw jobject"
    );
    assert_eq!(
        borrowed, raw,
        "JObject::as_raw must return exactly the raw pointer supplied to from_raw"
    );

    let unwrapped: jobject = JObject::into_raw(obj);

    assert!(
        unwrapped.is_null(),
        "JObject::into_raw must preserve a null pointer supplied through from_raw"
    );
    assert_eq!(
        unwrapped, raw,
        "a null raw jobject must round-trip without pointer value changes"
    );
}

#[test]
fn non_null_raw_pointer_identity_is_preserved_without_dereferencing() {
    let raw: jobject = NonNull::<jni::sys::_jobject>::dangling().as_ptr();

    assert!(
        !raw.is_null(),
        "test setup must use a non-null dangling pointer and never dereference it"
    );

    let obj: JObject<'static> = unsafe { JObject::from_raw(raw) };

    let first_borrow: jobject = JObject::as_raw(&obj);
    let second_borrow: jobject = JObject::as_raw(&obj);

    assert!(
        !first_borrow.is_null(),
        "JObject::from_raw(non_null) must preserve non-nullness"
    );
    assert_eq!(
        first_borrow, raw,
        "JObject::as_raw must expose the exact non-null raw pointer supplied"
    );
    assert_eq!(
        second_borrow, raw,
        "repeated JObject::as_raw calls must keep returning the same pointer"
    );

    let unwrapped: jobject = JObject::into_raw(obj);

    assert_eq!(
        unwrapped, raw,
        "JObject::into_raw must return the exact raw pointer originally supplied"
    );
    assert!(
        !unwrapped.is_null(),
        "JObject::into_raw must preserve non-nullness"
    );
}

#[test]
fn separate_wrappers_created_from_the_same_raw_pointer_have_matching_raw_identity() {
    let raw: jobject = NonNull::<jni::sys::_jobject>::dangling().as_ptr();

    let first: JObject<'static> = unsafe { JObject::from_raw(raw) };
    let second: JObject<'static> = unsafe { JObject::from_raw(raw) };

    assert_eq!(
        JObject::as_raw(&first),
        JObject::as_raw(&second),
        "two JObject wrappers created from the same raw jobject must expose matching raw identity"
    );
    assert_eq!(
        JObject::as_raw(&first),
        raw,
        "the first wrapper must preserve the original raw pointer"
    );
    assert_eq!(
        JObject::as_raw(&second),
        raw,
        "the second wrapper must preserve the original raw pointer"
    );

    let first_raw: jobject = JObject::into_raw(first);
    let second_raw: jobject = JObject::into_raw(second);

    assert_eq!(
        first_raw, second_raw,
        "consuming separate wrappers for the same raw jobject must return matching raw pointers"
    );
    assert_eq!(
        first_raw, raw,
        "consuming the first wrapper must return the original raw pointer"
    );
    assert_eq!(
        second_raw, raw,
        "consuming the second wrapper must return the original raw pointer"
    );
}