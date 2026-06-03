use scroll::ctx::StrCtx;
use scroll::{Endian, Pread};

#[test]
fn test_endian_network_returns_big_endian() {
    let net = Endian::network();

    assert_eq!(net, Endian::Big);
}

#[test]
fn test_endian_network_used_to_read_u32() {
    let bytes: [u8; 4] = [0x12, 0x34, 0x56, 0x78];
    let ctx = Endian::network();
    let v: u32 = bytes.pread_with(0, ctx).expect("read should succeed");
    assert_eq!(v, 0x12345678);
}

#[test]
fn test_endian_network_used_to_read_u16() {
    let bytes: [u8; 2] = [0xAB, 0xCD];
    let ctx = Endian::network();
    let v: u16 = bytes.pread_with(0, ctx).expect("read u16");
    assert_eq!(v, 0xABCD);
}

#[test]
fn test_endian_network_differs_from_little() {
    let bytes: [u8; 4] = [0x01, 0x00, 0x00, 0x00];
    let net = Endian::network();
    let le: u32 = bytes.pread_with(0, Endian::Little).expect("le");
    let be: u32 = bytes.pread_with(0, net).expect("be");
    assert_eq!(le, 1);
    assert_eq!(be, 0x01000000);
    assert_ne!(le, be);
}

#[test]
fn test_endian_network_multiple_invocations_consistent() {
    let a = Endian::network();
    let b = Endian::network();
    let c = Endian::network();
    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn test_endian_network_i64_roundtrip_via_pread() {

    let bytes: [u8; 8] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe];
    let ctx = Endian::network();
    let v: i64 = bytes.pread_with(0, ctx).expect("read i64");
    assert_eq!(v, -2);
}

#[test]
fn test_endian_network_with_strctx_unused_combo() {

    let net = Endian::network();
    let _ = format!("{:?}", net);
    let ctx = StrCtx::Length(4);
    match ctx {
        StrCtx::Length(n) => assert_eq!(n, 4),
        _ => panic!("expected Length variant"),
    }
    let bytes = b"abcd";
    let s: &str = bytes.pread_with(0, ctx).expect("str read");
    assert_eq!(s, "abcd");
}

#[test]
fn test_endian_network_error_path() {
    let bytes: [u8; 2] = [0x00, 0x01];
    let ctx = Endian::network();

    let result: Result<u32, scroll::Error> = bytes.pread_with(0, ctx);
    match result {
        Ok(_) => panic!("expected error reading u32 from 2-byte buffer"),
        Err(e) => {
            let msg = format!("{}", e);
            assert!(!msg.is_empty());
        }
    }
}