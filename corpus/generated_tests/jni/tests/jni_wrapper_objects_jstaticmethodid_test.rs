use jni::objects::JStaticMethodID;
use jni::sys::jmethodID;
use std::ptr;

#[test]
#[should_panic(expected = "from_raw methodID argument")]
fn null_jstaticmethodid_is_rejected_by_from_raw() {
    let raw: jmethodID = ptr::null_mut();

    assert!(
        raw.is_null(),
        "test setup must use a null jmethodID to verify from_raw rejects it"
    );

    let _method_id: JStaticMethodID = unsafe { JStaticMethodID::from_raw(raw) };
}

#[test]
fn non_null_jstaticmethodid_pointer_identity_is_preserved_without_dereferencing() {
    let raw: jmethodID = 0x1usize as jmethodID;

    assert!(
        !raw.is_null(),
        "the synthetic jmethodID used for identity testing should be non-null"
    );

    let method_id: JStaticMethodID = unsafe { JStaticMethodID::from_raw(raw) };
    let first_unwrapped: jmethodID = JStaticMethodID::into_raw(method_id);

    assert!(
        !first_unwrapped.is_null(),
        "JStaticMethodID::into_raw should preserve non-null pointer-ness"
    );
    assert_eq!(
        first_unwrapped, raw,
        "JStaticMethodID::into_raw should return the exact non-null pointer supplied to from_raw"
    );

    let method_id_again: JStaticMethodID = unsafe { JStaticMethodID::from_raw(first_unwrapped) };
    let second_unwrapped: jmethodID = JStaticMethodID::into_raw(method_id_again);

    assert_eq!(
        second_unwrapped, raw,
        "wrapping and unwrapping the same non-null jmethodID repeatedly should preserve identity"
    );
    assert_eq!(
        second_unwrapped as usize, 0x1usize,
        "JStaticMethodID must not transform the raw pointer value during round-trips"
    );
}

#[test]
fn distinct_raw_jstaticmethodids_remain_distinct_after_round_trips() {
    let first_raw: jmethodID = 0x1usize as jmethodID;
    let second_raw: jmethodID = 0x2usize as jmethodID;

    assert_ne!(
        first_raw, second_raw,
        "test setup should use two distinct synthetic jmethodID values"
    );
    assert!(
        !first_raw.is_null() && !second_raw.is_null(),
        "from_raw requires non-null jmethodID values"
    );

    let first_id: JStaticMethodID = unsafe { JStaticMethodID::from_raw(first_raw) };
    let second_id: JStaticMethodID = unsafe { JStaticMethodID::from_raw(second_raw) };

    let first_unwrapped: jmethodID = JStaticMethodID::into_raw(first_id);
    let second_unwrapped: jmethodID = JStaticMethodID::into_raw(second_id);

    assert_eq!(
        first_unwrapped, first_raw,
        "the first JStaticMethodID should preserve its own raw pointer"
    );
    assert_eq!(
        second_unwrapped, second_raw,
        "the second JStaticMethodID should preserve its own raw pointer"
    );
    assert_ne!(
        first_unwrapped, second_unwrapped,
        "two distinct raw static method IDs should remain distinct after wrapping"
    );
}