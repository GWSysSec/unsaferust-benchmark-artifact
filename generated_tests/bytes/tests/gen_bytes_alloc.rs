






use bytes::{Buf, Bytes};

#[test]
fn gen_bytes_from_vec_round_trip() {
    for &n in &[1usize, 7, 16, 17, 64, 1024, 4096] {
        let vec: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
        let b = Bytes::from(vec.clone());
        assert_eq!(b.len(), n);
        assert_eq!(&b[..], &vec[..]);
    }
}

#[test]
fn gen_bytes_from_empty_vec() {
    let b = Bytes::from(Vec::<u8>::new());
    assert_eq!(b.len(), 0);
    assert!(b.is_empty());
}

#[test]
fn gen_bytes_from_vec_drop_only() {
    let vec = vec![33u8; 1024];
    let _b = Bytes::from(vec);
}

#[test]
fn gen_bytes_from_vec_clone_promote_drop_orders() {
    let vec = vec![7u8; 4096];
    let b1 = Bytes::from(vec);
    let b2 = b1.clone();
    let b3 = b1.clone();
    assert_eq!(b1.len(), 4096);
    assert_eq!(b2.len(), 4096);
    assert_eq!(b3.len(), 4096);
    drop(b1);
    drop(b3);
    drop(b2);
}

#[test]
fn gen_bytes_from_vec_drop_original_first() {
    let b1 = Bytes::from(vec![1u8; 512]);
    let b2 = b1.clone();
    drop(b1);
    assert_eq!(b2.len(), 512);
}

#[test]
fn gen_bytes_advance_then_drop() {
    let mut b = Bytes::from(vec![10u8, 20, 30, 40, 50]);
    b.advance(1);
    assert_eq!(&b[..], &[20, 30, 40, 50]);
    drop(b);
}

#[test]
fn gen_bytes_truncate_then_drop() {
    let mut b = Bytes::from(vec![10u8, 20, 30, 40, 50]);
    b.truncate(2);
    assert_eq!(&b[..], &[10, 20]);
    drop(b);
}

#[test]
fn gen_bytes_truncate_then_advance() {
    let mut b = Bytes::from(vec![10u8, 20, 30, 40, 50]);
    b.truncate(4);
    b.advance(1);
    assert_eq!(&b[..], &[20, 30, 40]);
    drop(b);
}

#[test]
fn gen_bytes_split_off_drop() {
    let mut b = Bytes::from(vec![0u8; 256]);
    let tail = b.split_off(128);
    assert_eq!(b.len(), 128);
    assert_eq!(tail.len(), 128);
    drop(tail);
    drop(b);
}

#[test]
fn gen_bytes_split_to_drop() {
    let mut b = Bytes::from(vec![0u8; 256]);
    let head = b.split_to(64);
    assert_eq!(head.len(), 64);
    assert_eq!(b.len(), 192);
    drop(head);
    drop(b);
}

#[test]
fn gen_bytes_slice_of_promoted() {
    let b = Bytes::from(vec![9u8; 2048]);
    let c = b.clone();
    let s1 = b.slice(0..1024);
    let s2 = c.slice(1024..2048);
    assert_eq!(s1.len(), 1024);
    assert_eq!(s2.len(), 1024);
    drop(b);
    drop(c);
    drop(s1);
    drop(s2);
}

#[test]
fn gen_bytes_many_clones_stress() {
    let b = Bytes::from(vec![42u8; 8192]);
    let clones: Vec<Bytes> = (0..32).map(|_| b.clone()).collect();
    drop(b);
    for c in &clones {
        assert_eq!(c.len(), 8192);
    }
    drop(clones);
}

#[test]
fn gen_bytes_from_string() {
    let s = String::from("hello world repeated content ".repeat(64));
    let len = s.len();
    let b = Bytes::from(s);
    assert_eq!(b.len(), len);
}
