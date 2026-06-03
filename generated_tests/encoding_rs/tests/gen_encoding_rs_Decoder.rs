use encoding_rs::*;

fn make_str_buf(buf: &mut [u8]) -> &mut str {

    for b in buf.iter_mut() {
        *b = 0;
    }
    std::str::from_utf8_mut(buf).unwrap()
}

#[test]
fn test_decode_to_str_utf8_ascii_complete() {
    let enc = Encoding::for_label(b"utf-8").expect("utf-8 must exist");
    let mut decoder = enc.new_decoder();
    assert_eq!(decoder.encoding().name(), "UTF-8");

    let src = b"Hello, world!";
    let mut backing = [0u8; 64];
    let dst = make_str_buf(&mut backing);

    let (result, read, written, had_errors) = decoder.decode_to_str(src, dst, true);

    match result {
        CoderResult::InputEmpty => {}
        CoderResult::OutputFull => panic!("output should not be full"),
    }
    assert_eq!(read, src.len());
    assert_eq!(written, src.len());
    assert_eq!(had_errors, false);
    assert_eq!(&dst.as_bytes()[..written], src);
    assert_eq!(&dst[..written], "Hello, world!");
    assert_ne!(written, 0);
    assert!(written <= dst.len());
}

#[test]
fn test_decode_to_str_streaming_two_chunks() {
    let enc = Encoding::for_label(b"UTF-8").unwrap();
    let mut decoder = enc.new_decoder();

    let chunk1 = b"abc";
    let chunk2 = b"def";

    let mut buf1 = [0u8; 32];
    let dst1 = make_str_buf(&mut buf1);
    let (r1, read1, written1, errs1) = decoder.decode_to_str(chunk1, dst1, false);
    assert!(matches!(r1, CoderResult::InputEmpty));
    assert_eq!(read1, 3);
    assert_eq!(written1, 3);
    assert_eq!(errs1, false);
    assert_eq!(&dst1[..written1], "abc");

    let mut buf2 = [0u8; 32];
    let dst2 = make_str_buf(&mut buf2);
    let (r2, read2, written2, errs2) = decoder.decode_to_str(chunk2, dst2, true);
    assert!(matches!(r2, CoderResult::InputEmpty));
    assert_eq!(read2, 3);
    assert_eq!(written2, 3);
    assert_eq!(errs2, false);
    assert_eq!(&dst2[..written2], "def");

    assert_ne!(read1 + read2, 0);
    assert_eq!(read1 + read2, 6);
}

#[test]
fn test_decode_to_str_output_full() {
    let enc = Encoding::for_label(b"utf-8").unwrap();
    let mut decoder = enc.new_decoder();

    let src = b"abcdefghij";
    let mut backing = [0u8; 4];
    let dst = make_str_buf(&mut backing);

    let (result, read, written, errs) = decoder.decode_to_str(src, dst, false);
    assert!(matches!(result, CoderResult::OutputFull));
    assert!(written <= 4);
    assert!(written > 0);
    assert!(read >= written);
    assert!(read <= src.len());
    assert_eq!(errs, false);
    assert_eq!(&dst.as_bytes()[..written], &src[..written]);


    let mut backing2 = [0u8; 64];
    let dst2 = make_str_buf(&mut backing2);
    let (result2, read2, written2, errs2) =
        decoder.decode_to_str(&src[read..], dst2, true);
    assert!(matches!(result2, CoderResult::InputEmpty));
    assert_eq!(read2, src.len() - read);
    assert_eq!(written2, src.len() - read);
    assert_eq!(errs2, false);
    assert_eq!(written + written2, src.len());
}

#[test]
fn test_decode_to_str_with_replacement_on_invalid_utf8() {
    let enc = Encoding::for_label(b"utf-8").unwrap();
    let mut decoder = enc.new_decoder();


    let src = &[b'a', 0xFF, b'b'];
    let mut backing = [0u8; 64];
    let dst = make_str_buf(&mut backing);

    let (result, read, written, had_errors) = decoder.decode_to_str(src, dst, true);
    assert!(matches!(result, CoderResult::InputEmpty));
    assert_eq!(read, 3);
    assert_eq!(had_errors, true);
    assert_ne!(had_errors, false);
    let out = &dst[..written];
    assert!(out.starts_with("a"));
    assert!(out.ends_with("b"));
    assert!(out.contains('\u{FFFD}'));
    assert!(written >= 3);
}

#[test]
fn test_decode_to_str_without_replacement_success() {
    let enc = Encoding::for_label(b"utf-8").unwrap();
    let mut decoder = enc.new_decoder();
    assert_eq!(decoder.encoding().name(), "UTF-8");

    let src = "héllo".as_bytes();
    let mut backing = [0u8; 64];
    let dst = make_str_buf(&mut backing);

    let (result, read, written) = decoder.decode_to_str_without_replacement(src, dst, true);
    match result {
        DecoderResult::InputEmpty => {}
        DecoderResult::OutputFull => panic!("unexpected OutputFull"),
        DecoderResult::Malformed(_, _) => panic!("valid input flagged malformed"),
    }
    assert_eq!(read, src.len());
    assert_eq!(written, src.len());
    assert_ne!(written, 0);
    assert_eq!(&dst[..written], "héllo");
    assert_eq!(dst[..written].chars().count(), 5);
}

#[test]
fn test_decode_to_str_without_replacement_malformed_and_output_full() {

    let enc = Encoding::for_label(b"utf-8").unwrap();
    let mut decoder = enc.new_decoder();

    let src = &[b'X', 0xC0, 0xC0, b'Y'];
    let mut backing = [0u8; 64];
    let dst = make_str_buf(&mut backing);

    let (result, read, written) = decoder.decode_to_str_without_replacement(src, dst, false);
    assert!(matches!(result, DecoderResult::Malformed(_, _)));
    assert!(read >= 1);
    assert!(read <= src.len());
    assert!(written >= 1);
    assert_eq!(&dst.as_bytes()[..1], b"X");


    let mut decoder2 = enc.new_decoder();
    let src2 = b"abcdefgh";
    let mut backing2 = [0u8; 3];
    let dst2 = make_str_buf(&mut backing2);
    let (r2, read2, written2) = decoder2.decode_to_str_without_replacement(src2, dst2, false);
    assert!(matches!(r2, DecoderResult::OutputFull));
    assert!(written2 <= 3);
    assert!(written2 > 0);
    assert!(read2 >= written2);
    assert_eq!(&dst2.as_bytes()[..written2], &src2[..written2]);
}

#[test]
fn test_decode_to_str_multibyte_split_across_calls() {



    let enc = Encoding::for_label(b"utf-8").unwrap();
    let mut decoder = enc.new_decoder();


    let part1 = &[0xC3u8];
    let part2 = &[0xA9u8, b'!'];

    let mut backing1 = [0u8; 16];
    let dst1 = make_str_buf(&mut backing1);
    let (r1, read1, written1, errs1) = decoder.decode_to_str(part1, dst1, false);
    assert!(matches!(r1, CoderResult::InputEmpty));
    assert_eq!(read1, 1);
    assert_eq!(written1, 0);
    assert_eq!(errs1, false);

    let mut backing2 = [0u8; 16];
    let dst2 = make_str_buf(&mut backing2);
    let (r2, read2, written2, errs2) = decoder.decode_to_str(part2, dst2, true);
    assert!(matches!(r2, CoderResult::InputEmpty));
    assert_eq!(read2, 2);
    assert_eq!(errs2, false);
    assert!(written2 >= 3);
    assert_eq!(&dst2[..written2], "é!");
    assert_ne!(written2, 0);
}