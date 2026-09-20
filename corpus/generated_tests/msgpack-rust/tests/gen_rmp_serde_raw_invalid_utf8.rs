//! Unsafe-oriented test for `msgpack-rust` (rmp-serde).
//!
//! `Raw` holds either a valid `String` or invalid UTF-8 bytes. Serializing the
//! invalid-bytes variant hits `unsafe { mem::transmute(&b[..]) }`
//! (rmp-serde/src/lib.rs:177), reinterpreting `&[u8]` as `&str`. The corpus
//! suite never constructs the invalid variant, so this path is unexercised.
//! NB: the transmute is a zero-cost fat-pointer reinterpret, so it registers as
//! an unsafe instruction (RQ3) but contributes ~0 unsafe cycles (RQ1).
use rmp_serde::Raw;

#[test]
fn serialize_invalid_utf8_raw_hits_transmute() {
    // Not valid UTF-8 -> `Raw` stores the `Err(bytes)` variant.
    let raw = Raw::from_utf8(vec![0xff, 0xfe, 0x00, 0x80]);
    assert!(raw.as_str().is_none());
    assert_eq!(raw.as_bytes(), &[0xff, 0xfe, 0x00, 0x80]);

    // Re-serializing routes through the `Serialize` impl's Err branch:
    //   Err((ref b, ..)) => unsafe { mem::transmute(&b[..]) }
    let encoded = rmp_serde::to_vec(&raw).expect("serialize Raw");
    assert!(!encoded.is_empty());
}
