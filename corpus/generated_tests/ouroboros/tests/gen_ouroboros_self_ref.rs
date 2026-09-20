//! Unsafe-oriented test for `ouroboros`.
//!
//! The `#[self_referencing]` macro emits calls to `change_lifetime`
//! (`&*(data as *const _)`, a raw-pointer deref at ouroboros/src/lib.rs:391).
//! The library crate reports ~0 unsafe in the corpus because its real macro
//! usage lives in the unbenched `examples` workspace member. This in-crate test
//! builds and accesses a self-referential struct so the generated unsafe deref
//! actually executes.
use ouroboros::self_referencing;

#[self_referencing]
struct Holder {
    data: i32,
    #[borrows(data)]
    dref: &'this i32,
}

#[test]
fn self_referencing_accessors_exercise_change_lifetime() {
    let holder = HolderBuilder {
        data: 12,
        dref_builder: |data| data,
    }
    .build();

    // Each accessor routes the self-referential borrow through the
    // macro-generated `change_lifetime` unsafe pointer deref.
    assert_eq!(holder.with_dref(|dref| **dref), 12);
    assert_eq!(**holder.borrow_dref(), 12);
    assert_eq!(*holder.borrow_data(), 12);

    let heads = holder.into_heads();
    assert_eq!(heads.data, 12);
}
