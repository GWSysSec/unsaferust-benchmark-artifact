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

        // Initial position is 0
        assert_eq!(cursor.position(), 0);

        // Set position to 5
        cursor.set_position(5);
        assert_eq!(cursor.position(), 5);

        // Read from position 5
        let mut buf = [0u8; 3];
        let n = cursor.read(&mut buf).await.unwrap();
        assert_eq!(n, 3);
        assert_eq!(buf, [6, 7, 8]);

        // Position should have advanced by 3
        assert_eq!(cursor.position(), 8);

        // Set position back to 0
        cursor.set_position(0);
        assert_eq!(cursor.position(), 0);

        // Read from beginning
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

        // Verify initial state
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.get_ref().len(), 5);
        assert_eq!(cursor.get_ref()[0], 0);

        // Modify underlying data via get_mut
        {
            let inner = cursor.get_mut();
            inner[0] = 42;
            inner[1] = 99;
            inner[4] = 255;
        }

        // Verify modifications took effect
        assert_eq!(cursor.get_ref()[0], 42);
        assert_eq!(cursor.get_ref()[1], 99);
        assert_eq!(cursor.get_ref()[4], 255);

        // Position should be unchanged after get_mut
        assert_eq!(cursor.position(), 0);

        // Read the modified data
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

        // Set position beyond the end of data
        cursor.set_position(100);
        assert_eq!(cursor.position(), 100);

        // Reading should return 0 bytes (EOF)
        let mut buf = [0u8; 5];
        let n = cursor.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);

        // Position remains at 100 after failed read
        assert_eq!(cursor.position(), 100);

        // Set back to valid position
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

        // Initial state
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.get_ref().len(), 0);

        // Write some data
        cursor.write_all(&[10, 20, 30]).await.unwrap();
        assert_eq!(cursor.position(), 3);
        assert_eq!(cursor.get_ref().len(), 3);
        assert_eq!(cursor.get_ref()[0], 10);
        assert_eq!(cursor.get_ref()[1], 20);
        assert_eq!(cursor.get_ref()[2], 30);

        // Use get_mut to extend the buffer
        {
            let inner = cursor.get_mut();
            inner.push(40);
            inner.push(50);
        }

        // Verify the extension
        assert_eq!(cursor.get_ref().len(), 5);
        assert_eq!(cursor.get_ref()[3], 40);
        assert_eq!(cursor.get_ref()[4], 50);

        // Position is still at 3 (write position)
        assert_eq!(cursor.position(), 3);

        // Read from current position should get the appended data
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

        // Jump around and verify reads
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

        // After reading last byte, position is at end
        assert_eq!(cursor.position(), 256);

        // Reading at end gives 0 bytes
        let n = cursor.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);

        // Jump back to middle
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

        // Read some data
        let mut buf = [0u8; 3];
        cursor.read(&mut buf).await.unwrap();
        assert_eq!(buf, [1, 2, 3]);
        assert_eq!(cursor.position(), 3);

        // Clear the underlying buffer via get_mut
        {
            let inner = cursor.get_mut();
            inner.clear();
            inner.extend_from_slice(&[100, 101, 102, 103]);
        }

        // Position is still 3 but data changed
        assert_eq!(cursor.position(), 3);
        assert_eq!(cursor.get_ref().len(), 4);

        // Read from position 3 in new data
        let mut buf2 = [0u8; 2];
        let n = cursor.read(&mut buf2).await.unwrap();
        assert_eq!(n, 1); // only one byte available at position 3
        assert_eq!(buf2[0], 103);

        // Reset position and read all new data
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

        // Write at beginning
        cursor.write_all(&[1, 2, 3]).await.unwrap();
        assert_eq!(cursor.position(), 3);
        assert_eq!(cursor.get_ref()[0], 1);
        assert_eq!(cursor.get_ref()[1], 2);
        assert_eq!(cursor.get_ref()[2], 3);
        // Rest should still be zeros
        assert_eq!(cursor.get_ref()[3], 0);

        // Set position to middle and overwrite
        cursor.set_position(5);
        cursor.write_all(&[55, 66]).await.unwrap();
        assert_eq!(cursor.position(), 7);
        assert_eq!(cursor.get_ref()[5], 55);
        assert_eq!(cursor.get_ref()[6], 66);

        // Verify untouched areas
        assert_eq!(cursor.get_ref()[4], 0);
        assert_eq!(cursor.get_ref()[7], 0);

        // Use get_mut to verify full state
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

        // Read everything
        let mut result = Vec::new();
        let bytes_read = async_std::io::copy(&mut cursor, &mut result).await.unwrap();
        assert_eq!(bytes_read as usize, expected_len);
        assert_eq!(cursor.position() as usize, expected_len);
        assert_eq!(&result, b"Hello, async-std cursor!");

        // Reset and read again
        cursor.set_position(0);
        assert_eq!(cursor.position(), 0);

        let mut result2 = Vec::new();
        let bytes_read2 = async_std::io::copy(&mut cursor, &mut result2).await.unwrap();
        assert_eq!(bytes_read2 as usize, expected_len);
        assert_eq!(result2, result);
        assert_eq!(cursor.position() as usize, expected_len);
    });
}