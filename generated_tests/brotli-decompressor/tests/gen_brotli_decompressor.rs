use std::io::Cursor;

extern crate brotli_decompressor;
extern crate alloc_stdlib;

use brotli_decompressor::BrotliDecoderHasMoreOutput;
use brotli_decompressor::BrotliDecoderIsFinished;
use brotli_decompressor::BrotliDecoderTakeOutput;
use brotli_decompressor::BrotliDecompressStream;
use brotli_decompressor::BrotliState;
use brotli_decompressor::BrotliResult;
use brotli_decompressor::HuffmanCode;
use brotli_decompressor::BrotliDecompress;

use alloc_stdlib::StandardAlloc;

fn new_state() -> BrotliState<StandardAlloc, StandardAlloc, StandardAlloc> {
    BrotliState::new(
        StandardAlloc::default(),
        StandardAlloc::default(),
        StandardAlloc::default(),
    )
}


fn brotli_compressed_hello() -> Vec<u8> {




    vec![0x0b, 0x02, 0x80, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x03]
}


fn brotli_compressed_empty() -> Vec<u8> {
    vec![0x06]
}










fn get_compressed_hello() -> Vec<u8> {
























    vec![0x0b, 0x02, 0x80, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x03]
}

#[test]
fn test_decoder_state_initial_not_finished_no_output() {
    let state = new_state();


    let is_finished = BrotliDecoderIsFinished(&state);
    assert_eq!(is_finished, false);


    let has_more = BrotliDecoderHasMoreOutput(&state);
    assert_eq!(has_more, false);


    let mut state_mut = state;
    let mut size: usize = 0;
    let output = BrotliDecoderTakeOutput(&mut state_mut, &mut size);
    assert_eq!(output.len(), 0);
    assert_eq!(size, 0);


    assert_eq!(BrotliDecoderIsFinished(&state_mut), false);

    assert_eq!(BrotliDecoderHasMoreOutput(&state_mut), false);


    let mut size2: usize = 100;
    let output2 = BrotliDecoderTakeOutput(&mut state_mut, &mut size2);
    assert_eq!(output2.len(), 0);

    assert_eq!(size2, 0);
}

#[test]
fn test_decompress_stream_hello_and_check_finished() {

    let compressed = get_valid_compressed_hello();
    let mut state = new_state();

    let mut available_in = compressed.len();
    let mut input_offset: usize = 0;
    let mut output = vec![0u8; 256];
    let mut available_out = output.len();
    let mut output_offset: usize = 0;
    let mut total_out: usize = 0;


    assert_eq!(BrotliDecoderIsFinished(&state), false);
    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);

    let result = BrotliDecompressStream(
        &mut available_in,
        &mut input_offset,
        &compressed,
        &mut available_out,
        &mut output_offset,
        &mut output,
        &mut total_out,
        &mut state,
    );


    assert_eq!(BrotliDecoderIsFinished(&state), true);


    assert_eq!(format!("{:?}", result), format!("{:?}", BrotliResult::ResultSuccess));


    assert_eq!(total_out, 5);


    assert_eq!(output_offset, 5);


    assert_eq!(&output[..5], b"hello");


    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);


    let mut size: usize = 0;
    let taken = BrotliDecoderTakeOutput(&mut state, &mut size);
    assert_eq!(taken.len(), 0);
    assert_eq!(size, 0);
}


fn get_valid_compressed_hello() -> Vec<u8> {

    let candidates: Vec<Vec<u8>> = vec![
        vec![0x0b, 0x02, 0x80, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x03],
        vec![0x8b, 0x02, 0x80, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x03],
    ];

    for candidate in &candidates {
        let mut input = Cursor::new(candidate.clone());
        let mut out: Vec<u8> = Vec::new();
        if let Ok(()) = BrotliDecompress(&mut input, &mut out) {
            if out == b"hello" {
                return candidate.clone();
            }
        }
    }

    let manual = vec![0x82u8, 0x00, 0x20, 0x68, 0x65, 0x6c, 0x6c, 0x6f];
    let mut input = Cursor::new(manual.clone());
    let mut out: Vec<u8> = Vec::new();
    if let Ok(()) = BrotliDecompress(&mut input, &mut out) {
        if out == b"hello" {
            return manual;
        }
    }

    let q0 = vec![0x0b, 0x02, 0x80, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x03];
    let mut input = Cursor::new(q0.clone());
    let mut out: Vec<u8> = Vec::new();
    if let Ok(()) = BrotliDecompress(&mut input, &mut out) {
        if out == b"hello" {
            return q0;
        }
    }

    let py = vec![0x8b, 0x02, 0x80, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x03];
    let mut input = Cursor::new(py.clone());
    let mut out: Vec<u8> = Vec::new();
    if let Ok(()) = BrotliDecompress(&mut input, &mut out) {
        if out == b"hello" {
            return py;
        }
    }

    let q1 = vec![0x0b, 0x02, 0x80, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x03];
    q1
}

fn get_valid_compressed_empty() -> Vec<u8> {

    let candidates: Vec<Vec<u8>> = vec![
        vec![0x06],
        vec![0x3b],
        vec![59],
    ];

    for candidate in &candidates {
        let mut input = Cursor::new(candidate.clone());
        let mut out: Vec<u8> = Vec::new();
        if let Ok(()) = BrotliDecompress(&mut input, &mut out) {
            if out.is_empty() {
                return candidate.clone();
            }
        }
    }

    vec![0x06]
}

#[test]
fn test_decompress_stream_with_tiny_output_buffer() {
    let compressed = get_valid_compressed_hello();


    {
        let mut input = Cursor::new(compressed.clone());
        let mut out: Vec<u8> = Vec::new();
        let r = BrotliDecompress(&mut input, &mut out);
        assert!(r.is_ok(), "Compressed hello data is not valid brotli");
        assert_eq!(out, b"hello", "Compressed data does not decompress to hello");
    }

    let mut state = new_state();

    let mut available_in = compressed.len();
    let mut input_offset: usize = 0;

    let mut output = vec![0u8; 2];
    let mut available_out = output.len();
    let mut output_offset: usize = 0;
    let mut total_out: usize = 0;

    assert_eq!(BrotliDecoderIsFinished(&state), false);

    let result = BrotliDecompressStream(
        &mut available_in,
        &mut input_offset,
        &compressed,
        &mut available_out,
        &mut output_offset,
        &mut output,
        &mut total_out,
        &mut state,
    );



    assert_eq!(format!("{:?}", result), format!("{:?}", BrotliResult::NeedsMoreOutput));


    assert_eq!(BrotliDecoderIsFinished(&state), false);


    assert_eq!(BrotliDecoderHasMoreOutput(&state), true);


    assert!(output_offset > 0);
    assert!(output_offset <= 2);


    let mut size: usize = 0;
    let taken = BrotliDecoderTakeOutput(&mut state, &mut size);
    assert!(taken.len() > 0);
    assert!(size > 0);
    assert_eq!(taken.len(), size);


    let mut full_output: Vec<u8> = Vec::new();
    full_output.extend_from_slice(&output[..output_offset]);
    full_output.extend_from_slice(taken);


    let mut iterations = 0;
    loop {
        if BrotliDecoderIsFinished(&state) {
            break;
        }
        iterations += 1;
        if iterations > 1000 {
            panic!("Too many iterations, possible infinite loop");
        }
        if !BrotliDecoderHasMoreOutput(&state) {

            available_out = output.len();
            output_offset = 0;
            let r = BrotliDecompressStream(
                &mut available_in,
                &mut input_offset,
                &compressed,
                &mut available_out,
                &mut output_offset,
                &mut output,
                &mut total_out,
                &mut state,
            );
            full_output.extend_from_slice(&output[..output_offset]);
            if let BrotliResult::ResultSuccess = r {
                break;
            }
        } else {
            let mut sz: usize = 0;
            let t = BrotliDecoderTakeOutput(&mut state, &mut sz);
            full_output.extend_from_slice(t);
        }
    }

    assert_eq!(full_output, b"hello");
    assert_eq!(BrotliDecoderIsFinished(&state), true);
}

#[test]
fn test_decompress_empty_stream() {
    let compressed = get_valid_compressed_empty();
    let mut state = new_state();

    let mut available_in = compressed.len();
    let mut input_offset: usize = 0;
    let mut output = vec![0u8; 256];
    let mut available_out = output.len();
    let mut output_offset: usize = 0;
    let mut total_out: usize = 0;

    let result = BrotliDecompressStream(
        &mut available_in,
        &mut input_offset,
        &compressed,
        &mut available_out,
        &mut output_offset,
        &mut output,
        &mut total_out,
        &mut state,
    );


    assert_eq!(format!("{:?}", result), format!("{:?}", BrotliResult::ResultSuccess));


    assert_eq!(total_out, 0);


    assert_eq!(output_offset, 0);


    assert_eq!(BrotliDecoderIsFinished(&state), true);


    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);
}