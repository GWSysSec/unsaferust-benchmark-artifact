use byteorder::{BigEndian, ByteOrder, LittleEndian};
use std::sync::atomic::{AtomicUsize, Ordering};

static PROPERTY_RUNS: AtomicUsize = AtomicUsize::new(0);

fn endian_roundtrip_property(a: u16, b: u32, c: i64) -> bool {
    PROPERTY_RUNS.fetch_add(1, Ordering::SeqCst);

    let mut bytes = [0u8; 14];

    <BigEndian as ByteOrder>::write_u16(&mut bytes[0..2], a);
    <LittleEndian as ByteOrder>::write_u32(&mut bytes[2..6], b);
    <BigEndian as ByteOrder>::write_i64(&mut bytes[6..14], c);

    let decoded_a = <BigEndian as ByteOrder>::read_u16(&bytes[0..2]);
    let decoded_b = <LittleEndian as ByteOrder>::read_u32(&bytes[2..6]);
    let decoded_c = <BigEndian as ByteOrder>::read_i64(&bytes[6..14]);

    decoded_a == a && decoded_b == b && decoded_c == c
}

fn run_sized_property(property: fn(u16, u32, i64) -> bool, runs: usize) {
    let mut state = 0x1234_5678_9ABC_DEF0u64;

    for _ in 0..runs {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a = state as u16;

        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let b = state as u32;

        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let c = state as i64;

        assert!(property(a, b, c));
    }
}

#[test]
fn qc_sized_runs_byteorder_roundtrip_property() {
    PROPERTY_RUNS.store(0, Ordering::SeqCst);

    run_sized_property(endian_roundtrip_property as fn(u16, u32, i64) -> bool, 32);

    assert!(PROPERTY_RUNS.load(Ordering::SeqCst) > 0);
}

#[test]
fn mixed_endian_buffer_workflow_has_expected_layout_and_values() {
    let mut packet = vec![0u8; 20];

    <BigEndian as ByteOrder>::write_u16(&mut packet[0..2], 0x1234);
    <LittleEndian as ByteOrder>::write_u32(&mut packet[2..6], 0x89ABCDEF);
    <BigEndian as ByteOrder>::write_i64(&mut packet[6..14], -0x0102_0304_0506_0708);
    <LittleEndian as ByteOrder>::write_u16(&mut packet[14..16], 0xBEEF);
    <BigEndian as ByteOrder>::write_u32(&mut packet[16..20], 0x0A0B0C0D);

    assert_eq!(&packet[0..2], &[0x12, 0x34]);
    assert_eq!(&packet[2..6], &[0xEF, 0xCD, 0xAB, 0x89]);
    assert_eq!(<BigEndian as ByteOrder>::read_u16(&packet[0..2]), 0x1234);
    assert_eq!(
        <LittleEndian as ByteOrder>::read_u32(&packet[2..6]),
        0x89ABCDEF
    );
    assert_eq!(
        <BigEndian as ByteOrder>::read_i64(&packet[6..14]),
        -0x0102_0304_0506_0708
    );
    assert_eq!(
        <LittleEndian as ByteOrder>::read_u16(&packet[14..16]),
        0xBEEF
    );
    assert_eq!(
        <BigEndian as ByteOrder>::read_u32(&packet[16..20]),
        0x0A0B0C0D
    );

    let wrong_endian_header = <LittleEndian as ByteOrder>::read_u16(&packet[0..2]);
    assert_ne!(wrong_endian_header, 0x1234);
    assert_eq!(wrong_endian_header, 0x3412);
}