use zopfli::{compress, BlockType, Format, GzipEncoder, Options};
use std::io::Write;

#[test]
fn test_gzip_new_buffered_produces_valid_gzip_header_and_trailer() {
    let opts = Options::default();
    let mut sink: Vec<u8> = Vec::new();
    let input: Vec<u8> = b"The quick brown fox jumps over the lazy dog. "
        .iter()
        .cycle()
        .take(2000)
        .copied()
        .collect();


    assert_eq!(sink.len(), 0, "pre: sink empty");
    assert_eq!(input.len(), 2000, "pre: input has expected size");
    assert_ne!(input[0], 0, "pre: input bytes are printable");

    {
        let mut enc = GzipEncoder::new_buffered(opts.clone(), BlockType::Dynamic, &mut sink)
            .expect("new_buffered must construct successfully");
        enc.write_all(&input).expect("write_all should succeed");
        enc.flush().expect("flush should succeed");

    }


    assert!(sink.len() >= 18, "gzip stream must have header+trailer, got {}", sink.len());
    assert_eq!(sink[0], 0x1f, "gzip magic byte 0");
    assert_eq!(sink[1], 0x8b, "gzip magic byte 1");
    assert_eq!(sink[2], 0x08, "compression method must be deflate");


    let n = sink.len();
    let isize_val =
        u32::from_le_bytes([sink[n - 4], sink[n - 3], sink[n - 2], sink[n - 1]]);
    assert_eq!(isize_val as usize, input.len(), "ISIZE must match input length");


    assert!(
        sink.len() < input.len(),
        "repetitive input should compress smaller: {} < {}",
        sink.len(),
        input.len()
    );
}

#[test]
fn test_gzip_new_buffered_empty_input_still_valid_frame() {
    let opts = Options::default();
    let mut sink: Vec<u8> = Vec::new();

    assert_eq!(sink.len(), 0, "pre: sink empty");
    assert_eq!(sink.capacity(), 0, "pre: sink no capacity");

    {
        let mut enc = GzipEncoder::new_buffered(opts.clone(), BlockType::Dynamic, &mut sink)
            .expect("new_buffered for empty input must succeed");

        enc.flush().expect("flush on empty must succeed");
    }


    assert!(sink.len() >= 18, "empty gzip still has full framing: {}", sink.len());
    assert_eq!(sink[0], 0x1f, "magic[0]");
    assert_eq!(sink[1], 0x8b, "magic[1]");
    assert_eq!(sink[2], 0x08, "CM=deflate");

    let n = sink.len();
    let isize_val =
        u32::from_le_bytes([sink[n - 4], sink[n - 3], sink[n - 2], sink[n - 1]]);
    assert_eq!(isize_val, 0u32, "ISIZE must be 0 for empty input");
    let crc_val =
        u32::from_le_bytes([sink[n - 8], sink[n - 7], sink[n - 6], sink[n - 5]]);
    assert_eq!(crc_val, 0u32, "CRC32 of empty input must be 0");

    assert!(sink.len() < 64, "empty gzip frame should be small, got {}", sink.len());
}

#[test]
fn test_gzip_new_buffered_matches_compress_function_on_same_input() {
    let opts = Options::default();
    let input: Vec<u8> = (0u32..2000)
        .map(|i| i.wrapping_mul(31).wrapping_add(7) as u8)
        .collect();

    assert_eq!(input.len(), 2000, "pre: expected input length");
    assert_ne!(input[0], input[1], "pre: input is not constant");


    let mut sink_buf: Vec<u8> = Vec::new();
    {
        let mut enc = GzipEncoder::new_buffered(opts.clone(), BlockType::Dynamic, &mut sink_buf)
            .expect("new_buffered should succeed");
        enc.write_all(&input).expect("write_all to buffered encoder");
        enc.flush().expect("flush buffered encoder");
    }


    let mut sink_cmp: Vec<u8> = Vec::new();
    compress(opts.clone(), Format::Gzip, &input[..], &mut sink_cmp)
        .expect("compress call should succeed");

    assert!(sink_buf.len() >= 18, "buffered output framed");
    assert!(sink_cmp.len() >= 18, "compress output framed");
    assert_eq!(sink_buf[0], sink_cmp[0], "magic[0] matches between methods");
    assert_eq!(sink_buf[1], sink_cmp[1], "magic[1] matches between methods");
    assert_eq!(sink_buf[2], sink_cmp[2], "CM (deflate=8) matches between methods");
    assert_eq!(sink_buf[2], 0x08, "CM must be deflate");

    let nb = sink_buf.len();
    let nc = sink_cmp.len();
    let isize_buf = u32::from_le_bytes([
        sink_buf[nb - 4], sink_buf[nb - 3], sink_buf[nb - 2], sink_buf[nb - 1],
    ]);
    let isize_cmp = u32::from_le_bytes([
        sink_cmp[nc - 4], sink_cmp[nc - 3], sink_cmp[nc - 2], sink_cmp[nc - 1],
    ]);
    let crc_buf = u32::from_le_bytes([
        sink_buf[nb - 8], sink_buf[nb - 7], sink_buf[nb - 6], sink_buf[nb - 5],
    ]);
    let crc_cmp = u32::from_le_bytes([
        sink_cmp[nc - 8], sink_cmp[nc - 7], sink_cmp[nc - 6], sink_cmp[nc - 5],
    ]);

    assert_eq!(isize_buf as usize, input.len(), "new_buffered ISIZE matches input length");
    assert_eq!(isize_cmp as usize, input.len(), "compress ISIZE matches input length");
    assert_eq!(isize_buf, isize_cmp, "ISIZE fields identical across methods");
    assert_eq!(crc_buf, crc_cmp, "CRC32 identical across methods");
}