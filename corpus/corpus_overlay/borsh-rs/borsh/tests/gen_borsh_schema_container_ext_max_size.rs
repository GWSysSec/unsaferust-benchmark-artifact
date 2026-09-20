#![cfg(all(feature = "unstable__schema", feature = "std"))]

use borsh::schema::container_ext::max_size::{
    is_zero_size, is_zero_size_impl, max_serialized_size_impl,
};
use borsh::schema::BorshSchemaContainer;
use core::num::NonZeroUsize;

#[test]
fn test_max_serialized_size_impl_for_fixed_primitives() {
    let schema_u32 = BorshSchemaContainer::for_type::<u32>();
    let decl_u32: &str = schema_u32.declaration().as_str();

    let mut stack1: Vec<&str> = Vec::new();
    assert_eq!(stack1.len(), 0);
    let one = NonZeroUsize::new(1).unwrap();
    let size1 = max_serialized_size_impl(one, decl_u32, &schema_u32, &mut stack1);
    assert!(size1.is_ok());
    assert_eq!(size1.unwrap(), 4);

    // count = 5 → 20 bytes
    let mut stack2: Vec<&str> = Vec::new();
    let five = NonZeroUsize::new(5).unwrap();
    let size5 = max_serialized_size_impl(five, decl_u32, &schema_u32, &mut stack2);
    assert!(size5.is_ok());
    assert_eq!(size5.unwrap(), 20);

    // u64 schema
    let schema_u64 = BorshSchemaContainer::for_type::<u64>();
    let decl_u64: &str = schema_u64.declaration().as_str();
    let mut stack3: Vec<&str> = Vec::new();
    let size_u64 = max_serialized_size_impl(one, decl_u64, &schema_u64, &mut stack3);
    assert!(size_u64.is_ok());
    assert_eq!(size_u64.unwrap(), 8);

    // u8 array of 16
    let schema_arr = BorshSchemaContainer::for_type::<[u8; 16]>();
    let decl_arr: &str = schema_arr.declaration().as_str();
    let mut stack4: Vec<&str> = Vec::new();
    let size_arr = max_serialized_size_impl(one, decl_arr, &schema_arr, &mut stack4);
    assert!(size_arr.is_ok());
    assert_eq!(size_arr.unwrap(), 16);
}

#[test]
fn test_max_serialized_size_impl_unbounded_errors() {
    // Vec<u8> has no upper bound on serialized size — must yield an error.
    let schema_vec = BorshSchemaContainer::for_type::<Vec<u8>>();
    let decl_vec: &str = schema_vec.declaration().as_str();
    let mut stack: Vec<&str> = Vec::new();
    let one = NonZeroUsize::new(1).unwrap();
    let r = max_serialized_size_impl(one, decl_vec, &schema_vec, &mut stack);
    assert!(r.is_err());

    // String is similarly unbounded.
    let schema_str = BorshSchemaContainer::for_type::<String>();
    let decl_str: &str = schema_str.declaration().as_str();
    let mut stack2: Vec<&str> = Vec::new();
    let r_str = max_serialized_size_impl(one, decl_str, &schema_str, &mut stack2);
    assert!(r_str.is_err());

    // Unknown declaration in a known schema should error too.
    let schema_u32 = BorshSchemaContainer::for_type::<u32>();
    let mut stack3: Vec<&str> = Vec::new();
    let r_unknown = max_serialized_size_impl(one, "NoSuchType", &schema_u32, &mut stack3);
    assert!(r_unknown.is_err());

    // Sanity: the known declaration in the same schema still works.
    let mut stack4: Vec<&str> = Vec::new();
    let decl_u32: &str = schema_u32.declaration().as_str();
    let ok = max_serialized_size_impl(one, decl_u32, &schema_u32, &mut stack4);
    assert!(ok.is_ok());
    assert_eq!(ok.unwrap(), 4);
}

#[test]
fn test_is_zero_size_basic() {
    // u32 is not zero-sized.
    let schema_u32 = BorshSchemaContainer::for_type::<u32>();
    let decl_u32 = schema_u32.declaration().clone();
    let r = is_zero_size(&decl_u32, &schema_u32);
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), false);

    // u8
    let schema_u8 = BorshSchemaContainer::for_type::<u8>();
    let decl_u8 = schema_u8.declaration().clone();
    let r2 = is_zero_size(&decl_u8, &schema_u8);
    assert!(r2.is_ok());
    assert_eq!(r2.unwrap(), false);

    // Unit type () IS zero-sized.
    let schema_unit = BorshSchemaContainer::for_type::<()>();
    let decl_unit = schema_unit.declaration().clone();
    let r3 = is_zero_size(&decl_unit, &schema_unit);
    assert!(r3.is_ok());
    assert_eq!(r3.unwrap(), true);

    // Array [u8; 0] should also be zero-sized.
    let schema_arr0 = BorshSchemaContainer::for_type::<[u8; 0]>();
    let decl_arr0 = schema_arr0.declaration().clone();
    let r4 = is_zero_size(&decl_arr0, &schema_arr0);
    assert!(r4.is_ok());
    assert_eq!(r4.unwrap(), true);
}

#[test]
fn test_is_zero_size_impl_with_stack_threading() {
    // For a non-zero-sized primitive, is_zero_size_impl returns false and
    // leaves the stack empty after the call returns successfully.
    let schema = BorshSchemaContainer::for_type::<u32>();
    let decl: &str = schema.declaration().as_str();
    let mut stack: Vec<&str> = Vec::new();
    assert_eq!(stack.len(), 0);
    assert!(stack.is_empty());

    let r = is_zero_size_impl(decl, &schema, &mut stack);
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), false);
    assert_eq!(stack.len(), 0);

    // Unit type via _impl: should be zero-sized.
    let schema_unit = BorshSchemaContainer::for_type::<()>();
    let decl_unit: &str = schema_unit.declaration().as_str();
    let mut stack2: Vec<&str> = Vec::new();
    let r2 = is_zero_size_impl(decl_unit, &schema_unit, &mut stack2);
    assert!(r2.is_ok());
    assert_eq!(r2.unwrap(), true);
    assert_eq!(stack2.len(), 0);

    // Unknown declaration in a real schema yields an error.
    let mut stack3: Vec<&str> = Vec::new();
    let r3 = is_zero_size_impl("DoesNotExistName_qq", &schema, &mut stack3);
    assert!(r3.is_err());
}