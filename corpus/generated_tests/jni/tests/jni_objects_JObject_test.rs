use jni::objects::JObject;
use jni::sys::jobject;
use std::ptr;

#[test]
fn null_jobject_reports_and_round_trips_null_raw_pointer() {
    let obj: JObject<'static> = JObject::null();

    let raw_from_borrow: jobject = JObject::as_raw(&obj);
    assert!(
        raw_from_borrow.is_null(),
        "JObject::null should expose a null raw jobject through as_raw"
    );

    let raw_from_into: jobject = JObject::into_raw(obj);
    assert!(
        raw_from_into.is_null(),
        "JObject::into_raw should preserve the null pointer from JObject::null"
    );
    assert_eq!(
        raw_from_into,
        ptr::null_mut(),
        "JObject::null should round-trip to exactly the platform null jobject value"
    );
}

#[test]
fn from_raw_accepts_null_and_preserves_it_through_borrow_and_unwrap() {
    let raw: jobject = ptr::null_mut();

    let obj: JObject<'static> = unsafe { JObject::from_raw(raw) };
    let borrowed_raw: jobject = JObject::as_raw(&obj);

    assert!(
        borrowed_raw.is_null(),
        "JObject::from_raw(null) should create an object whose raw pointer is null"
    );
    assert_eq!(
        borrowed_raw, raw,
        "JObject::as_raw should return the exact raw pointer supplied to from_raw"
    );

    let unwrapped_raw: jobject = JObject::into_raw(obj);
    assert!(
        unwrapped_raw.is_null(),
        "JObject::into_raw should return null after constructing from a null raw pointer"
    );
    assert_eq!(
        unwrapped_raw, raw,
        "JObject::into_raw should preserve the exact null raw pointer supplied to from_raw"
    );
}

#[test]
fn null_object_can_be_rewrapped_multiple_times_without_changing_raw_value() {
    let first: JObject<'static> = JObject::null();
    let first_raw: jobject = JObject::into_raw(first);

    let second: JObject<'static> = unsafe { JObject::from_raw(first_raw) };
    let second_raw_from_borrow: jobject = JObject::as_raw(&second);

    assert_eq!(
        second_raw_from_borrow, first_raw,
        "rewrapping a null raw jobject should not alter the pointer observed by as_raw"
    );

    let second_raw_from_into: jobject = JObject::into_raw(second);
    assert_eq!(
        second_raw_from_into, first_raw,
        "into_raw after rewrapping should return the same null pointer value"
    );

    let third: JObject<'static> = unsafe { JObject::from_raw(second_raw_from_into) };
    assert!(
        JObject::as_raw(&third).is_null(),
        "a null raw jobject should remain null after repeated from_raw/as_raw conversions"
    );
    assert_eq!(
        JObject::into_raw(third),
        ptr::null_mut(),
        "repeated null conversions should still finish as exactly a null jobject"
    );
}