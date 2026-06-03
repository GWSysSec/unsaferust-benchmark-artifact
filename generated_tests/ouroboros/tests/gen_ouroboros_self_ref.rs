







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



    assert_eq!(holder.with_dref(|dref| **dref), 12);
    assert_eq!(**holder.borrow_dref(), 12);
    assert_eq!(*holder.borrow_data(), 12);

    let heads = holder.into_heads();
    assert_eq!(heads.data, 12);
}
