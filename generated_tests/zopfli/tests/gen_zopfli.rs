use std::io::Cursor;
use zopfli::{compress, Format, Options};

#[test]
fn compress_gzip_roundtrip_structure() {
    let input = b"The quick brown fox jumps over the lazy dog. \
                  The quick brown fox jumps over the lazy dog. \
                  The quick brown fox jumps over the lazy dog.";
    let mut out: Vec<u8> = Vec::new();
    let options = Options::default();
    let result = compress(options.clone(), Format::Gzip, Cursor::new(&input[..]), &mut out);
    assert!(result.is_ok(), "compress should succeed");


    assert!(out.len() > 18, "gzip output too small: {}", out.len());
    assert_eq!(out[0], 0x1f, "gzip magic byte 0");
    assert_eq!(out[1], 0x8b, "gzip magic byte 1");
    assert_eq!(out[2], 0x08, "deflate compression method");


    let n = out.len();
    let isize_bytes = &out[n - 4..n];
    let isize_val = u32::from_le_bytes([isize_bytes[0], isize_bytes[1], isize_bytes[2], isize_bytes[3]]);
    assert_eq!(isize_val as usize, input.len(), "gzip ISIZE must equal input length mod 2^32");


    let expected_crc = crc32(input);
    let crc_bytes = &out[n - 8..n - 4];
    let crc_val = u32::from_le_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);
    assert_eq!(crc_val, expected_crc, "gzip CRC32 mismatch");


    assert!(out.len() < input.len(), "expected compression: {} >= {}", out.len(), input.len());
}

#[test]
fn compress_zlib_header_and_adler() {
    let input: Vec<u8> = (0..2048u32).map(|i| (i % 256) as u8).collect();
    let mut out: Vec<u8> = Vec::new();
    let result = compress(Options::default(), Format::Zlib, Cursor::new(&input), &mut out);
    assert!(result.is_ok());
    assert!(out.len() >= 6, "zlib output too small");


    let cmf = out[0];
    assert_eq!(cmf & 0x0f, 0x08, "zlib CM must be 8 (deflate)");
    let cinfo = (cmf >> 4) & 0x0f;
    assert!(cinfo <= 7, "zlib CINFO out of range: {}", cinfo);


    let flg = out[1];
    let header = (cmf as u32) * 256 + (flg as u32);
    assert_eq!(header % 31, 0, "zlib header checksum invalid");


    let n = out.len();
    let adler_bytes = &out[n - 4..n];
    let adler_val = u32::from_be_bytes([adler_bytes[0], adler_bytes[1], adler_bytes[2], adler_bytes[3]]);
    assert_eq!(adler_val, adler32(&input), "zlib Adler-32 mismatch");
    assert_ne!(out.len(), 0);
}

#[test]
fn compress_deflate_raw_output() {
    let input = b"aaaaaaaaaabbbbbbbbbbccccccccccddddddddddeeeeeeeeeeffffffffff";
    let mut out: Vec<u8> = Vec::new();
    let result = compress(Options::default(), Format::Deflate, Cursor::new(&input[..]), &mut out);
    assert!(result.is_ok());


    assert!(!out.is_empty(), "deflate output empty");
    assert_ne!(out[0], 0x1f, "deflate should not have gzip magic");


    assert!(out.len() < input.len() / 2, "expected strong compression: out={} in={}", out.len(), input.len());


    let bfinal = out[0] & 0x01;
    assert_eq!(bfinal, 1, "BFINAL should be set on final block");
}

#[test]
fn compress_empty_input_all_formats() {
    for &fmt in &[Format::Gzip, Format::Zlib, Format::Deflate] {
        let mut out: Vec<u8> = Vec::new();
        let result = compress(Options::default(), fmt.clone(), Cursor::new(&[][..]), &mut out);
        assert!(result.is_ok(), "compress empty failed for {:?}-ish format", out.len());
        assert!(!out.is_empty(), "empty input should still produce framing bytes");
        match fmt {
            Format::Gzip => {
                assert_eq!(out[0], 0x1f);
                assert_eq!(out[1], 0x8b);
                let n = out.len();
                let isize_val = u32::from_le_bytes([out[n-4], out[n-3], out[n-2], out[n-1]]);
                assert_eq!(isize_val, 0);
                let crc_val = u32::from_le_bytes([out[n-8], out[n-7], out[n-6], out[n-5]]);
                assert_eq!(crc_val, 0);
            }
            Format::Zlib => {
                assert_eq!(out[0] & 0x0f, 0x08);
                let header = (out[0] as u32) * 256 + (out[1] as u32);
                assert_eq!(header % 31, 0);
                let n = out.len();
                let adler_val = u32::from_be_bytes([out[n-4], out[n-3], out[n-2], out[n-1]]);
                assert_eq!(adler_val, 1, "Adler-32 of empty is 1");
            }
            Format::Deflate => {
                assert!(out.len() >= 1);
            }
        }
    }
}

#[test]
fn compress_large_repetitive_input_ratio() {
    let unit = b"zopfli-compression-test-";
    let mut input: Vec<u8> = Vec::new();
    for _ in 0..1000 {
        input.extend_from_slice(unit);
    }
    assert_eq!(input.len(), unit.len() * 1000);

    let mut out: Vec<u8> = Vec::new();
    let result = compress(Options::default(), Format::Gzip, Cursor::new(&input), &mut out);
    assert!(result.is_ok());

    assert!(out.len() < input.len() / 10, "repetitive data should achieve >10x: out={} in={}", out.len(), input.len());
    assert_eq!(out[0], 0x1f);
    assert_eq!(out[1], 0x8b);

    let n = out.len();
    let isize_val = u32::from_le_bytes([out[n-4], out[n-3], out[n-2], out[n-1]]);
    assert_eq!(isize_val as usize, input.len() % (1usize << 32));

    let crc_val = u32::from_le_bytes([out[n-8], out[n-7], out[n-6], out[n-5]]);
    assert_eq!(crc_val, crc32(&input));
}



fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
        }
        table[i as usize] = c;
    }
    let mut crc: u32 = 0xffffffff;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffffffff
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}