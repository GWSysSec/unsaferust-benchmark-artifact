use async_std::io::Cursor;
use async_std::prelude::*;
use async_std::task;

#[test]
fn test_cursor_position_initial_is_zero() {
    task::block_on(async {
        let data = vec![10u8, 20, 30, 40, 50];
        let cursor = Cursor::new(data);
        assert_eq!(cursor.position(), 0);
    });
}

#[test]
fn test_cursor_set_position_and_read() {
    task::block_on(async {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut cursor = Cursor::new(data.clone());


        assert_eq!(cursor.position(), 0);


        cursor.set_position(5);
        assert_eq!(cursor.position(), 5);


        let mut buf = [0u8; 3];
        let n = cursor.read(&mut buf).await.unwrap();
        assert_eq!(n, 3);
        assert_eq!(buf, [6, 7, 8]);


        assert_eq!(cursor.position(), 8);


        cursor.set_position(0);
        assert_eq!(cursor.position(), 0);


        let mut buf2 = [0u8; 2];
        let n2 = cursor.read(&mut buf2).await.unwrap();
        assert_eq!(n2, 2);
        assert_eq!(buf2, [1, 2]);
        assert_eq!(cursor.position(), 2);
    });
}

#[test]
fn test_cursor_get_mut_modify_underlying_data() {
    task::block_on(async {
        let data = vec![0u8, 0, 0, 0, 0];
        let mut cursor = Cursor::new(data);


        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.get_ref().len(), 5);
        assert_eq!(cursor.get_ref()[0], 0);


        {
            let inner = cursor.get_mut();
            inner[0] = 42;
            inner[1] = 99;
            inner[4] = 255;
        }


        assert_eq!(cursor.get_ref()[0], 42);
        assert_eq!(cursor.get_ref()[1], 99);
        assert_eq!(cursor.get_ref()[4], 255);


        assert_eq!(cursor.position(), 0);


        let mut buf = [0u8; 5];
        let n = cursor.read(&mut buf).await.unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [42, 99, 0, 0, 255]);
    });
}

#[test]
fn test_cursor_set_position_beyond_end() {
    task::block_on(async {
        let data = vec![1u8, 2, 3];
        let mut cursor = Cursor::new(data);


        cursor.set_position(100);
        assert_eq!(cursor.position(), 100);


        let mut buf = [0u8; 5];
        let n = cursor.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);


        assert_eq!(cursor.position(), 100);


        cursor.set_position(1);
        assert_eq!(cursor.position(), 1);

        let mut buf2 = [0u8; 2];
        let n2 = cursor.read(&mut buf2).await.unwrap();
        assert_eq!(n2, 2);
        assert_eq!(buf2, [2, 3]);
    });
}

#[test]
fn test_cursor_write_and_get_mut_interaction() {
    task::block_on(async {
        let mut cursor = Cursor::new(Vec::<u8>::new());


        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.get_ref().len(), 0);


        cursor.write_all(&[10, 20, 30]).await.unwrap();
        assert_eq!(cursor.position(), 3);
        assert_eq!(cursor.get_ref().len(), 3);
        assert_eq!(cursor.get_ref()[0], 10);
        assert_eq!(cursor.get_ref()[1], 20);
        assert_eq!(cursor.get_ref()[2], 30);


        {
            let inner = cursor.get_mut();
            inner.push(40);
            inner.push(50);
        }


        assert_eq!(cursor.get_ref().len(), 5);
        assert_eq!(cursor.get_ref()[3], 40);
        assert_eq!(cursor.get_ref()[4], 50);


        assert_eq!(cursor.position(), 3);


        let mut buf = [0u8; 2];
        let n = cursor.read(&mut buf).await.unwrap();
        assert_eq!(n, 2);
        assert_eq!(buf, [40, 50]);
        assert_eq!(cursor.position(), 5);
    });
}

#[test]
fn test_cursor_multiple_set_position_cycles() {
    task::block_on(async {
        let data: Vec<u8> = (0..=255).collect();
        let mut cursor = Cursor::new(data);


        cursor.set_position(0);
        assert_eq!(cursor.position(), 0);
        let mut buf = [0u8; 1];
        cursor.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], 0);

        cursor.set_position(127);
        assert_eq!(cursor.position(), 127);
        cursor.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], 127);

        cursor.set_position(255);
        assert_eq!(cursor.position(), 255);
        cursor.read(&mut buf).await.unwrap();
        assert_eq!(buf[0], 255);


        assert_eq!(cursor.position(), 256);


        let n = cursor.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);


        cursor.set_position(128);
        let mut buf4 = [0u8; 4];
        let n = cursor.read(&mut buf4).await.unwrap();
        assert_eq!(n, 4);
        assert_eq!(buf4, [128, 129, 130, 131]);
        assert_eq!(cursor.position(), 132);
    });
}

#[test]
fn test_cursor_get_mut_clear_and_reuse() {
    task::block_on(async {
        let mut cursor = Cursor::new(vec![1u8, 2, 3, 4, 5]);


        let mut buf = [0u8; 3];
        cursor.read(&mut buf).await.unwrap();
        assert_eq!(buf, [1, 2, 3]);
        assert_eq!(cursor.position(), 3);


        {
            let inner = cursor.get_mut();
            inner.clear();
            inner.extend_from_slice(&[100, 101, 102, 103]);
        }


        assert_eq!(cursor.position(), 3);
        assert_eq!(cursor.get_ref().len(), 4);


        let mut buf2 = [0u8; 2];
        let n = cursor.read(&mut buf2).await.unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf2[0], 103);


        cursor.set_position(0);
        let mut all = [0u8; 4];
        let n = cursor.read(&mut all).await.unwrap();
        assert_eq!(n, 4);
        assert_eq!(all, [100, 101, 102, 103]);
    });
}

#[test]
fn test_cursor_position_after_write_overwrite() {
    task::block_on(async {
        let mut cursor = Cursor::new(vec![0u8; 10]);

        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.get_ref().len(), 10);


        cursor.write_all(&[1, 2, 3]).await.unwrap();
        assert_eq!(cursor.position(), 3);
        assert_eq!(cursor.get_ref()[0], 1);
        assert_eq!(cursor.get_ref()[1], 2);
        assert_eq!(cursor.get_ref()[2], 3);

        assert_eq!(cursor.get_ref()[3], 0);


        cursor.set_position(5);
        cursor.write_all(&[55, 66]).await.unwrap();
        assert_eq!(cursor.position(), 7);
        assert_eq!(cursor.get_ref()[5], 55);
        assert_eq!(cursor.get_ref()[6], 66);


        assert_eq!(cursor.get_ref()[4], 0);
        assert_eq!(cursor.get_ref()[7], 0);


        let inner = cursor.get_mut();
        assert_eq!(inner.len(), 10);
        assert_eq!(&inner[..], &[1, 2, 3, 0, 0, 55, 66, 0, 0, 0]);
    });
}

#[test]
fn test_cursor_set_position_zero_after_full_read() {
    task::block_on(async {
        let data = b"Hello, async-std cursor!".to_vec();
        let expected_len = data.len();
        let mut cursor = Cursor::new(data);


        let mut result = Vec::new();
        let bytes_read = async_std::io::copy(&mut cursor, &mut result).await.unwrap();
        assert_eq!(bytes_read as usize, expected_len);
        assert_eq!(cursor.position() as usize, expected_len);
        assert_eq!(&result, b"Hello, async-std cursor!");


        cursor.set_position(0);
        assert_eq!(cursor.position(), 0);

        let mut result2 = Vec::new();
        let bytes_read2 = async_std::io::copy(&mut cursor, &mut result2).await.unwrap();
        assert_eq!(bytes_read2 as usize, expected_len);
        assert_eq!(result2, result);
        assert_eq!(cursor.position() as usize, expected_len);
    });
}