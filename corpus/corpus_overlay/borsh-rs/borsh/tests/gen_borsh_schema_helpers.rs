#![cfg(all(feature = "unstable__schema", feature = "std"))]

use borsh::schema::BorshSchemaContainer;
use borsh::{
    max_serialized_size, schema_container_of, try_from_slice_with_schema, try_to_vec_with_schema,
};

#[test]
fn test_schema_container_of_basic() {
    let c: BorshSchemaContainer = schema_container_of::<u32>();
    assert_eq!(c.declaration(), "u32");
    assert!(c.get_definition("u32").is_some());
    assert!(c.get_definition("nonexistent_type_xyz").is_none());

    let count = c.definitions().count();
    assert_ne!(count, 0);
    assert!(count >= 1);

    // Same call yields equivalent declaration.
    let c2 = schema_container_of::<u32>();
    assert_eq!(c.declaration(), c2.declaration());
    assert_eq!(c.definitions().count(), c2.definitions().count());

    // A different type produces a different declaration.
    let cs = schema_container_of::<String>();
    assert_ne!(cs.declaration(), c.declaration());
    assert!(cs.get_definition(cs.declaration().clone().as_str()).is_some());
}

#[test]
fn test_max_serialized_size_fixed_types() {
    let s_u8 = max_serialized_size::<u8>().unwrap();
    assert_eq!(s_u8, 1);

    let s_u16 = max_serialized_size::<u16>().unwrap();
    assert_eq!(s_u16, 2);

    let s_u32 = max_serialized_size::<u32>().unwrap();
    assert_eq!(s_u32, 4);

    let s_u64 = max_serialized_size::<u64>().unwrap();
    assert_eq!(s_u64, 8);

    let s_i64 = max_serialized_size::<i64>().unwrap();
    assert_eq!(s_i64, 8);

    let s_arr = max_serialized_size::<[u32; 4]>().unwrap();
    assert_eq!(s_arr, 16);

    // Variable-length types (Vec<u8>, String) must report an error because
    // their serialized size is unbounded.
    let s_vec = max_serialized_size::<Vec<u8>>();
    assert!(s_vec.is_err());

    let s_string = max_serialized_size::<String>();
    assert!(s_string.is_err());
}

#[test]
fn test_try_to_vec_with_schema_and_back_u32() {
    let value: u32 = 0xCAFEBABE;
    let bytes = try_to_vec_with_schema(&value).unwrap();
    assert_ne!(bytes.len(), 0);
    // Layout: [schema_container_serialized..., value_le_bytes]
    // Verify the trailing 4 bytes are the LE value.
    assert!(bytes.len() > 4);
    let tail = &bytes[bytes.len() - 4..];
    assert_eq!(tail, &[0xBE, 0xBA, 0xFE, 0xCA]);

    let restored: u32 = try_from_slice_with_schema(&bytes).unwrap();
    assert_eq!(restored, value);
    assert_ne!(restored, 0);

    // Round-trip with a different value yields different bytes.
    let other: u32 = 0;
    let other_bytes = try_to_vec_with_schema(&other).unwrap();
    assert_ne!(other_bytes, bytes);
    let other_back: u32 = try_from_slice_with_schema(&other_bytes).unwrap();
    assert_eq!(other_back, 0);
}

#[test]
fn test_try_with_schema_type_mismatch_and_strings() {
    // Round-trip a String through the schema-tagged helpers.
    let s = String::from("schema-tagged-roundtrip");
    let bytes = try_to_vec_with_schema(&s).unwrap();
    assert_ne!(bytes.len(), 0);
    assert!(bytes.len() > s.len());

    let back: String = try_from_slice_with_schema(&bytes).unwrap();
    assert_eq!(back, s);
    assert_eq!(back.len(), s.len());

    // Truncated buffer must fail to deserialize.
    let truncated = &bytes[..bytes.len() - 1];
    let res: Result<String, _> = try_from_slice_with_schema(truncated);
    assert!(res.is_err());

    // Garbage / empty buffer must fail.
    let empty_res: Result<String, _> = try_from_slice_with_schema(&[]);
    assert!(empty_res.is_err());

    // Bytes produced for u32 should NOT deserialize as String due to schema mismatch.
    let v: u32 = 7;
    let v_bytes = try_to_vec_with_schema(&v).unwrap();
    let mismatched: Result<String, _> = try_from_slice_with_schema(&v_bytes);
    assert!(mismatched.is_err());

    // But it should deserialize back as u32.
    let v_back: u32 = try_from_slice_with_schema(&v_bytes).unwrap();
    assert_eq!(v_back, 7);
}