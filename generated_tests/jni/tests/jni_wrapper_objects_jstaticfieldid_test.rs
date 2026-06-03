use jni::objects::JStaticFieldID;
use jni::sys::jfieldID;
use std::panic;
use std::ptr;

#[test]
fn null_static_field_id_is_rejected_by_from_raw() {
    let raw: jfieldID = ptr::null_mut();

    let result = panic::catch_unwind(|| unsafe {
        JStaticFieldID::from_raw(raw);
    });

    assert!(
        result.is_err(),
        "JStaticFieldID::from_raw must reject a null jfieldID"
    );
    assert!(
        raw.is_null(),
        "the rejected raw jfieldID should still be the exact null pointer supplied"
    );
}

#[test]
fn non_null_static_field_id_pointer_identity_is_preserved_without_dereferencing() {
    let raw: jfieldID = 0x1usize as jfieldID;

    assert!(
        !raw.is_null(),
        "the synthetic raw jfieldID used by this test must be non-null"
    );

    let field_id: JStaticFieldID = unsafe { JStaticFieldID::from_raw(raw) };
    let round_tripped: jfieldID = JStaticFieldID::into_raw(field_id);

    assert!(
        !round_tripped.is_null(),
        "JStaticFieldID::into_raw should preserve non-nullness"
    );
    assert_eq!(
        round_tripped, raw,
        "JStaticFieldID should preserve the exact opaque JNI field-id pointer value"
    );
}

#[test]
fn multiple_static_field_ids_round_trip_independently_and_in_order() {
    let raw_ids: [jfieldID; 4] = [
        0x1usize as jfieldID,
        0x2usize as jfieldID,
        0x10usize as jfieldID,
        0x100usize as jfieldID,
    ];

    assert!(
        raw_ids.iter().all(|raw| !raw.is_null()),
        "JStaticFieldID::from_raw requires every jfieldID in this round-trip test to be non-null"
    );

    let wrapped_ids: Vec<JStaticFieldID> = raw_ids
        .iter()
        .copied()
        .map(|raw: jfieldID| unsafe { JStaticFieldID::from_raw(raw) })
        .collect();

    assert_eq!(
        wrapped_ids.len(),
        raw_ids.len(),
        "each non-null raw jfieldID should produce exactly one JStaticFieldID wrapper"
    );

    let round_tripped_ids: Vec<jfieldID> = wrapped_ids
        .into_iter()
        .map(JStaticFieldID::into_raw)
        .collect();

    assert_eq!(
        round_tripped_ids.len(),
        raw_ids.len(),
        "unwrapping should preserve the number of field IDs"
    );
    assert_eq!(
        round_tripped_ids, raw_ids,
        "wrapping and unwrapping several opaque static field IDs should preserve pointer identity and order"
    );
    assert!(
        round_tripped_ids.iter().all(|raw: &jfieldID| !raw.is_null()),
        "all synthetic non-null field IDs should remain non-null after the batch round trip"
    );
}