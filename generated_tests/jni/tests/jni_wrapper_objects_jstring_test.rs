use jni::objects::{JObject, JString};
use jni::sys::{jobject, jstring};
use std::ptr;

#[test]
fn null_jstring_round_trips_through_from_raw_and_into_raw() {
    let raw: jstring = ptr::null_mut();

    let string: JString<'static> = unsafe { JString::from_raw(raw) };
    let unwrapped: jstring = JString::into_raw(string);

    assert!(
        unwrapped.is_null(),
        "JString::into_raw should preserve a null jstring pointer"
    );
    assert_eq!(
        unwrapped, raw,
        "JString::from_raw followed by JString::into_raw should return the exact null pointer supplied"
    );
    assert_eq!(
        unwrapped,
        ptr::null_mut(),
        "the round-tripped null jstring should equal the platform null pointer"
    );
}

#[test]
fn non_null_jstring_pointer_identity_is_preserved_without_dereferencing() {
    let raw: jstring = 0x1usize as jstring;

    let string: JString<'static> = unsafe { JString::from_raw(raw) };
    let unwrapped: jstring = JString::into_raw(string);

    assert!(
        !unwrapped.is_null(),
        "a non-null raw jstring should remain non-null after wrapping and unwrapping"
    );
    assert_eq!(
        unwrapped, raw,
        "JString::into_raw should return the exact non-null raw pointer supplied to JString::from_raw"
    );

    let wrapped_again: JString<'static> = unsafe { JString::from_raw(unwrapped) };
    let unwrapped_again: jstring = JString::into_raw(wrapped_again);

    assert_eq!(
        unwrapped_again, raw,
        "repeated JString::from_raw / into_raw cycles should preserve pointer identity"
    );
}

#[test]
fn jstring_raw_pointer_can_be_compared_with_equivalent_jobject_pointer() {
    let raw_string: jstring = 0x2usize as jstring;
    let equivalent_object_raw: jobject = raw_string as jobject;

    let string: JString<'static> = unsafe { JString::from_raw(raw_string) };
    let object: JObject<'static> = unsafe { JObject::from_raw(equivalent_object_raw) };

    let string_raw_after_round_trip: jstring = JString::into_raw(string);
    let object_raw_after_round_trip: jobject = JObject::into_raw(object);

    assert_eq!(
        string_raw_after_round_trip as jobject, equivalent_object_raw,
        "a raw jstring should have the same object pointer value when viewed as jobject"
    );
    assert_eq!(
        object_raw_after_round_trip, equivalent_object_raw,
        "JObject should preserve the same underlying raw pointer value"
    );
    assert_eq!(
        string_raw_after_round_trip as jobject, object_raw_after_round_trip,
        "JString and JObject wrappers should not alter the shared JNI object pointer value"
    );
    assert_ne!(
        string_raw_after_round_trip,
        ptr::null_mut(),
        "the chosen sentinel jstring pointer should remain non-null after round-trip"
    );
}