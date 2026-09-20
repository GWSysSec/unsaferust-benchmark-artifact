use rgb::GrayAlpha;

#[test]
fn map_alpha_transforms_alpha_only_and_preserves_gray_channel() {
    let source: GrayAlpha<u8, u16> = GrayAlpha(42_u8, 1024_u16);

    let mut callback_seen = None;
    let mapped: GrayAlpha<u8, String> = source.map_alpha(|alpha| {
        callback_seen = Some(alpha);
        format!("opacity:{alpha}")
    });

    assert_eq!(callback_seen, Some(1024_u16));
    assert_eq!(mapped.0, 42_u8);
    assert_eq!(mapped.1, "opacity:1024");
    assert_eq!(source.0, 42_u8);
    assert_eq!(source.1, 1024_u16);
}

#[test]
fn map_gray_transforms_gray_channel_and_converts_alpha_type() {
    let source: GrayAlpha<u8, u8> = GrayAlpha(13_u8, 200_u8);

    let mut callback_seen = None;
    let mapped: GrayAlpha<u16, u16> = source.map_gray(|gray| {
        callback_seen = Some(gray);
        u16::from(gray) * 4 + 1
    });

    assert_eq!(callback_seen, Some(13_u8));
    assert_eq!(mapped.0, 53_u16);
    assert_eq!(mapped.1, 200_u16);
    assert_eq!(source.0, 13_u8);
    assert_eq!(source.1, 200_u8);
}

#[test]
fn chained_gray_and_alpha_mapping_supports_multi_step_pixel_workflow() {
    let original: GrayAlpha<u8, u8> = GrayAlpha(0_u8, 255_u8);

    let normalized: GrayAlpha<f32, u16> = original.map_gray(|gray| f32::from(gray) / 255.0);
    let labeled: GrayAlpha<f32, String> = normalized.map_alpha(|alpha| {
        if alpha == 255_u16 {
            "opaque".to_owned()
        } else {
            format!("alpha={alpha}")
        }
    });

    assert_eq!(original, GrayAlpha(0_u8, 255_u8));
    assert_eq!(normalized.0, 0.0_f32);
    assert_eq!(normalized.1, 255_u16);
    assert_eq!(labeled.0, 0.0_f32);
    assert_eq!(labeled.1, "opaque");
}

#[test]
fn edge_values_are_mapped_without_saturating_or_mutating_source() {
    let black_transparent: GrayAlpha<u8, u8> = GrayAlpha(u8::MIN, u8::MIN);
    let white_opaque: GrayAlpha<u8, u8> = GrayAlpha(u8::MAX, u8::MAX);

    let inverted_black: GrayAlpha<u8, u16> = black_transparent.map_gray(|gray| u8::MAX - gray);
    let scaled_white_alpha: GrayAlpha<u8, u16> =
        white_opaque.map_alpha(|alpha| u16::from(alpha) * 257);

    assert_eq!(inverted_black.0, u8::MAX);
    assert_eq!(inverted_black.1, 0_u16);
    assert_eq!(scaled_white_alpha.0, u8::MAX);
    assert_eq!(scaled_white_alpha.1, 65535_u16);
    assert_eq!(black_transparent, GrayAlpha(0_u8, 0_u8));
    assert_eq!(white_opaque, GrayAlpha(255_u8, 255_u8));
}