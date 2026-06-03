use rgb::Abgr;

#[test]
fn new_abgr_builds_pixel_with_expected_channel_order() {
    const PIXEL: Abgr<u8> = Abgr::new_abgr(255, 10, 20, 30);

    assert_eq!(PIXEL.a, 255);
    assert_eq!(PIXEL.b, 10);
    assert_eq!(PIXEL.g, 20);
    assert_eq!(PIXEL.r, 30);

    let literal = Abgr {
        a: 255,
        b: 10,
        g: 20,
        r: 30,
    };
    assert_eq!(PIXEL, literal);
}

#[test]
fn map_rgb_transforms_only_color_channels_and_preserves_alpha() {
    let source: Abgr<u8> = Abgr::new_abgr(200, 5, 10, 15);

    let mapped: Abgr<u16, u8> = source.map_rgb(|channel| u16::from(channel) * 10);

    assert_eq!(mapped.a, 200_u8);
    assert_eq!(mapped.b, 50);
    assert_eq!(mapped.g, 100);
    assert_eq!(mapped.r, 150);

    assert_eq!(source, Abgr::new_abgr(200, 5, 10, 15));
}

#[test]
fn map_alpha_transforms_only_alpha_and_can_change_alpha_type() {
    let source: Abgr<u8, u16> = Abgr {
        a: 1024_u16,
        b: 1_u8,
        g: 2_u8,
        r: 3_u8,
    };

    let mapped: Abgr<u8, String> = source.map_alpha(|alpha| format!("alpha={alpha}"));

    assert_eq!(mapped.a, "alpha=1024");
    assert_eq!(mapped.b, 1);
    assert_eq!(mapped.g, 2);
    assert_eq!(mapped.r, 3);

    assert_eq!(source.a, 1024);
    assert_eq!(source.b, 1);
    assert_eq!(source.g, 2);
    assert_eq!(source.r, 3);
}

#[test]
fn chained_rgb_and_alpha_mapping_models_pixel_normalization_workflow() {
    let raw: Abgr<u8> = Abgr::new_abgr(128, 0, 127, 255);

    let normalized_rgb: Abgr<f32, u8> = raw.map_rgb(|channel| f32::from(channel) / 255.0);
    let normalized: Abgr<f32, f32> = normalized_rgb.map_alpha(|alpha| f32::from(alpha) / 255.0);

    assert!((normalized.a - (128.0 / 255.0)).abs() < f32::EPSILON);
    assert_eq!(normalized.b, 0.0);
    assert!((normalized.g - (127.0 / 255.0)).abs() < f32::EPSILON);
    assert_eq!(normalized.r, 1.0);
}

#[test]
fn edge_values_are_preserved_or_mapped_without_channel_swapping() {
    let black_transparent: Abgr<u8> = Abgr::new_abgr(0, 0, 0, 0);
    let white_opaque: Abgr<u8> = Abgr::new_abgr(u8::MAX, u8::MAX, u8::MAX, u8::MAX);

    let inverted_black: Abgr<u8> = black_transparent.map_rgb(|channel| u8::MAX - channel);
    let transparent_white: Abgr<u8> = white_opaque.map_alpha(|_| 0);

    assert_eq!(inverted_black.a, 0);
    assert_eq!(inverted_black.b, u8::MAX);
    assert_eq!(inverted_black.g, u8::MAX);
    assert_eq!(inverted_black.r, u8::MAX);

    assert_eq!(transparent_white.a, 0);
    assert_eq!(transparent_white.b, u8::MAX);
    assert_eq!(transparent_white.g, u8::MAX);
    assert_eq!(transparent_white.r, u8::MAX);
}