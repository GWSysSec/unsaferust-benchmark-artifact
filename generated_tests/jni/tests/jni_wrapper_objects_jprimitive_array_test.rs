use jni::objects::JPrimitiveArray;
use jni::sys::{jarray, jbyte, jint, jlong};
use std::ptr;

#[test]
fn null_jprimitive_array_round_trips_through_from_raw_and_into_raw() {
    let raw: jarray = ptr::null_mut();

    let array: JPrimitiveArray<'static, jbyte> = unsafe { JPrimitiveArray::from_raw(raw) };
    let round_tripped: jarray = JPrimitiveArray::into_raw(array);

    assert!(
        round_tripped.is_null(),
        "JPrimitiveArray::into_raw should preserve a null primitive array pointer"
    );
    assert_eq!(
        round_tripped, raw,
        "JPrimitiveArray::from_raw followed by into_raw should return the exact null pointer supplied"
    );
    assert_eq!(
        round_tripped,
        ptr::null_mut(),
        "the round-tripped primitive array pointer should equal the platform null pointer"
    );
}

#[test]
fn non_null_jprimitive_array_pointer_identity_is_preserved_for_byte_arrays() {
    let raw: jarray = 0x1234usize as jarray;

    let array: JPrimitiveArray<'static, jbyte> = unsafe { JPrimitiveArray::from_raw(raw) };
    let first_round_trip: jarray = JPrimitiveArray::into_raw(array);

    assert!(
        !first_round_trip.is_null(),
        "a non-null raw primitive array pointer should remain non-null after wrapping"
    );
    assert_eq!(
        first_round_trip, raw,
        "JPrimitiveArray<jbyte> should preserve the exact raw pointer identity"
    );

    let array_again: JPrimitiveArray<'static, jbyte> =
        unsafe { JPrimitiveArray::from_raw(first_round_trip) };
    let second_round_trip: jarray = JPrimitiveArray::into_raw(array_again);

    assert_eq!(
        second_round_trip, first_round_trip,
        "re-wrapping a primitive array raw pointer should not change its identity"
    );
}

#[test]
fn primitive_array_raw_round_trip_is_independent_of_element_marker_type() {
    let byte_raw: jarray = 0x2000usize as jarray;
    let int_raw: jarray = 0x3000usize as jarray;
    let long_raw: jarray = 0x4000usize as jarray;

    let byte_array: JPrimitiveArray<'static, jbyte> = unsafe { JPrimitiveArray::from_raw(byte_raw) };
    let int_array: JPrimitiveArray<'static, jint> = unsafe { JPrimitiveArray::from_raw(int_raw) };
    let long_array: JPrimitiveArray<'static, jlong> = unsafe { JPrimitiveArray::from_raw(long_raw) };

    let byte_round_trip: jarray = JPrimitiveArray::into_raw(byte_array);
    let int_round_trip: jarray = JPrimitiveArray::into_raw(int_array);
    let long_round_trip: jarray = JPrimitiveArray::into_raw(long_array);

    assert_eq!(
        byte_round_trip, byte_raw,
        "JPrimitiveArray<jbyte> should return the byte-array raw pointer unchanged"
    );
    assert_eq!(
        int_round_trip, int_raw,
        "JPrimitiveArray<jint> should return the int-array raw pointer unchanged"
    );
    assert_eq!(
        long_round_trip, long_raw,
        "JPrimitiveArray<jlong> should return the long-array raw pointer unchanged"
    );
    assert_ne!(
        byte_round_trip, int_round_trip,
        "distinct raw primitive array pointers should remain distinct after round-tripping"
    );
    assert_ne!(
        int_round_trip, long_round_trip,
        "round-tripping should not collapse distinct primitive array identities"
    );
}