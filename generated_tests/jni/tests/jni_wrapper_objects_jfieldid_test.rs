use jni::objects::JFieldID;
use jni::sys::jfieldID;
use std::panic;
use std::ptr;

#[test]
fn null_jfieldid_is_rejected_by_from_raw() {
    let raw: jfieldID = ptr::null_mut();

    let result = panic::catch_unwind(|| unsafe {
        JFieldID::from_raw(raw);
    });

    assert!(
        result.is_err(),
        "JFieldID::from_raw(null) should panic because jfieldID must be non-null"
    );
}

#[test]
fn non_null_jfieldid_pointer_identity_is_preserved_without_dereferencing() {
    let raw: jfieldID = 0x1usize as jfieldID;

    assert!(
        !raw.is_null(),
        "the synthetic jfieldID used for identity testing should be non-null"
    );

    let field_id: JFieldID = unsafe { JFieldID::from_raw(raw) };
    let round_tripped: jfieldID = JFieldID::into_raw(field_id);

    assert!(
        !round_tripped.is_null(),
        "JFieldID::into_raw should preserve non-nullness for opaque JNI field IDs"
    );
    assert_eq!(
        round_tripped, raw,
        "JFieldID should preserve the exact opaque pointer value supplied by JNI"
    );
}

#[test]
fn multiple_jfieldids_can_be_wrapped_and_unwrapped_independently() {
    let raw_ids: [jfieldID; 4] = [
        0x1usize as jfieldID,
        0x2usize as jfieldID,
        0x10usize as jfieldID,
        0x20usize as jfieldID,
    ];

    for (index, raw) in raw_ids.iter().enumerate() {
        assert!(
            !raw.is_null(),
            "test jfieldID at index {index} must be non-null because JFieldID::from_raw rejects null"
        );
    }

    let round_tripped: Vec<jfieldID> = raw_ids
        .iter()
        .copied()
        .map(|raw| {
            let field_id: JFieldID = unsafe { JFieldID::from_raw(raw) };
            JFieldID::into_raw(field_id)
        })
        .collect();

    assert_eq!(
        round_tripped.len(),
        raw_ids.len(),
        "wrapping and unwrapping should produce one output for every input jfieldID"
    );

    for (index, (actual, expected)) in round_tripped.iter().zip(raw_ids.iter()).enumerate() {
        assert!(
            !actual.is_null(),
            "jfieldID at index {index} should remain non-null after round-trip"
        );
        assert_eq!(
            *actual, *expected,
            "jfieldID at index {index} should retain exact pointer identity"
        );
    }

    assert_ne!(
        round_tripped[0], round_tripped[1],
        "distinct opaque non-null jfieldID values should not be collapsed together"
    );
    assert_ne!(
        round_tripped[1], round_tripped[2],
        "later distinct opaque jfieldID values should also remain distinct"
    );
    assert_ne!(
        round_tripped[2], round_tripped[3],
        "all distinct opaque jfieldID values should remain independently identifiable"
    );
}