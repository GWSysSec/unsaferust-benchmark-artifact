use jni::objects::JClass;
use jni::sys::jclass;
use std::ptr;

#[test]
fn null_jclass_round_trips_through_from_as_and_into_raw() {
    let raw: jclass = ptr::null_mut();

    let class: JClass<'static> = unsafe { JClass::from_raw(raw) };

    let first_borrow: jclass = JClass::as_raw(&class);
    let second_borrow: jclass = JClass::as_raw(&class);

    assert!(
        first_borrow.is_null(),
        "JClass::from_raw(null) should produce a class whose borrowed raw pointer is null"
    );
    assert_eq!(
        first_borrow, raw,
        "JClass::as_raw should return the exact null raw pointer passed to from_raw"
    );
    assert_eq!(
        second_borrow, first_borrow,
        "repeated JClass::as_raw calls should be stable and non-mutating"
    );

    let unwrapped: jclass = JClass::into_raw(class);

    assert!(
        unwrapped.is_null(),
        "JClass::into_raw should preserve a null jclass pointer"
    );
    assert_eq!(
        unwrapped, raw,
        "JClass::into_raw should return exactly the raw pointer originally supplied"
    );
}

#[test]
fn non_null_jclass_pointer_identity_is_preserved_without_dereferencing() {
    let raw: jclass = std::ptr::NonNull::<std::ffi::c_void>::dangling().as_ptr() as jclass;

    let class: JClass<'static> = unsafe { JClass::from_raw(raw) };

    let borrowed_before: jclass = JClass::as_raw(&class);
    assert!(
        !borrowed_before.is_null(),
        "the synthetic non-null jclass pointer used for identity testing should remain non-null"
    );
    assert_eq!(
        borrowed_before, raw,
        "JClass::as_raw should preserve non-null pointer identity"
    );

    let borrowed_again: jclass = JClass::as_raw(&class);
    assert_eq!(
        borrowed_again, borrowed_before,
        "borrowing the raw jclass multiple times should not change the stored pointer"
    );

    let unwrapped: jclass = JClass::into_raw(class);
    assert_eq!(
        unwrapped, raw,
        "JClass::into_raw should return the same non-null pointer supplied to from_raw"
    );
    assert_eq!(
        unwrapped, borrowed_before,
        "the pointer observed through as_raw should match the pointer returned by into_raw"
    );
}

#[test]
fn separate_jclass_wrappers_keep_their_own_raw_pointer_values() {
    let null_raw: jclass = ptr::null_mut();
    let non_null_raw: jclass = std::ptr::NonNull::<usize>::dangling().as_ptr() as jclass;

    let null_class: JClass<'static> = unsafe { JClass::from_raw(null_raw) };
    let non_null_class: JClass<'static> = unsafe { JClass::from_raw(non_null_raw) };

    assert_ne!(
        JClass::as_raw(&null_class),
        JClass::as_raw(&non_null_class),
        "wrappers created from different raw jclass values should expose different pointers"
    );
    assert!(
        JClass::as_raw(&null_class).is_null(),
        "the null wrapper should continue to expose a null raw pointer"
    );
    assert!(
        !JClass::as_raw(&non_null_class).is_null(),
        "the non-null wrapper should continue to expose a non-null raw pointer"
    );

    let null_unwrapped: jclass = JClass::into_raw(null_class);
    let non_null_unwrapped: jclass = JClass::into_raw(non_null_class);

    assert_eq!(
        null_unwrapped, null_raw,
        "the null wrapper should unwrap to its original raw pointer"
    );
    assert_eq!(
        non_null_unwrapped, non_null_raw,
        "the non-null wrapper should unwrap to its original raw pointer"
    );
    assert_ne!(
        null_unwrapped, non_null_unwrapped,
        "unwrapping separate wrappers should preserve their distinct raw pointer identities"
    );
}