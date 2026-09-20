#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

// The target API `borsh::nostd_io::Error` with its specific signatures
// (`get_ref -> Option<&str>`, `into_inner -> Option<String>`) is the
// no_std I/O shim that borsh exposes when the `std` feature is disabled.
// These tests exercise the full constructor + inspection + consumption
// lifecycle of that error type.
#[cfg(not(feature = "std"))]
mod no_std_io_tests {
    use alloc::string::{String, ToString};
    use borsh::nostd_io::{Error, ErrorKind};

    #[test]
    fn test_error_new_with_str_literal_roundtrip() {
        let err = Error::new(ErrorKind::InvalidData, "invalid bytes detected");

        // pre-consume state
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_ne!(err.kind(), ErrorKind::UnexpectedEof);
        assert_ne!(err.kind(), ErrorKind::Other);

        let borrowed = err.get_ref();
        assert!(borrowed.is_some());
        assert_eq!(borrowed, Some("invalid bytes detected"));
        assert_eq!(borrowed.unwrap().len(), 22);

        // consume
        let inner = err.into_inner();
        assert!(inner.is_some());
        let s = inner.unwrap();
        assert_eq!(s.as_str(), "invalid bytes detected");
        assert_eq!(s.len(), 22);
        assert_ne!(s.as_str(), "");
    }

    #[test]
    fn test_error_new_with_owned_string_preserves_payload() {
        let message = String::from("premature end of stream");
        let original_len = message.len();

        let err = Error::new(ErrorKind::UnexpectedEof, message.clone());

        assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
        assert_ne!(err.kind(), ErrorKind::InvalidInput);
        assert_ne!(err.kind(), ErrorKind::InvalidData);

        let borrowed = err.get_ref();
        assert!(borrowed.is_some());
        assert_eq!(borrowed.unwrap(), message.as_str());
        assert_eq!(borrowed.unwrap().len(), original_len);

        let inner = err.into_inner();
        assert!(inner.is_some());
        let s = inner.unwrap();
        assert_eq!(s, message);
        assert_eq!(s.len(), original_len);
    }

    #[test]
    fn test_error_underscore_new_direct_string_construction() {
        let msg = String::from("low level");
        let err = Error::_new(ErrorKind::InvalidInput, msg.clone());

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_ne!(err.kind(), ErrorKind::Other);
        assert_ne!(err.kind(), ErrorKind::InvalidData);

        assert_eq!(err.get_ref(), Some(msg.as_str()));
        assert_eq!(err.get_ref().map(|s| s.len()), Some(9));
        assert!(err.get_ref().is_some());

        let consumed = err.into_inner();
        assert!(consumed.is_some());
        let s = consumed.unwrap();
        assert_eq!(s.as_str(), "low level");
        assert_eq!(s, msg);
        assert_ne!(s.as_str(), "");
    }

    #[test]
    fn test_error_underscore_new_empty_and_large_payloads() {
        // empty payload
        let e_empty = Error::_new(ErrorKind::Other, String::new());
        assert_eq!(e_empty.kind(), ErrorKind::Other);
        assert_ne!(e_empty.kind(), ErrorKind::InvalidData);
        assert_eq!(e_empty.get_ref(), Some(""));
        assert_eq!(e_empty.get_ref().unwrap().len(), 0);
        let i_empty = e_empty.into_inner();
        assert!(i_empty.is_some());
        let s_empty = i_empty.unwrap();
        assert_eq!(s_empty.len(), 0);
        assert_eq!(s_empty.as_str(), "");

        // large payload (boundary stress for the internal String storage)
        let long: String = core::iter::repeat('x').take(4096).collect();
        assert_eq!(long.len(), 4096);
        let e_long = Error::_new(ErrorKind::InvalidData, long.clone());
        assert_eq!(e_long.kind(), ErrorKind::InvalidData);
        assert_eq!(e_long.get_ref().map(|s| s.len()), Some(4096));
        let i_long = e_long.into_inner();
        assert!(i_long.is_some());
        let s_long = i_long.unwrap();
        assert_eq!(s_long.len(), 4096);
        assert_eq!(s_long, long);
    }

    #[test]
    fn test_error_kind_distinct_across_constructors_and_messages() {
        let a = Error::new(ErrorKind::InvalidData, "a");
        let b = Error::new(ErrorKind::UnexpectedEof, "bb");
        let c = Error::_new(ErrorKind::InvalidInput, "ccc".to_string());
        let d = Error::_new(ErrorKind::Other, "dddd".to_string());

        // kinds match their constructors
        assert_eq!(a.kind(), ErrorKind::InvalidData);
        assert_eq!(b.kind(), ErrorKind::UnexpectedEof);
        assert_eq!(c.kind(), ErrorKind::InvalidInput);
        assert_eq!(d.kind(), ErrorKind::Other);

        // kinds are pairwise distinct
        assert_ne!(a.kind(), b.kind());
        assert_ne!(b.kind(), c.kind());
        assert_ne!(c.kind(), d.kind());
        assert_ne!(a.kind(), d.kind());

        // get_ref returns exactly the message supplied at construction
        assert_eq!(a.get_ref(), Some("a"));
        assert_eq!(b.get_ref(), Some("bb"));
        assert_eq!(c.get_ref(), Some("ccc"));
        assert_eq!(d.get_ref(), Some("dddd"));

        // into_inner yields the same data, usable as an owned String
        let sa = a.into_inner().unwrap();
        let sb = b.into_inner().unwrap();
        let sc = c.into_inner().unwrap();
        let sd = d.into_inner().unwrap();
        assert_eq!(sa.len(), 1);
        assert_eq!(sb.len(), 2);
        assert_eq!(sc.len(), 3);
        assert_eq!(sd.len(), 4);
        assert_ne!(sa, sb);
        assert_ne!(sc, sd);
    }

    #[test]
    fn test_error_new_generic_accepts_tostring_types() {
        // `Error::new` is generic; it should accept owned `String` values.
        let owned = "dynamic message".to_string();
        let err = Error::new(ErrorKind::Interrupted, owned);

        assert_eq!(err.kind(), ErrorKind::Interrupted);
        assert_ne!(err.kind(), ErrorKind::WriteZero);
        assert!(err.get_ref().is_some());
        assert_eq!(err.get_ref().unwrap(), "dynamic message");
        assert_eq!(err.get_ref().unwrap().len(), 15);

        let reclaimed = err.into_inner();
        assert!(reclaimed.is_some());
        let s = reclaimed.unwrap();
        assert_eq!(s, "dynamic message");
        assert_eq!(s.len(), 15);
        assert_ne!(s.len(), 0);
    }
}