use std::fs::File;
use std::io::Cursor;
use std::path::Path;

use jpeg_decoder as jpeg;
use jpeg_decoder::{Decoder, PixelFormat, ColorTransform};

#[test]
fn test_set_color_transform_rgb_decode() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");


    let mut decoder = Decoder::new(File::open(&path).unwrap());
    let baseline_data = decoder.decode().unwrap();
    let baseline_info = decoder.info().unwrap();


    let mut decoder2 = Decoder::new(File::open(&path).unwrap());
    decoder2.set_color_transform(ColorTransform::RGB);
    let rgb_data = decoder2.decode().unwrap();
    let rgb_info = decoder2.info().unwrap();


    assert_eq!(baseline_info.width, rgb_info.width);
    assert_eq!(baseline_info.height, rgb_info.height);
    assert!(baseline_info.width > 0);
    assert!(baseline_info.height > 0);
    assert!(!baseline_data.is_empty());
    assert!(!rgb_data.is_empty());

    assert_eq!(baseline_data.len(), rgb_data.len());

    assert_eq!(baseline_info.pixel_format, rgb_info.pixel_format);
}

#[test]
fn test_set_color_transform_cmyk() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("ycck.jpg");


    let mut decoder = Decoder::new(File::open(&path).unwrap());
    decoder.set_color_transform(ColorTransform::CMYK);
    let result = decoder.decode();


    match result {
        Ok(data) => {
            let info = decoder.info().unwrap();
            assert!(info.width > 0);
            assert!(info.height > 0);
            assert!(!data.is_empty());
            let pixel_bytes = info.pixel_format.pixel_bytes();
            assert!(pixel_bytes > 0);
            assert_eq!(data.len(), info.width as usize * info.height as usize * pixel_bytes);

            assert_eq!(pixel_bytes, 4);
            assert!(data.len() >= 4);
        }
        Err(_) => {


            let mut decoder2 = Decoder::new(File::open(&path).unwrap());
            let data2 = decoder2.decode().unwrap();
            let info2 = decoder2.info().unwrap();
            assert!(info2.width > 0);
            assert!(info2.height > 0);
            assert!(!data2.is_empty());
            let pixel_bytes = info2.pixel_format.pixel_bytes();
            assert!(pixel_bytes > 0);
            assert_eq!(data2.len(), info2.width as usize * info2.height as usize * pixel_bytes);
        }
    }
}

#[test]
fn test_set_color_transform_ycck() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("ycck.jpg");

    let mut decoder_none = Decoder::new(File::open(&path).unwrap());
    decoder_none.set_color_transform(ColorTransform::YCCK);
    let result_ycck = decoder_none.decode();

    let mut decoder_baseline = Decoder::new(File::open(&path).unwrap());
    let baseline_data = decoder_baseline.decode().unwrap();
    let baseline_info = decoder_baseline.info().unwrap();

    assert!(baseline_info.width > 0);
    assert!(baseline_info.height > 0);
    assert!(!baseline_data.is_empty());

    match result_ycck {
        Ok(ycck_data) => {
            let ycck_info = decoder_none.info().unwrap();
            assert_eq!(ycck_info.width, baseline_info.width);
            assert_eq!(ycck_info.height, baseline_info.height);
            assert!(!ycck_data.is_empty());

            let pixel_bytes = ycck_info.pixel_format.pixel_bytes();
            assert!(pixel_bytes > 0);
            assert_eq!(ycck_data.len(), ycck_info.width as usize * ycck_info.height as usize * pixel_bytes);
        }
        Err(_) => {

            let pixel_bytes = baseline_info.pixel_format.pixel_bytes();
            assert!(pixel_bytes > 0);
            assert_eq!(baseline_data.len(), baseline_info.width as usize * baseline_info.height as usize * pixel_bytes);
        }
    }
}

#[test]
fn test_set_max_decoding_buffer_size_too_small() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");


    let mut decoder_baseline = Decoder::new(File::open(&path).unwrap());
    let baseline_data = decoder_baseline.decode().unwrap();
    let baseline_info = decoder_baseline.info().unwrap();
    let actual_size = baseline_data.len();

    assert!(actual_size > 0);
    assert!(baseline_info.width > 0);
    assert!(baseline_info.height > 0);


    let mut decoder_limited = Decoder::new(File::open(&path).unwrap());
    decoder_limited.set_max_decoding_buffer_size(1);
    let result = decoder_limited.decode();


    assert!(result.is_err());


    let err = result.unwrap_err();
    let err_string = format!("{}", err);
    assert!(!err_string.is_empty());
}

#[test]
fn test_set_max_decoding_buffer_size_exact() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");


    let mut decoder_baseline = Decoder::new(File::open(&path).unwrap());
    let baseline_data = decoder_baseline.decode().unwrap();
    let actual_size = baseline_data.len();
    let baseline_info = decoder_baseline.info().unwrap();

    assert!(actual_size > 100);
    assert!(baseline_info.width > 0);
    assert!(baseline_info.height > 0);


    let mut decoder_exact = Decoder::new(File::open(&path).unwrap());
    decoder_exact.set_max_decoding_buffer_size(actual_size);
    let result_exact = decoder_exact.decode();
    assert!(result_exact.is_ok());
    let exact_data = result_exact.unwrap();
    assert_eq!(exact_data.len(), actual_size);
    assert_eq!(exact_data, baseline_data);


    let mut decoder_one_less = Decoder::new(File::open(&path).unwrap());
    decoder_one_less.set_max_decoding_buffer_size(actual_size - 1);
    let result_one_less = decoder_one_less.decode();
    assert!(result_one_less.is_err());
}

#[test]
fn test_set_max_decoding_buffer_size_generous() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");


    let mut decoder = Decoder::new(File::open(&path).unwrap());
    decoder.set_max_decoding_buffer_size(100 * 1024 * 1024);
    let data = decoder.decode().unwrap();
    let info = decoder.info().unwrap();

    assert!(!data.is_empty());
    assert!(info.width > 0);
    assert!(info.height > 0);

    let pixel_bytes = info.pixel_format.pixel_bytes();
    assert!(pixel_bytes >= 1);
    assert!(pixel_bytes <= 4);
    let expected_len = info.width as usize * info.height as usize * pixel_bytes;
    assert_eq!(data.len(), expected_len);


    let non_zero_count = data.iter().filter(|&&b| b != 0).count();
    assert!(non_zero_count > 0);
}

#[test]
fn test_set_color_transform_then_max_buffer_combined() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");


    let mut decoder = Decoder::new(File::open(&path).unwrap());
    decoder.set_color_transform(ColorTransform::RGB);
    decoder.set_max_decoding_buffer_size(50 * 1024 * 1024);
    let data = decoder.decode().unwrap();
    let info = decoder.info().unwrap();

    assert!(!data.is_empty());
    assert!(info.width > 0);
    assert!(info.height > 0);

    let pixel_bytes = info.pixel_format.pixel_bytes();
    assert_eq!(data.len(), info.width as usize * info.height as usize * pixel_bytes);


    let mut decoder_fail = Decoder::new(File::open(&path).unwrap());
    decoder_fail.set_color_transform(ColorTransform::RGB);
    decoder_fail.set_max_decoding_buffer_size(10);
    let result = decoder_fail.decode();
    assert!(result.is_err());
}

#[test]
fn test_set_max_decoding_buffer_size_zero() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");


    let mut decoder = Decoder::new(File::open(&path).unwrap());
    decoder.set_max_decoding_buffer_size(0);
    let result = decoder.decode();
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(!err_msg.is_empty());


    let mut decoder_normal = Decoder::new(File::open(&path).unwrap());
    let normal_data = decoder_normal.decode().unwrap();
    assert!(!normal_data.is_empty());

    let normal_info = decoder_normal.info().unwrap();
    assert!(normal_info.width > 0);
    assert!(normal_info.height > 0);
    let pixel_bytes = normal_info.pixel_format.pixel_bytes();
    assert!(pixel_bytes >= 1);
}

#[test]
fn test_pixel_format_pixel_bytes_values() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");

    let mut decoder = Decoder::new(File::open(&path).unwrap());
    decoder.decode().unwrap();
    let info = decoder.info().unwrap();


    let pb = info.pixel_format.pixel_bytes();
    assert!(pb == 1 || pb == 3 || pb == 4);


    assert_eq!(pb, 3);
    assert_eq!(info.pixel_format, PixelFormat::RGB24);


    assert_eq!(PixelFormat::L8.pixel_bytes(), 1);
    assert_eq!(PixelFormat::L16.pixel_bytes(), 2);
    assert_eq!(PixelFormat::RGB24.pixel_bytes(), 3);
    assert_eq!(PixelFormat::CMYK32.pixel_bytes(), 4);
}

#[test]
fn test_color_transform_clone() {
    let transform = ColorTransform::RGB;
    let cloned = transform.clone();
    assert_eq!(format!("{:?}", transform), format!("{:?}", cloned));

    let transform_cmyk = ColorTransform::CMYK;
    let cloned_cmyk = transform_cmyk.clone();
    assert_eq!(format!("{:?}", transform_cmyk), format!("{:?}", cloned_cmyk));
    assert_ne!(format!("{:?}", transform), format!("{:?}", transform_cmyk));

    let transform_ycck = ColorTransform::YCCK;
    let cloned_ycck = transform_ycck.clone();
    assert_eq!(format!("{:?}", transform_ycck), format!("{:?}", cloned_ycck));
    assert_ne!(format!("{:?}", transform_ycck), format!("{:?}", transform));
    assert_ne!(format!("{:?}", transform_ycck), format!("{:?}", transform_cmyk));


    let rgb_str = format!("{:?}", ColorTransform::RGB);
    let cmyk_str = format!("{:?}", ColorTransform::CMYK);
    let ycck_str = format!("{:?}", ColorTransform::YCCK);
    assert!(rgb_str.contains("RGB"));
    assert!(cmyk_str.contains("CMYK"));
    assert!(ycck_str.contains("YCCK"));
}

#[test]
fn test_set_color_transform_multiple_images() {
    let path_progressive = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");

    let path_ycck = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("ycck.jpg");


    let mut dec1 = Decoder::new(File::open(&path_progressive).unwrap());
    dec1.set_color_transform(ColorTransform::RGB);
    let data1 = dec1.decode().unwrap();
    let info1 = dec1.info().unwrap();


    let mut dec2 = Decoder::new(File::open(&path_ycck).unwrap());
    let data2 = dec2.decode().unwrap();
    let info2 = dec2.info().unwrap();


    assert!(!data1.is_empty());
    assert!(!data2.is_empty());
    assert!(info1.width > 0);
    assert!(info2.width > 0);
    assert!(info1.height > 0);
    assert!(info2.height > 0);


    assert_ne!(data1.len(), data2.len());
}

#[test]
fn test_set_max_decoding_buffer_read_info_then_decode() {
    let path = Path::new("tests")
        .join("reftest")
        .join("images")
        .join("mozilla")
        .join("jpg-progressive.jpg");


    let mut decoder = Decoder::new(File::open(&path).unwrap());
    decoder.set_max_decoding_buffer_size(50 * 1024 * 1024);
    decoder.read_info().unwrap();
    let info = decoder.info().unwrap();

    assert!(info.width > 0);
    assert!(info.height > 0);

    let expected_size = info.width as usize * info.height as usize * info.pixel_format.pixel_bytes();
    assert!(expected_size > 0);
    assert!(expected_size < 50 * 1024 * 1024);

    let data = decoder.decode().unwrap();
    assert_eq!(data.len(), expected_size);


    let mut decoder2 = Decoder::new(File::open(&path).unwrap());
    decoder2.set_max_decoding_buffer_size(10);

    let read_info_result = decoder2.read_info();

    if read_info_result.is_ok() {
        let result = decoder2.decode();
        assert!(result.is_err());
    }
}