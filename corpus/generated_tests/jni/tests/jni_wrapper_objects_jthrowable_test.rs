use jni::objects::JThrowable;
use jni::sys::jthrowable;
use std::ptr;

#[test]
fn null_jthrowable_round_trips_through_from_raw_and_into_raw() {
    let raw: jthrowable = ptr::null_mut();

    let throwable: JThrowable<'static> = unsafe { JThrowable::from_raw(raw) };
    let unwrapped: jthrowable = JThrowable::into_raw(throwable);

    assert!(
        unwrapped.is_null(),
        "JThrowable::into_raw should preserve a null throwable pointer"
    );
    assert_eq!(
        unwrapped, raw,
        "JThrowable::from_raw followed by into_raw should return the exact null pointer supplied"
    );
    assert_eq!(
        unwrapped,
        ptr::null_mut(),
        "the raw representation of a null JThrowable should be the platform null pointer"
    );
}

#[test]
fn non_null_jthrowable_pointer_identity_is_preserved_without_dereferencing() {
    let raw: jthrowable = 0x1usize as jthrowable;

    assert!(
        !raw.is_null(),
        "the synthetic raw throwable used by this test should be non-null"
    );

    let throwable: JThrowable<'static> = unsafe { JThrowable::from_raw(raw) };
    let unwrapped: jthrowable = JThrowable::into_raw(throwable);

    assert!(
        !unwrapped.is_null(),
        "JThrowable::into_raw should preserve non-nullness for an opaque raw pointer"
    );
    assert_eq!(
        unwrapped, raw,
        "JThrowable should preserve raw pointer identity when wrapping and unwrapping"
    );
}

#[test]
fn repeated_wrap_unwrap_cycles_do_not_change_the_raw_jthrowable_value() {
    let original: jthrowable = 0x2usize as jthrowable;

    let first_wrapper: JThrowable<'static> = unsafe { JThrowable::from_raw(original) };
    let first_raw: jthrowable = JThrowable::into_raw(first_wrapper);

    let second_wrapper: JThrowable<'static> = unsafe { JThrowable::from_raw(first_raw) };
    let second_raw: jthrowable = JThrowable::into_raw(second_wrapper);

    assert_eq!(
        first_raw, original,
        "the first JThrowable wrap/unwrap cycle should preserve the original raw pointer"
    );
    assert_eq!(
        second_raw, first_raw,
        "a second JThrowable wrap/unwrap cycle should preserve the pointer returned by the first"
    );
    assert_eq!(
        second_raw, original,
        "JThrowable raw pointer identity should remain stable across multiple cycles"
    );
    assert!(
        !second_raw.is_null(),
        "the final raw throwable should remain non-null after multiple cycles"
    );
}