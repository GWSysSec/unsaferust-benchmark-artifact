use encoding_rs::*;

#[test]
fn test_encode_from_utf8_to_vec_ascii_windows1252() {
    let encoding = Encoding::for_label(b"windows-1252").expect("windows-1252 exists");
    let mut encoder = encoding.new_encoder();

    let input = "Hello, World!";
    assert_eq!(input.len(), 13);

    let max_len = encoder
        .max_buffer_length_from_utf8_without_replacement(input.len())
        .expect("reasonable size");
    let mut output: Vec<u8> = Vec::with_capacity(max_len);
    unsafe { output.set_len(max_len); }

    let (result, read, written) =
        encoder.encode_from_utf8_without_replacement(input, &mut output, true);

    match result {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read, 13);
    output.truncate(written);
    assert_eq!(output.len(), 13);
    assert_eq!(&output[..], b"Hello, World!");
    assert_eq!(output[0], b'H');
    assert_eq!(output[6], b' ');
    assert_eq!(output[12], b'!');
    assert_eq!(encoder.encoding(), encoding);
}

#[test]
fn test_encode_from_utf8_to_vec_unmappable_emoji_in_windows1252() {
    let encoding = Encoding::for_label(b"windows-1252").expect("windows-1252 exists");
    let mut encoder = encoding.new_encoder();

    let input = "Hi 😀!";
    assert_eq!(input.len(), 8);

    let max_len = encoder
        .max_buffer_length_from_utf8_without_replacement(input.len())
        .expect("reasonable size");
    let mut output: Vec<u8> = Vec::with_capacity(max_len);
    unsafe { output.set_len(max_len); }

    let (result, read, written) =
        encoder.encode_from_utf8_without_replacement(input, &mut output, true);

    let unmappable = match result {
        EncoderResult::Unmappable(c) => c,
        other => panic!("expected Unmappable, got {:?}", other),
    };
    assert_eq!(unmappable, '😀');
    assert_eq!(read, 7);
    output.truncate(written);
    assert_eq!(output.len(), 3);
    assert_eq!(&output[..], b"Hi ");
    assert_ne!(output.len(), input.len());


    let remaining = &input[read..];
    assert_eq!(remaining, "!");
    let pre_len = output.len();

    let max_len2 = encoder
        .max_buffer_length_from_utf8_without_replacement(remaining.len())
        .expect("reasonable size");
    let mut output2: Vec<u8> = Vec::with_capacity(max_len2);
    unsafe { output2.set_len(max_len2); }

    let (result2, read2, written2) =
        encoder.encode_from_utf8_without_replacement(remaining, &mut output2, true);
    match result2 {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read2, 1);
    output.extend_from_slice(&output2[..written2]);
    assert_eq!(output.len(), pre_len + 1);
    assert_eq!(*output.last().unwrap(), b'!');
}

#[test]
fn test_encode_from_utf8_to_vec_appends_to_prefilled_vec() {
    let encoding = Encoding::for_label(b"windows-1252").expect("windows-1252 exists");
    let mut encoder = encoding.new_encoder();

    let mut output: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD];
    assert_eq!(output.len(), 4);
    assert_eq!(output[0], 0xAA);
    assert_eq!(output[1], 0xBB);
    assert_eq!(output[2], 0xCC);
    assert_eq!(output[3], 0xDD);

    let input = "XYZ";
    let max_len = encoder
        .max_buffer_length_from_utf8_without_replacement(input.len())
        .expect("reasonable size");
    let mut tmp: Vec<u8> = Vec::with_capacity(max_len);
    unsafe { tmp.set_len(max_len); }

    let (result, read, written) =
        encoder.encode_from_utf8_without_replacement(input, &mut tmp, true);

    match result {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read, 3);
    output.extend_from_slice(&tmp[..written]);
    assert_eq!(output.len(), 7);

    assert_eq!(output[0], 0xAA);
    assert_eq!(output[1], 0xBB);
    assert_eq!(output[2], 0xCC);
    assert_eq!(output[3], 0xDD);

    assert_eq!(output[4], b'X');
    assert_eq!(output[5], b'Y');
    assert_eq!(output[6], b'Z');
}

#[test]
fn test_encode_from_utf8_to_vec_empty_input_with_last_true() {
    let encoding = Encoding::for_label(b"iso-8859-1").expect("iso-8859-1 exists");
    let mut encoder = encoding.new_encoder();

    let input = "";

    assert_eq!(input.len(), 0);

    let max_len = encoder
        .max_buffer_length_from_utf8_without_replacement(input.len())
        .expect("reasonable size");
    let mut output: Vec<u8> = Vec::with_capacity(max_len);
    unsafe { output.set_len(max_len); }

    let (result, read, written) =
        encoder.encode_from_utf8_without_replacement(input, &mut output, true);

    match result {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty on empty input, got {:?}", other),
    }
    assert_eq!(read, 0);
    output.truncate(written);
    assert_eq!(output.len(), 0);
    assert!(output.is_empty());
    assert_eq!(output.capacity() >= 0, true);
    assert_eq!(&output[..], b"");
}

#[test]
fn test_encode_from_utf8_to_vec_streaming_last_false_then_true() {
    let encoding = Encoding::for_label(b"windows-1252").expect("windows-1252 exists");
    let mut encoder = encoding.new_encoder();

    let mut output: Vec<u8> = Vec::new();


    let chunk1 = "Hello ";
    let max1 = encoder
        .max_buffer_length_from_utf8_without_replacement(chunk1.len())
        .expect("reasonable size");
    let mut tmp1: Vec<u8> = vec![0u8; max1];
    let (r1, read1, written1) =
        encoder.encode_from_utf8_without_replacement(chunk1, &mut tmp1, false);
    match r1 {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read1, 6);
    output.extend_from_slice(&tmp1[..written1]);
    assert_eq!(output.len(), 6);
    assert_eq!(&output[..6], b"Hello ");


    let chunk2 = "beautiful ";
    let pre_len = output.len();
    let max2 = encoder
        .max_buffer_length_from_utf8_without_replacement(chunk2.len())
        .expect("reasonable size");
    let mut tmp2: Vec<u8> = vec![0u8; max2];
    let (r2, read2, written2) =
        encoder.encode_from_utf8_without_replacement(chunk2, &mut tmp2, false);
    match r2 {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read2, 10);
    output.extend_from_slice(&tmp2[..written2]);
    assert_eq!(output.len(), pre_len + 10);


    let chunk3 = "world!";
    let pre_len2 = output.len();
    let max3 = encoder
        .max_buffer_length_from_utf8_without_replacement(chunk3.len())
        .expect("reasonable size");
    let mut tmp3: Vec<u8> = vec![0u8; max3];
    let (r3, read3, written3) =
        encoder.encode_from_utf8_without_replacement(chunk3, &mut tmp3, true);
    match r3 {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read3, 6);
    output.extend_from_slice(&tmp3[..written3]);
    assert_eq!(output.len(), pre_len2 + 6);
    assert_eq!(&output[..], b"Hello beautiful world!");
}

#[test]
fn test_encode_from_utf8_to_vec_gbk_chinese_chars() {
    let encoding = Encoding::for_label(b"gbk").expect("gbk exists");
    let mut encoder = encoding.new_encoder();

    let input = "中文ABC";
    assert_eq!(input.len(), 9);

    let max_len = encoder
        .max_buffer_length_from_utf8_without_replacement(input.len())
        .expect("reasonable size");
    let mut output: Vec<u8> = vec![0u8; max_len];

    let (result, read, written) =
        encoder.encode_from_utf8_without_replacement(input, &mut output, true);

    match result {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read, 9);
    output.truncate(written);

    assert_eq!(output.len(), 7);

    assert_eq!(output[4], b'A');
    assert_eq!(output[5], b'B');
    assert_eq!(output[6], b'C');

    assert!(output[0] >= 0x80);
    assert!(output[1] >= 0x40);
    assert!(output[2] >= 0x80);
    assert!(output[3] >= 0x40);
}

#[test]
fn test_encode_from_utf8_to_vec_utf8_passthrough_includes_astral() {
    let encoding = Encoding::for_label(b"utf-8").expect("utf-8 exists");
    let mut encoder = encoding.new_encoder();

    let input = "Hello, 世界! 🌍";

    let expected_bytes = input.as_bytes().to_vec();
    assert!(expected_bytes.len() > input.chars().count());

    let max_len = encoder
        .max_buffer_length_from_utf8_without_replacement(input.len())
        .expect("reasonable size");
    let mut output: Vec<u8> = vec![0u8; max_len];

    let (result, read, written) =
        encoder.encode_from_utf8_without_replacement(input, &mut output, true);

    match result {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty for utf-8 encoder, got {:?}", other),
    }
    assert_eq!(read, input.len());
    output.truncate(written);
    assert_eq!(output.len(), expected_bytes.len());
    assert_eq!(&output[..], &expected_bytes[..]);
    assert_eq!(output[0], b'H');
    assert_ne!(output[7], 0);
}

#[test]
fn test_max_buffer_length_from_utf16_windows1252_monotonic() {
    let encoding = Encoding::for_label(b"windows-1252").expect("windows-1252 exists");
    let encoder = encoding.new_encoder();

    let m0 = encoder.max_buffer_length_from_utf16_if_no_unmappables(0);
    let m1 = encoder.max_buffer_length_from_utf16_if_no_unmappables(1);
    let m10 = encoder.max_buffer_length_from_utf16_if_no_unmappables(10);
    let m100 = encoder.max_buffer_length_from_utf16_if_no_unmappables(100);
    let m1000 = encoder.max_buffer_length_from_utf16_if_no_unmappables(1000);

    assert!(m0.is_some());
    assert!(m1.is_some());
    assert!(m10.is_some());
    assert!(m100.is_some());
    assert!(m1000.is_some());

    let v0 = m0.unwrap();
    let v1 = m1.unwrap();
    let v10 = m10.unwrap();
    let v100 = m100.unwrap();
    let v1000 = m1000.unwrap();


    assert!(v1 >= 1);
    assert!(v10 >= 10);
    assert!(v100 >= 100);
    assert!(v1000 >= 1000);


    assert!(v1 >= v0);
    assert!(v10 >= v1);
    assert!(v100 >= v10);
    assert!(v1000 >= v100);
}

#[test]
fn test_max_buffer_length_from_utf16_shift_jis_and_overflow() {
    let encoding = Encoding::for_label(b"shift_jis").expect("shift_jis exists");
    let encoder = encoding.new_encoder();

    let small = encoder.max_buffer_length_from_utf16_if_no_unmappables(16);
    assert!(small.is_some());
    let small_v = small.unwrap();
    assert!(small_v >= 16);

    let medium = encoder.max_buffer_length_from_utf16_if_no_unmappables(4096);
    assert!(medium.is_some());
    let medium_v = medium.unwrap();
    assert!(medium_v >= 4096);
    assert!(medium_v > small_v);


    let overflow = encoder.max_buffer_length_from_utf16_if_no_unmappables(usize::MAX);
    assert_eq!(overflow, None);

    let overflow2 = encoder.max_buffer_length_from_utf16_if_no_unmappables(usize::MAX - 1);
    assert_eq!(overflow2, None);


    let ok = encoder.max_buffer_length_from_utf16_if_no_unmappables(100_000);
    assert!(ok.is_some());
    assert!(ok.unwrap() >= 100_000);
}

#[test]
fn test_max_buffer_length_from_utf16_sizing_drives_actual_encode() {


    let encoding = Encoding::for_label(b"windows-1252").expect("windows-1252 exists");
    let mut encoder = encoding.new_encoder();

    let utf16_sample: Vec<u16> = "abcdefghij".encode_utf16().collect();
    assert_eq!(utf16_sample.len(), 10);

    let needed = encoder
        .max_buffer_length_from_utf16_if_no_unmappables(utf16_sample.len())
        .expect("reasonable size should be Some");
    assert!(needed >= utf16_sample.len());



    let utf8_input = "abcdefghij";
    let max_len = encoder
        .max_buffer_length_from_utf8_without_replacement(utf8_input.len())
        .expect("reasonable size");
    let mut output: Vec<u8> = vec![0u8; max_len];

    let (result, read, written) =
        encoder.encode_from_utf8_without_replacement(utf8_input, &mut output, true);
    match result {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read, utf8_input.len());
    output.truncate(written);
    assert_eq!(output.len(), 10);
    assert!(output.len() <= needed);
    assert_eq!(&output[..], b"abcdefghij");
}

#[test]
fn test_encode_from_utf8_to_vec_multiple_unmappables_sequential() {
    let encoding = Encoding::for_label(b"windows-1252").expect("windows-1252 exists");
    let mut encoder = encoding.new_encoder();


    let input = "A中B文C";
    let mut output: Vec<u8> = Vec::new();

    let total_bytes = input.len();
    assert_eq!(total_bytes, 9);


    let max1 = encoder
        .max_buffer_length_from_utf8_without_replacement(input.len())
        .expect("reasonable size");
    let mut tmp1: Vec<u8> = vec![0u8; max1];
    let (r1, read1, written1) =
        encoder.encode_from_utf8_without_replacement(input, &mut tmp1, true);
    let c1 = match r1 {
        EncoderResult::Unmappable(c) => c,
        other => panic!("expected Unmappable, got {:?}", other),
    };
    assert_eq!(c1, '中');
    assert_eq!(read1, 4);
    output.extend_from_slice(&tmp1[..written1]);
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], b'A');


    let rest = &input[read1..];
    assert_eq!(rest, "B文C");
    let max2 = encoder
        .max_buffer_length_from_utf8_without_replacement(rest.len())
        .expect("reasonable size");
    let mut tmp2: Vec<u8> = vec![0u8; max2];
    let (r2, read2, written2) =
        encoder.encode_from_utf8_without_replacement(rest, &mut tmp2, true);
    let c2 = match r2 {
        EncoderResult::Unmappable(c) => c,
        other => panic!("expected Unmappable, got {:?}", other),
    };
    assert_eq!(c2, '文');
    assert_eq!(read2, 4);
    output.extend_from_slice(&tmp2[..written2]);
    assert_eq!(output.len(), 2);
    assert_eq!(output[1], b'B');


    let rest2 = &rest[read2..];
    assert_eq!(rest2, "C");
    let max3 = encoder
        .max_buffer_length_from_utf8_without_replacement(rest2.len())
        .expect("reasonable size");
    let mut tmp3: Vec<u8> = vec![0u8; max3];
    let (r3, read3, written3) =
        encoder.encode_from_utf8_without_replacement(rest2, &mut tmp3, true);
    match r3 {
        EncoderResult::InputEmpty => {}
        other => panic!("expected InputEmpty, got {:?}", other),
    }
    assert_eq!(read3, 1);
    output.extend_from_slice(&tmp3[..written3]);
    assert_eq!(output.len(), 3);
    assert_eq!(&output[..], b"ABC");
}