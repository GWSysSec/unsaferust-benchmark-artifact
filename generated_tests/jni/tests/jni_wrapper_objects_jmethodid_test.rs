use jni::objects::JMethodID;
use jni::sys::jmethodID;
use std::panic;
use std::ptr;

#[test]
fn null_jmethodid_is_rejected_by_from_raw() {
    let raw: jmethodID = ptr::null_mut();

    assert!(
        raw.is_null(),
        "test fixture should be a null jmethodID pointer"
    );

    let result = panic::catch_unwind(|| unsafe {
        JMethodID::from_raw(raw);
    });

    assert!(
        result.is_err(),
        "JMethodID::from_raw should reject a null jmethodID instead of constructing an invalid JMethodID"
    );
}

#[test]
fn non_null_jmethodid_pointer_identity_is_preserved_without_dereferencing() {
    let backing = Box::new(0x5a5a_1234_u64);
    let raw: jmethodID = Box::into_raw(backing) as jmethodID;

    assert!(
        !raw.is_null(),
        "Box::into_raw should provide a non-null pointer for the test fixture"
    );

    let method_id: JMethodID = unsafe { JMethodID::from_raw(raw) };
    let unwrapped: jmethodID = JMethodID::into_raw(method_id);

    assert!(
        !unwrapped.is_null(),
        "JMethodID::into_raw should preserve non-null pointer-ness"
    );
    assert_eq!(
        unwrapped, raw,
        "JMethodID should preserve the exact raw pointer identity"
    );

    let rewrapped: JMethodID = unsafe { JMethodID::from_raw(unwrapped) };
    let reunwrapped: jmethodID = JMethodID::into_raw(rewrapped);

    assert_eq!(
        reunwrapped, raw,
        "wrapping an already-unwrapped jmethodID should not alter its pointer value"
    );

    unsafe {
        drop(Box::from_raw(reunwrapped as *mut u64));
    }
}

#[test]
fn multiple_jmethodids_round_trip_independently_and_keep_ordered_identity() {
    let raw_values: Vec<jmethodID> = vec![
        Box::into_raw(Box::new(11_u64)) as jmethodID,
        Box::into_raw(Box::new(22_u64)) as jmethodID,
        Box::into_raw(Box::new(33_u64)) as jmethodID,
    ];

    assert_eq!(
        raw_values.len(),
        3,
        "test fixture should contain three independent raw method identifiers"
    );
    assert!(
        raw_values.iter().all(|raw| !raw.is_null()),
        "all allocated raw method identifiers should be non-null"
    );
    assert_ne!(
        raw_values[0], raw_values[1],
        "separate allocations should produce distinct method identifier pointers"
    );
    assert_ne!(
        raw_values[1], raw_values[2],
        "separate allocations should produce distinct method identifier pointers"
    );
    assert_ne!(
        raw_values[0], raw_values[2],
        "separate allocations should produce distinct method identifier pointers"
    );

    let wrapped: Vec<JMethodID> = raw_values
        .iter()
        .copied()
        .map(|raw| unsafe { JMethodID::from_raw(raw) })
        .collect();

    let unwrapped: Vec<jmethodID> = wrapped.into_iter().map(JMethodID::into_raw).collect();

    assert_eq!(
        unwrapped, raw_values,
        "each JMethodID should unwrap to the exact raw pointer originally used to construct it"
    );

    for raw in unwrapped {
        unsafe {
            drop(Box::from_raw(raw as *mut u64));
        }
    }
}