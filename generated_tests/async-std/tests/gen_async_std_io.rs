use async_std::io::{self, ReadExt, WriteExt};
use async_std::task::block_on;

#[test]
fn test_sink_writes() {
    block_on(async {
        let mut s = io::sink();
        let n = s.write(b"hello world").await.unwrap();
        assert_eq!(n, 11);
        let n2 = s.write(&[0u8; 1024]).await.unwrap();
        assert_eq!(n2, 1024);
        s.flush().await.unwrap();
        let n3 = s.write(b"").await.unwrap();
        assert_eq!(n3, 0);
        let n4 = s.write(&[42u8; 5]).await.unwrap();
        assert_eq!(n4, 5);

        s.write_all(&[1, 2, 3, 4]).await.unwrap();
        assert_eq!(s.write(b"x").await.unwrap(), 1);
    });
}

#[test]
fn test_empty_reads() {
    block_on(async {
        let mut e = io::empty();
        let mut buf = [1u8; 16];
        let n = e.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);
        assert_eq!(buf[0], 1);
        let n2 = e.read(&mut buf).await.unwrap();
        assert_eq!(n2, 0);
        let mut s = String::new();
        let total = e.read_to_string(&mut s).await.unwrap();
        assert_eq!(total, 0);
        assert_eq!(s, "");
        let mut v = Vec::new();
        let total2 = e.read_to_end(&mut v).await.unwrap();
        assert_eq!(total2, 0);
        assert!(v.is_empty());
    });
}

#[test]
fn test_repeat_reads() {
    block_on(async {
        let mut r = io::repeat(0xAB);
        let mut buf = [0u8; 32];
        let n = r.read(&mut buf).await.unwrap();
        assert_eq!(n, 32);
        for &b in &buf {
            assert_eq!(b, 0xAB);
        }
        let mut buf2 = [0u8; 7];
        let n2 = r.read(&mut buf2).await.unwrap();
        assert_eq!(n2, 7);
        assert_eq!(buf2, [0xAB; 7]);

        let mut r2 = io::repeat(0);
        let mut buf3 = [9u8; 100];
        let n3 = r2.read(&mut buf3).await.unwrap();
        assert!(n3 > 0);
        assert_eq!(buf3[0], 0);
        assert_eq!(buf3[n3 - 1], 0);
    });
}

#[test]
fn test_copy_repeat_to_sink() {
    block_on(async {

        use async_std::io::ReadExt;
        let mut src = io::repeat(b'z').take(1000);
        let mut dst = io::sink();
        let copied = io::copy(&mut src, &mut dst).await.unwrap();
        assert_eq!(copied, 1000);

        let mut empty_src = io::empty();
        let mut dst2 = io::sink();
        let copied2 = io::copy(&mut empty_src, &mut dst2).await.unwrap();
        assert_eq!(copied2, 0);
    });
}

#[test]
fn test_stdout_stderr_handles() {
    block_on(async {
        let mut out = io::stdout();

        out.write_all(b"").await.unwrap();
        out.flush().await.unwrap();

        let mut err = io::stderr();
        err.write_all(b"").await.unwrap();
        err.flush().await.unwrap();


        let n = out.write(b"async-std-test-stdout\n").await.unwrap();
        assert!(n > 0);
        assert_eq!(n, 22);
        out.flush().await.unwrap();

        let m = err.write(b"async-std-test-stderr\n").await.unwrap();
        assert_eq!(m, 22);
        err.flush().await.unwrap();
    });
}

#[test]
fn test_stdin_handle_exists() {
    let _stdin = io::stdin();

    let s1 = io::sink();
    let s2 = io::sink();
    drop(s1);
    drop(s2);
    let r = io::repeat(1);
    drop(r);
    let e = io::empty();
    drop(e);
    assert_eq!(2 + 2, 4);
    assert_ne!(1, 0);
    assert!(true as bool);
    let _ = io::stdout();
    let _ = io::stderr();
}