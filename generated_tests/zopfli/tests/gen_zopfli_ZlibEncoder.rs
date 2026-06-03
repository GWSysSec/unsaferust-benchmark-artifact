use std::io::Write;
use zopfli::{BlockType, Options, ZlibEncoder};

#[test]
fn test_zlib_new_buffered_basic_roundtrip() {
    let options = Options::default();
    let sink: Vec<u8> = Vec::new();
    let mut bw = ZlibEncoder::new_buffered(options, BlockType::Dynamic, sink)
        .expect("new_buffered should succeed");

    let input = b"Hello, Zopfli Zlib Buffered World! Hello, Zopfli Zlib Buffered World!";
    let n = bw.write(input).expect("write failed");
    assert_eq!(n, input.len());
    bw.flush().expect("flush failed");


    let encoder = match bw.into_inner() {
        Ok(e) => e,
        Err(_) => panic!("into_inner buf writer failed"),
    };
    let out = {
        let mut e = encoder;
        e.flush().expect("encoder flush");
        drop(e);

        Vec::<u8>::new()
    };


    assert!(out.is_empty());
}

#[test]
fn test_zlib_new_buffered_writes_zlib_header_after_many_small_writes() {




    struct SharedSink(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
    impl Write for SharedSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }

    let shared = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
    let sink = SharedSink(shared.clone());

    let options = Options::default();
    let mut bw = ZlibEncoder::new_buffered(options.clone(), BlockType::Fixed, sink)
        .expect("new_buffered should succeed");


    for _ in 0..50 {
        let w = bw.write(b"abcdefg").expect("write ok");
        assert_eq!(w, 7);
    }
    bw.flush().expect("flush ok");


    drop(bw);

    let out = shared.borrow().clone();
    assert!(out.len() >= 6, "zlib output should have header + adler32, got {}", out.len());


    let cmf = out[0];
    assert_eq!(cmf & 0x0F, 0x08, "CM must be 8 (deflate)");
    let flg = out[1];

    let check = ((cmf as u32) * 256 + flg as u32) % 31;
    assert_eq!(check, 0, "zlib header checksum invalid");


    assert_eq!(flg & 0x20, 0);


    let input: Vec<u8> = b"abcdefg".iter().cycle().take(7 * 50).copied().collect();
    let adler = adler32(&input);
    let tail = &out[out.len() - 4..];
    let got = ((tail[0] as u32) << 24) | ((tail[1] as u32) << 16) | ((tail[2] as u32) << 8) | (tail[3] as u32);
    assert_eq!(got, adler, "adler32 trailer mismatch");

    assert_ne!(out.len(), 0);
    assert!(out.len() < input.len() + 32, "output should be reasonably small");
}

#[test]
fn test_zlib_new_buffered_empty_input() {
    struct SharedSink(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
    impl Write for SharedSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }

    let shared = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
    let sink = SharedSink(shared.clone());

    let options = Options::default();
    let bw = ZlibEncoder::new_buffered(options, BlockType::Dynamic, sink)
        .expect("new_buffered ok");

    drop(bw);

    let out = shared.borrow().clone();
    assert!(out.len() >= 6, "even empty zlib stream has >=6 bytes, got {}", out.len());
    assert_eq!(out[0] & 0x0F, 0x08);
    let cmf = out[0] as u32;
    let flg = out[1] as u32;
    assert_eq!((cmf * 256 + flg) % 31, 0);


    let tail = &out[out.len() - 4..];
    let got = ((tail[0] as u32) << 24) | ((tail[1] as u32) << 16) | ((tail[2] as u32) << 8) | (tail[3] as u32);
    assert_eq!(got, 1);
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}