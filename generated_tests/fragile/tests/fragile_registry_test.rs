use fragile::Fragile;

#[test]
fn fragile_tracks_validity_and_allows_roundtrip_access() {
    let mut value = Fragile::new(vec![1, 2, 3]);

    assert!(value.is_valid());
    assert_eq!(value.try_get().unwrap(), &vec![1, 2, 3]);

    value.get_mut().push(4);
    assert_eq!(value.get(), &vec![1, 2, 3, 4]);

    let inner = value.try_into_inner().unwrap();
    assert_eq!(inner, vec![1, 2, 3, 4]);
}