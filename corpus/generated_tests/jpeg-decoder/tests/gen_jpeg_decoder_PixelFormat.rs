use jpeg_decoder as jpeg;
use jpeg_decoder::PixelFormat;
use std::fs::File;
use std::path::Path;

#[test]
fn pixel_bytes_l8_format() {
    // Decode a known grayscale JPEG to get L8 pixel format
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-gray.jpg");

    let mut decoder = jpeg::Decoder::new(File::open(&path).unwrap());
    decoder.decode().unwrap();
    let info = decoder.info().unwrap();

    // Grayscale images should have pixel_format L8 with 1 byte per pixel
    assert_eq!(info.pixel_format, PixelFormat::L8);
    assert_eq!(info.pixel_format.pixel_bytes(), 1);

    // Verify clone behavior of ImageInfo
    let info_clone = info.clone();
    assert_eq!(info_clone.pixel_format, info.pixel_format);
    assert_eq!(info_clone.pixel_format.pixel_bytes(), 1);
    assert_eq!(info_clone.width, info.width);
    assert_eq!(info_clone.height, info.height);

    // Verify the decoded data length matches dimensions * pixel_bytes
    let mut decoder2 = jpeg::Decoder::new(File::open(&path).unwrap());
    let data = decoder2.decode().unwrap();
    let info2 = decoder2.info().unwrap();
    let expected_len = info2.width as usize * info2.height as usize * info2.pixel_format.pixel_bytes();
    assert_eq!(data.len(), expected_len);
}

#[test]
fn pixel_bytes_rgb24_format() {
    // Decode a known color JPEG to get RGB24 pixel format
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");

    let mut decoder = jpeg::Decoder::new(File::open(&path).unwrap());
    let data = decoder.decode().unwrap();
    let info = decoder.info().unwrap();

    // Color images should have pixel_format RGB24 with 3 bytes per pixel
    assert_eq!(info.pixel_format, PixelFormat::RGB24);
    assert_eq!(info.pixel_format.pixel_bytes(), 3);

    // Verify the decoded data length matches dimensions * pixel_bytes
    let expected_len = info.width as usize * info.height as usize * info.pixel_format.pixel_bytes();
    assert_eq!(data.len(), expected_len);

    // Verify that pixel_bytes is consistent across clones
    let info_clone = info.clone();
    assert_eq!(info_clone.pixel_format.pixel_bytes(), 3);
    assert_eq!(info_clone.pixel_format.pixel_bytes(), info.pixel_format.pixel_bytes());

    // Verify dimensions are positive
    assert!(info.width > 0);
    assert!(info.height > 0);
}

#[test]
fn pixel_bytes_cmyk32_format() {
    // Decode a known CMYK/YCCK JPEG to get CMYK32 pixel format
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("ycck.jpg");

    let mut decoder = jpeg::Decoder::new(File::open(&path).unwrap());
    let data = decoder.decode().unwrap();
    let info = decoder.info().unwrap();

    // YCCK images should have pixel_format CMYK32 with 4 bytes per pixel
    assert_eq!(info.pixel_format, PixelFormat::CMYK32);
    assert_eq!(info.pixel_format.pixel_bytes(), 4);

    // Verify the decoded data length matches dimensions * pixel_bytes
    let expected_len = info.width as usize * info.height as usize * info.pixel_format.pixel_bytes();
    assert_eq!(data.len(), expected_len);

    // Verify consistency
    let cloned_format = info.pixel_format.clone();
    assert_eq!(cloned_format.pixel_bytes(), 4);
    assert_eq!(cloned_format, PixelFormat::CMYK32);

    // Verify that different formats have different pixel_bytes
    assert_ne!(PixelFormat::L8.pixel_bytes(), PixelFormat::RGB24.pixel_bytes());
    assert_ne!(PixelFormat::RGB24.pixel_bytes(), PixelFormat::CMYK32.pixel_bytes());
    assert_ne!(PixelFormat::L8.pixel_bytes(), PixelFormat::CMYK32.pixel_bytes());
}

#[test]
fn pixel_bytes_all_variants_consistency() {
    // Exhaustively test all PixelFormat variants for correct pixel_bytes values
    let l8 = PixelFormat::L8;
    let l16 = PixelFormat::L16;
    let rgb24 = PixelFormat::RGB24;
    let cmyk32 = PixelFormat::CMYK32;

    // L8: 1 byte per pixel (8-bit grayscale)
    assert_eq!(l8.pixel_bytes(), 1);

    // L16: 2 bytes per pixel (16-bit grayscale)
    assert_eq!(l16.pixel_bytes(), 2);

    // RGB24: 3 bytes per pixel (8-bit per channel, 3 channels)
    assert_eq!(rgb24.pixel_bytes(), 3);

    // CMYK32: 4 bytes per pixel (8-bit per channel, 4 channels)
    assert_eq!(cmyk32.pixel_bytes(), 4);

    // Verify ordering: L8 < L16 < RGB24 < CMYK32
    assert!(l8.pixel_bytes() < l16.pixel_bytes());
    assert!(l16.pixel_bytes() < rgb24.pixel_bytes());
    assert!(rgb24.pixel_bytes() < cmyk32.pixel_bytes());

    // Verify clones produce same results
    assert_eq!(l8.clone().pixel_bytes(), 1);
    assert_eq!(l16.clone().pixel_bytes(), 2);
    assert_eq!(rgb24.clone().pixel_bytes(), 3);
    assert_eq!(cmyk32.clone().pixel_bytes(), 4);
}

#[test]
fn pixel_bytes_used_for_buffer_calculation() {
    // Test that pixel_bytes can be used to correctly calculate buffer sizes
    // for multiple different images
    let paths_and_expected_formats: Vec<(&str, PixelFormat, usize)> = vec![
        ("tests/reftest/images/mozilla/jpg-gray.jpg", PixelFormat::L8, 1),
        ("tests/reftest/images/mozilla/jpg-progressive.jpg", PixelFormat::RGB24, 3),
        ("tests/reftest/images/ycck.jpg", PixelFormat::CMYK32, 4),
    ];

    for (path_str, expected_format, expected_bytes) in &paths_and_expected_formats {
        let path = Path::new(path_str);
        if !path.exists() {
            continue;
        }

        let mut decoder = jpeg::Decoder::new(File::open(path).unwrap());
        let data = decoder.decode().unwrap();
        let info = decoder.info().unwrap();

        assert_eq!(info.pixel_format, *expected_format);
        assert_eq!(info.pixel_format.pixel_bytes(), *expected_bytes);

        let calculated_size = info.width as usize * info.height as usize * info.pixel_format.pixel_bytes();
        assert_eq!(data.len(), calculated_size);

        // Verify we can use pixel_bytes to index into the buffer correctly
        // First pixel should be accessible
        assert!(data.len() >= info.pixel_format.pixel_bytes());

        // Last pixel should be at the expected offset
        let last_pixel_offset = (info.width as usize * info.height as usize - 1) * info.pixel_format.pixel_bytes();
        assert!(last_pixel_offset + info.pixel_format.pixel_bytes() <= data.len());
    }
}

#[test]
fn pixel_bytes_with_read_info_before_decode() {
    // Test that pixel_bytes is available after read_info without full decode
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");

    let mut decoder = jpeg::Decoder::new(File::open(&path).unwrap());
    decoder.read_info().unwrap();
    let info = decoder.info().unwrap();

    // pixel_bytes should be available from info obtained via read_info
    assert_eq!(info.pixel_format, PixelFormat::RGB24);
    assert_eq!(info.pixel_format.pixel_bytes(), 3);

    // Now decode and verify consistency
    let data = decoder.decode().unwrap();
    let info_after = decoder.info().unwrap();

    assert_eq!(info_after.pixel_format.pixel_bytes(), info.pixel_format.pixel_bytes());
    assert_eq!(info_after.pixel_format, info.pixel_format);

    let expected_len = info_after.width as usize * info_after.height as usize * info_after.pixel_format.pixel_bytes();
    assert_eq!(data.len(), expected_len);
    assert_eq!(info.width, info_after.width);
    assert_eq!(info.height, info_after.height);
}