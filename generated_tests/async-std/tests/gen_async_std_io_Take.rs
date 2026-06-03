#![cfg(not(target_os = "unknown"))]

use async_std::io::{Cursor, SeekFrom};
use async_std::prelude::*;
use async_std::task;

#[test]
fn test_take_limit_matches_set_value() {
    task::block_on(async {
        let data: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let cursor = Cursor::new(data);
        let take = cursor.take(7);


        assert_eq!(take.limit(), 7u64);
        assert_ne!(take.limit(), 0u64);
        assert_ne!(take.limit(), 10u64);
        assert_ne!(take.limit(), 6u64);


        let cursor2 = Cursor::new(vec![0u8; 100]);
        let take2 = cursor2.take(0);
        assert_eq!(take2.limit(), 0u64);
        assert_ne!(take2.limit(), 1u64);


        let cursor3 = Cursor::new(vec![0u8; 8]);
        let take3 = cursor3.take(u64::MAX);
        assert_eq!(take3.limit(), u64::MAX);
        assert_ne!(take3.limit(), u64::MAX - 1);
        assert_ne!(take3.limit(), 0u64);
    });
}

#[test]
fn test_take_limit_decrements_with_reads() {
    task::block_on(async {
        let data = b"0123456789ABCDEF".to_vec();
        let cursor = Cursor::new(data);
        let mut take = cursor.take(10);


        assert_eq!(take.limit(), 10u64);


        let mut buf = [0u8; 3];
        let n = take.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 3usize);
        assert_eq!(&buf, b"012");
        assert_eq!(take.limit(), 7u64);
        assert_ne!(take.limit(), 10u64);


        let mut buf2 = [0u8; 4];
        let n2 = take.read(&mut buf2).await.expect("read failed");
        assert_eq!(n2, 4usize);
        assert_eq!(&buf2, b"3456");
        assert_eq!(take.limit(), 3u64);


        let mut buf3 = [0u8; 16];
        let n3 = take.read(&mut buf3).await.expect("read failed");
        assert_eq!(n3, 3usize);
        assert_eq!(&buf3[..3], b"789");
        assert_eq!(take.limit(), 0u64);


        let n4 = take.read(&mut buf3).await.expect("read failed");
        assert_eq!(n4, 0usize);
        assert_eq!(take.limit(), 0u64);
    });
}

#[test]
fn test_take_set_limit_controls_subsequent_reads() {
    task::block_on(async {
        let data = b"ABCDEFGHIJKLMNOP".to_vec();
        let cursor = Cursor::new(data);
        let mut take = cursor.take(4);

        assert_eq!(take.limit(), 4u64);


        let mut buf = [0u8; 4];
        take.read_exact(&mut buf).await.expect("read_exact failed");
        assert_eq!(&buf, b"ABCD");
        assert_eq!(take.limit(), 0u64);


        take.set_limit(5);
        assert_eq!(take.limit(), 5u64);
        assert_ne!(take.limit(), 0u64);
        assert_ne!(take.limit(), 4u64);


        let mut buf2 = [0u8; 5];
        take.read_exact(&mut buf2).await.expect("read_exact failed");
        assert_eq!(&buf2, b"EFGHI");
        assert_eq!(take.limit(), 0u64);


        take.set_limit(2);
        assert_eq!(take.limit(), 2u64);
        let mut buf3 = [0u8; 16];
        let n = take.read(&mut buf3).await.expect("read failed");
        assert_eq!(n, 2usize);
        assert_eq!(&buf3[..2], b"JK");
        assert_eq!(take.limit(), 0u64);


        take.set_limit(3);
        assert_eq!(take.limit(), 3u64);
        let mut buf4 = [0u8; 3];
        take.read_exact(&mut buf4).await.expect("read_exact failed");
        assert_eq!(&buf4, b"LMN");
        assert_eq!(take.limit(), 0u64);
    });
}

#[test]
fn test_take_get_mut_allows_inner_mutation() {
    task::block_on(async {
        let data = b"0123456789".to_vec();
        let cursor = Cursor::new(data);
        let mut take = cursor.take(3);

        assert_eq!(take.limit(), 3u64);


        let mut buf = [0u8; 3];
        take.read_exact(&mut buf).await.expect("read_exact failed");
        assert_eq!(&buf, b"012");
        assert_eq!(take.limit(), 0u64);


        {
            let inner: &mut Cursor<Vec<u8>> = take.get_mut();
            assert_eq!(inner.position(), 3u64);
            inner.seek(SeekFrom::Start(0)).await.expect("seek failed");
            assert_eq!(inner.position(), 0u64);
        }


        take.set_limit(5);
        assert_eq!(take.limit(), 5u64);

        let mut buf2 = [0u8; 5];
        take.read_exact(&mut buf2).await.expect("read_exact failed");
        assert_eq!(&buf2, b"01234");
        assert_eq!(take.limit(), 0u64);


        {
            let inner2: &mut Cursor<Vec<u8>> = take.get_mut();
            assert_eq!(inner2.position(), 5u64);

            inner2.set_position(8);
            assert_eq!(inner2.position(), 8u64);
        }


        take.set_limit(2);
        assert_eq!(take.limit(), 2u64);
        let mut buf3 = [0u8; 2];
        take.read_exact(&mut buf3).await.expect("read_exact failed");
        assert_eq!(&buf3, b"89");
        assert_eq!(take.limit(), 0u64);
    });
}