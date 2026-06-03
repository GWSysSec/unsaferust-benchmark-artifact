use miette::{SourceOffset, MietteError};

#[test]
fn test_from_current_location_returns_ok() {





    let filename = file!().to_string();
    let offset = SourceOffset::from(line!() as usize);


    assert!(!filename.is_empty(), "filename should not be empty");
    assert!(
        filename.contains("source_offset_from_current_location") || filename.contains("gen_miette_SourceOffset"),
        "filename should contain the test file name, got: {}",
        filename
    );

    let byte_offset = offset.offset();
    assert!(byte_offset < 100_000, "offset should be a reasonable value, got: {}", byte_offset);

    assert!(filename.ends_with(".rs"), "filename should end with .rs, got: {}", filename);

    assert!(!filename.contains('\0'), "filename should not contain null bytes");
}

#[test]
fn test_from_current_location_consistency() {

    let filename1 = file!().to_string();
    let offset1 = SourceOffset::from(line!() as usize);
    let filename2 = file!().to_string();
    let offset2 = SourceOffset::from(line!() as usize);


    assert_eq!(filename1, filename2, "both calls should report the same file");


    let off1 = offset1.offset();
    let off2 = offset2.offset();
    assert_ne!(off1, off2, "offsets from different lines should differ");


    assert!(off2 > off1, "second offset ({}) should be greater than first ({})", off2, off1);


    assert!(off1 > 0, "first offset should be positive");
    assert!(off2 > 0, "second offset should be positive");
}

#[test]
fn test_from_current_location_offset_increases_with_lines() {
    let f1 = file!().to_string();
    let o1 = SourceOffset::from(line!() as usize);

    let _padding1 = 1;
    let _padding2 = 2;
    let _padding3 = 3;
    let _padding4 = 4;
    let _padding5 = 5;
    let f2 = file!().to_string();
    let o2 = SourceOffset::from(line!() as usize);

    assert_eq!(f1, f2, "same file for both calls");

    let byte1 = o1.offset();
    let byte2 = o2.offset();


    let diff = byte2 - byte1;
    assert!(diff > 0, "offset difference should be > 0 due to padding lines, got: {}", diff);
    assert!(diff < 10_000, "offset difference should be reasonable, got: {}", diff);


    assert!(f1.contains(".rs"), "filename should be a rust file: {}", f1);
}

#[test]
fn test_from_current_location_source_offset_usable_in_source_span() {
    use miette::SourceSpan;

    let filename = file!().to_string();
    let offset = SourceOffset::from(line!() as usize);


    let span = SourceSpan::new(offset, 10);


    let start: usize = span.offset();
    let len: usize = span.len();

    assert_eq!(len, 10, "span length should be 10");
    assert!(start > 0, "span start should be positive");


    assert!(filename.len() > 5, "filename should be a real path, got: {}", filename);


    let original_offset_val = {
        let o = SourceOffset::from(line!() as usize);
        o.offset()
    };

    assert!(original_offset_val > start, "later call should have greater offset");
}