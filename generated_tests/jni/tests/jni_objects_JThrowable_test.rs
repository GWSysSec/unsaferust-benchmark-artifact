use jni::objects::{JObject, JThrowable};
use jni::sys::{jobject, jthrowable};
use std::ptr;

#[test]
fn null_jthrowable_from_raw_round_trips_through_into_raw() {
    let raw: jthrowable = ptr::null_mut();

    let throwable: JThrowable<'static> = unsafe { jni::objects::JThrowable::from_raw(raw) };
    let unwrapped: jthrowable = jni::objects::JThrowable::into_raw(throwable);

    assert!(
        unwrapped.is_null(),
        "JThrowable::into_raw should preserve a null jthrowable"
    );
    assert_eq!(
        unwrapped, raw,
        "a null jthrowable should round-trip without changing pointer value"
    );
    assert_eq!(
        unwrapped,
        ptr::null_mut(),
        "the platform null jthrowable value should remain exact"
    );
}

#[test]
fn null_jthrowable_can_be_rewrapped_multiple_times_without_pointer_changes() {
    let first_raw: jthrowable = ptr::null_mut();

    let first: JThrowable<'static> = unsafe { jni::objects::JThrowable::from_raw(first_raw) };
    let second_raw: jthrowable = jni::objects::JThrowable::into_raw(first);

    let second: JThrowable<'static> = unsafe { jni::objects::JThrowable::from_raw(second_raw) };
    let final_raw: jthrowable = jni::objects::JThrowable::into_raw(second);

    assert!(
        second_raw.is_null(),
        "rewrapping should preserve null after the first unwrap"
    );
    assert!(
        final_raw.is_null(),
        "rewrapping should preserve null after the second unwrap"
    );
    assert_eq!(
        second_raw, first_raw,
        "first unwrap should return the exact raw pointer supplied to from_raw"
    );
    assert_eq!(
        final_raw, first_raw,
        "second unwrap should still return the original raw pointer value"
    );
}

#[test]
fn null_jthrowable_raw_pointer_is_compatible_with_jobject_wrapper_workflow() {
    let raw_throwable: jthrowable = ptr::null_mut();

    let throwable: JThrowable<'static> =
        unsafe { jni::objects::JThrowable::from_raw(raw_throwable) };
    let raw_after_throwable_round_trip: jthrowable = jni::objects::JThrowable::into_raw(throwable);

    let raw_as_object: jobject = raw_after_throwable_round_trip;
    let object: JObject<'static> = unsafe { jni::objects::JObject::from_raw(raw_as_object) };
    let raw_after_object_round_trip: jobject = jni::objects::JObject::into_raw(object);

    assert!(
        raw_after_throwable_round_trip.is_null(),
        "JThrowable round-trip should leave null throwable raw pointer null"
    );
    assert!(
        raw_after_object_round_trip.is_null(),
        "the same null raw reference should also remain null through JObject"
    );
    assert_eq!(
        raw_after_object_round_trip, raw_as_object,
        "JObject::into_raw should preserve the raw pointer obtained from JThrowable::into_raw"
    );
    assert_eq!(
        raw_after_object_round_trip as jthrowable, raw_throwable,
        "cross-wrapper null raw reference workflow should preserve the original jthrowable value"
    );
}