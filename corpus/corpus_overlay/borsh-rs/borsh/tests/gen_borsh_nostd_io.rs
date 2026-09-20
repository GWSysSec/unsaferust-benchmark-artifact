#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(all(not(feature = "std"), feature = "std"))]
use alloc::vec;
#[cfg(all(not(feature = "std"), feature = "std"))]
use alloc::vec::Vec;

use borsh::io::{ErrorKind, Read};

fn _assert_error_is_sync_send() {
    // The original function is not publicly accessible since nostd_io is cfg'd out.
    // We replicate its intent: assert Error is Sync + Send.
    fn _assert<T: Sync + Send>() {}
    _assert::<borsh::io::Error>();
}

fn default_read_exact<R: Read>(reader: &mut R, buf: &mut [u8]) -> borsh::io::Result<()> {
    reader.read_exact(buf)
}

#[test]
fn test_assert_error_is_sync_send_callable_returns_unit() {
    let r1: () = _assert_error_is_sync_send();
    let r2: () = _assert_error_is_sync_send();
    assert_eq!(r1, ());
    assert_eq!(r2, ());
    assert_eq!(r1, r2);

    let mut counter: usize = 0;
    for _ in 0..10 {
        _assert_error_is_sync_send();
        counter += 1;
    }
    assert_eq!(counter, 10);
    assert_ne!(counter, 0);
    assert!(counter > 5);
    assert!(counter < 100);
}

#[test]
fn test_default_read_exact_full_read_from_slice() {
    let data: [u8; 10] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    let mut slice: &[u8] = &data;
    assert_eq!(slice.len(), 10);
    assert_eq!(slice[0], 10);

    let mut buf = [0u8; 5];
    assert_eq!(buf, [0, 0, 0, 0, 0]);

    let result = default_read_exact(&mut slice, &mut buf);
    assert!(result.is_ok());
    assert_eq!(buf[0], 10);
    assert_eq!(buf[1], 20);
    assert_eq!(buf[2], 30);
    assert_eq!(buf[3], 40);
    assert_eq!(buf[4], 50);
    assert_eq!(slice.len(), 5);
    assert_eq!(slice[0], 60);
    assert_ne!(buf, [0u8; 5]);
}

#[test]
fn test_default_read_exact_sequential_chunks() {
    let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut slice: &[u8] = &data;
    let start_len = slice.len();
    assert_eq!(start_len, 8);

    let mut buf1 = [0u8; 3];
    let r1 = default_read_exact(&mut slice, &mut buf1);
    assert!(r1.is_ok());
    assert_eq!(buf1, [1, 2, 3]);
    assert_eq!(slice.len(), 5);

    let mut buf2 = [0u8; 3];
    let r2 = default_read_exact(&mut slice, &mut buf2);
    assert!(r2.is_ok());
    assert_eq!(buf2, [4, 5, 6]);
    assert_eq!(slice.len(), 2);

    let mut buf3 = [0u8; 2];
    let r3 = default_read_exact(&mut slice, &mut buf3);
    assert!(r3.is_ok());
    assert_eq!(buf3, [7, 8]);
    assert_eq!(slice.len(), 0);
    assert_ne!(buf1, buf2);
    assert_ne!(buf2[0], buf3[0]);
    assert_eq!(buf1[0] + buf3[1], 9);
}

#[test]
fn test_default_read_exact_unexpected_eof_on_short_source() {
    let data: [u8; 3] = [100, 200, 250];
    let mut slice: &[u8] = &data;
    let initial_len = slice.len();
    assert_eq!(initial_len, 3);
    assert_eq!(slice[0], 100);
    assert_eq!(slice[2], 250);

    let mut buf = [0u8; 5];
    assert_eq!(buf.len(), 5);
    let result = default_read_exact(&mut slice, &mut buf);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
    assert_ne!(err.kind(), ErrorKind::Other);
    assert!(slice.len() <= initial_len);
}

#[test]
fn test_default_read_exact_empty_buffer_is_noop() {
    let data: [u8; 4] = [9, 8, 7, 6];
    let mut slice: &[u8] = &data;
    let initial = slice.len();
    assert_eq!(initial, 4);

    let mut buf: [u8; 0] = [];
    assert_eq!(buf.len(), 0);

    let result = default_read_exact(&mut slice, &mut buf);
    assert!(result.is_ok());
    assert_eq!(slice.len(), 4);
    assert_eq!(slice[0], 9);
    assert_eq!(slice[1], 8);
    assert_eq!(slice[2], 7);
    assert_eq!(slice[3], 6);
    assert_eq!(initial, slice.len());
}

#[test]
fn test_default_read_exact_exhaustion_then_eof_error() {
    let data: [u8; 4] = [11, 22, 33, 44];
    let mut slice: &[u8] = &data;
    assert_eq!(slice.len(), 4);

    let mut buf1 = [0u8; 4];
    let r1 = default_read_exact(&mut slice, &mut buf1);
    assert!(r1.is_ok());
    assert_eq!(buf1[0], 11);
    assert_eq!(buf1[1], 22);
    assert_eq!(buf1[2], 33);
    assert_eq!(buf1[3], 44);
    assert_eq!(slice.len(), 0);

    let mut buf2 = [0u8; 1];
    let r2 = default_read_exact(&mut slice, &mut buf2);
    assert!(r2.is_err());
    let err = r2.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
    assert_eq!(buf2[0], 0);
    assert_eq!(slice.len(), 0);
}

#[test]
fn test_default_read_exact_exact_fit() {
    let data: [u8; 6] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    let mut slice: &[u8] = &data;
    assert_eq!(slice.len(), 6);

    let mut buf = [0u8; 6];
    let r = default_read_exact(&mut slice, &mut buf);
    assert!(r.is_ok());
    assert_eq!(buf[0], 0xAA);
    assert_eq!(buf[1], 0xBB);
    assert_eq!(buf[2], 0xCC);
    assert_eq!(buf[3], 0xDD);
    assert_eq!(buf[4], 0xEE);
    assert_eq!(buf[5], 0xFF);
    assert_eq!(slice.len(), 0);
    assert_ne!(buf, [0u8; 6]);
}

#[cfg(feature = "std")]
#[test]
fn test_default_read_exact_large_buffer_boundary() {
    let size: usize = 64 * 1024;
    let data: std::vec::Vec<u8> = (0..size).map(|i| (i & 0xff) as u8).collect();
    assert_eq!(data.len(), size);
    assert_eq!(data[0], 0);
    assert_eq!(data[255], 255);

    let mut slice: &[u8] = &data;
    let mut buf: std::vec::Vec<u8> = std::vec![0u8; size];
    assert_eq!(buf.len(), size);
    assert_eq!(buf[0], 0);

    let result = default_read_exact(&mut slice, &mut buf);
    assert!(result.is_ok());
    assert_eq!(buf[0], 0);
    assert_eq!(buf[255], 255);
    assert_eq!(buf[256], 0);
    assert_eq!(buf[size - 1], ((size - 1) & 0xff) as u8);
    assert_eq!(slice.len(), 0);
    assert_eq!(&buf[..], &data[..]);
}