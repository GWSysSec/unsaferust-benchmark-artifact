







use rmp_serde::Raw;

#[test]
fn serialize_invalid_utf8_raw_hits_transmute() {

    let raw = Raw::from_utf8(vec![0xff, 0xfe, 0x00, 0x80]);
    assert!(raw.as_str().is_none());
    assert_eq!(raw.as_bytes(), &[0xff, 0xfe, 0x00, 0x80]);



    let encoded = rmp_serde::to_vec(&raw).expect("serialize Raw");
    assert!(!encoded.is_empty());
}
