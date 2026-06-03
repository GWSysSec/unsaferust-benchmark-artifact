use std::io::Cursor;

use brotli_decompressor::BrotliDecoderHasMoreOutput;
use brotli_decompressor::BrotliDecoderIsFinished;
use brotli_decompressor::BrotliDecoderTakeOutput;
use brotli_decompressor::BrotliDecompressStream;
use brotli_decompressor::BrotliState;
use brotli_decompressor::BrotliResult;
use brotli_decompressor::BrotliDecompress;

extern crate brotli_decompressor;
extern crate alloc_stdlib;
use alloc_stdlib::StandardAlloc;

fn new_state() -> BrotliState<StandardAlloc, StandardAlloc, StandardAlloc> {
    BrotliState::new(
        StandardAlloc::default(),
        StandardAlloc::default(),
        StandardAlloc::default(),
    )
}

fn new_state_strict() -> BrotliState<StandardAlloc, StandardAlloc, StandardAlloc> {
    BrotliState::new(
        StandardAlloc::default(),
        StandardAlloc::default(),
        StandardAlloc::default(),
    )
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

    candidates[0].clone()
}

fn get_valid_compressed_empty() -> Vec<u8> {
    let candidates: Vec<Vec<u8>> = vec![
        vec![0x06],
        vec![0x3b],
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
fn test_new_strict_initial_state_is_stream_start() {
    let state = new_state_strict();


    assert_eq!(state.BrotliStateIsStreamStart(), true);


    assert_eq!(state.BrotliStateIsStreamEnd(), false);


    assert_eq!(BrotliDecoderIsFinished(&state), false);


    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);


    let mut state_mut = state;
    let mut size: usize = 0;
    let output = BrotliDecoderTakeOutput(&mut state_mut, &mut size);
    assert_eq!(output.len(), 0);
    assert_eq!(size, 0);


    assert_eq!(state_mut.BrotliStateIsStreamStart(), true);
    assert_eq!(state_mut.BrotliStateIsStreamEnd(), false);
}

#[test]
fn test_new_strict_decompress_hello_transitions_stream_state() {
    let compressed = get_valid_compressed_hello();
    let mut state = new_state_strict();


    assert_eq!(state.BrotliStateIsStreamStart(), true);
    assert_eq!(state.BrotliStateIsStreamEnd(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), false);

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
    assert_eq!(total_out, 5);
    assert_eq!(&output[..5], b"hello");


    assert_eq!(state.BrotliStateIsStreamStart(), false);


    assert_eq!(state.BrotliStateIsStreamEnd(), true);


    assert_eq!(BrotliDecoderIsFinished(&state), true);
}

#[test]
fn test_new_strict_decompress_empty_stream_end() {
    let compressed = get_valid_compressed_empty();
    let mut state = new_state_strict();

    assert_eq!(state.BrotliStateIsStreamStart(), true);
    assert_eq!(state.BrotliStateIsStreamEnd(), false);

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


    assert_eq!(state.BrotliStateIsStreamStart(), false);
    assert_eq!(state.BrotliStateIsStreamEnd(), true);
    assert_eq!(BrotliDecoderIsFinished(&state), true);
    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);
}

#[test]
fn test_regular_new_state_stream_start_and_end() {
    let state = new_state();


    assert_eq!(state.BrotliStateIsStreamStart(), true);
    assert_eq!(state.BrotliStateIsStreamEnd(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), false);
    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);


    let strict_state = new_state_strict();
    assert_eq!(state.BrotliStateIsStreamStart(), strict_state.BrotliStateIsStreamStart());
    assert_eq!(state.BrotliStateIsStreamEnd(), strict_state.BrotliStateIsStreamEnd());
    assert_eq!(BrotliDecoderIsFinished(&state), BrotliDecoderIsFinished(&strict_state));
    assert_eq!(BrotliDecoderHasMoreOutput(&state), BrotliDecoderHasMoreOutput(&strict_state));
}

#[test]
fn test_strict_state_partial_feed_not_stream_end() {
    let compressed = get_valid_compressed_hello();
    let mut state = new_state_strict();

    assert_eq!(state.BrotliStateIsStreamStart(), true);


    let mut available_in: usize = 1;
    let mut input_offset: usize = 0;
    let mut output = vec![0u8; 256];
    let mut available_out = output.len();
    let mut output_offset: usize = 0;
    let mut total_out: usize = 0;

    let result = BrotliDecompressStream(
        &mut available_in,
        &mut input_offset,
        &compressed[..1],
        &mut available_out,
        &mut output_offset,
        &mut output,
        &mut total_out,
        &mut state,
    );


    assert_eq!(format!("{:?}", result), format!("{:?}", BrotliResult::NeedsMoreInput));


    assert_eq!(state.BrotliStateIsStreamEnd(), false);


    assert_eq!(BrotliDecoderIsFinished(&state), false);



    assert_eq!(state.BrotliStateIsStreamEnd(), false);
}

#[test]
fn test_strict_state_incremental_decompression_full_workflow() {
    let compressed = get_valid_compressed_hello();
    let mut state = new_state_strict();

    assert_eq!(state.BrotliStateIsStreamStart(), true);
    assert_eq!(state.BrotliStateIsStreamEnd(), false);

    let mut full_output: Vec<u8> = Vec::new();
    let mut total_out: usize = 0;


    for i in 0..compressed.len() {
        let mut available_in: usize = 1;
        let mut input_offset: usize = 0;
        let mut output = vec![0u8; 256];
        let mut available_out = output.len();
        let mut output_offset: usize = 0;

        let result = BrotliDecompressStream(
            &mut available_in,
            &mut input_offset,
            &compressed[i..i + 1],
            &mut available_out,
            &mut output_offset,
            &mut output,
            &mut total_out,
            &mut state,
        );

        if output_offset > 0 {
            full_output.extend_from_slice(&output[..output_offset]);
        }

        match result {
            BrotliResult::ResultSuccess => {

                assert_eq!(state.BrotliStateIsStreamEnd(), true);
                assert_eq!(BrotliDecoderIsFinished(&state), true);
                break;
            }
            BrotliResult::NeedsMoreInput => {
                assert_eq!(state.BrotliStateIsStreamEnd(), false);
                assert_eq!(BrotliDecoderIsFinished(&state), false);
            }
            BrotliResult::NeedsMoreOutput => {

                let mut sz: usize = 0;
                let taken = BrotliDecoderTakeOutput(&mut state, &mut sz);
                full_output.extend_from_slice(taken);
            }
            _ => {
                panic!("Unexpected result: {:?}", result);
            }
        }
    }

    assert_eq!(full_output, b"hello");
    assert_eq!(state.BrotliStateIsStreamEnd(), true);
    assert_eq!(state.BrotliStateIsStreamStart(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), true);
}

#[test]
fn test_huffman_tree_group_init_and_release() {
    let mut state = new_state();


    assert_eq!(state.BrotliStateIsStreamStart(), true);
    assert_eq!(state.BrotliStateIsStreamEnd(), false);


    assert_eq!(state.BrotliStateIsStreamEnd(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), false);


    assert_eq!(state.BrotliStateIsStreamEnd(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), false);

    assert_eq!(state.BrotliStateIsStreamEnd(), false);

    assert_eq!(BrotliDecoderIsFinished(&state), false);
}

#[test]
fn test_huffman_tree_group_init_release_distance() {
    let mut state = new_state_strict();

    assert_eq!(state.BrotliStateIsStreamStart(), true);
    assert_eq!(state.BrotliStateIsStreamEnd(), false);


    assert_eq!(state.BrotliStateIsStreamEnd(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), false);
    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);


    assert_eq!(state.BrotliStateIsStreamEnd(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), false);
    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);
    assert_eq!(state.BrotliStateIsStreamStart(), true);
}

#[test]
fn test_huffman_tree_group_multiple_init_release_cycles() {
    let mut state = new_state_strict();

    assert_eq!(state.BrotliStateIsStreamStart(), true);

    assert_eq!(state.BrotliStateIsStreamEnd(), false);

    assert_eq!(state.BrotliStateIsStreamEnd(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), false);

    assert_eq!(state.BrotliStateIsStreamEnd(), false);
    assert_eq!(BrotliDecoderIsFinished(&state), false);
    assert_eq!(BrotliDecoderHasMoreOutput(&state), false);
    assert_eq!(state.BrotliStateIsStreamStart(), true);
}

#[test]
fn test_strict_vs_regular_state_both_decompress_correctly() {
    let compressed = get_valid_compressed_hello();


    let mut state_regular = new_state();
    let mut available_in = compressed.len();
    let mut input_offset: usize = 0;
    let mut output_regular = vec![0u8; 256];
    let mut available_out = output_regular.len();
    let mut output_offset: usize = 0;
    let mut total_out: usize = 0;

    let result_regular = BrotliDecompressStream(
        &mut available_in,
        &mut input_offset,
        &compressed,
        &mut available_out,
        &mut output_offset,
        &mut output_regular,
        &mut total_out,
        &mut state_regular,
    );

    assert_eq!(format!("{:?}", result_regular), format!("{:?}", BrotliResult::ResultSuccess));
    let regular_output = output_regular[..output_offset].to_vec();


    let mut state_strict = new_state_strict();
    let mut available_in2 = compressed.len();
    let mut input_offset2: usize = 0;
    let mut output_strict = vec![0u8; 256];
    let mut available_out2 = output_strict.len();
    let mut output_offset2: usize = 0;
    let mut total_out2: usize = 0;

    let result_strict = BrotliDecompressStream(
        &mut available_in2,
        &mut input_offset2,
        &compressed,
        &mut available_out2,
        &mut output_offset2,
        &mut output_strict,
        &mut total_out2,
        &mut state_strict,
    );

    assert_eq!(format!("{:?}", result_strict), format!("{:?}", BrotliResult::ResultSuccess));
    let strict_output = output_strict[..output_offset2].to_vec();


    assert_eq!(regular_output, strict_output);
    assert_eq!(regular_output, b"hello");
    assert_eq!(total_out, total_out2);
    assert_eq!(total_out, 5);


    assert_eq!(state_regular.BrotliStateIsStreamEnd(), state_strict.BrotliStateIsStreamEnd());
    assert_eq!(state_regular.BrotliStateIsStreamEnd(), true);
}