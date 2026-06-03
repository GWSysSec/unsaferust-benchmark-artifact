use rgb::{Gray, GrayAlpha};

#[test]
fn map_alpha_can_change_alpha_type_while_preserving_gray_value_and_source() {
    let source: GrayAlpha<u8, u16> = GrayAlpha(64_u8, 1_000_u16);

    let mut callback_input = None;
    let mapped: GrayAlpha<u8, String> = source.map_alpha(|alpha| {
        callback_input = Some(alpha);
        format!("{:.2}", f32::from(alpha) / 1000.0)
    });

    assert_eq!(callback_input, Some(1_000_u16));
    assert_eq!(mapped.gray(), Gray::new(64_u8));
    assert_eq!(mapped.0, 64_u8);
    assert_eq!(mapped.1, "1.00");
    assert_eq!(source.gray(), Gray::new(64_u8));
    assert_eq!(source.1, 1_000_u16);
}

#[test]
fn map_gray_can_change_gray_type_and_converts_alpha_with_from() {
    let source: GrayAlpha<u8, u8> = GrayAlpha::new(13_u8, 200_u8);

    let mut callback_input = None;
    let mapped: GrayAlpha<u16, u16> = source.map_gray(|gray| {
        callback_input = Some(gray);
        u16::from(gray) * 4 + 1
    });

    assert_eq!(callback_input, Some(13_u8));
    assert_eq!(mapped.gray(), Gray::new(53_u16));
    assert_eq!(mapped.0, 53_u16);
    assert_eq!(mapped.1, 200_u16);
    assert_eq!(source.gray(), Gray::new(13_u8));
    assert_eq!(source.1, 200_u8);
}

#[test]
fn chained_grayalpha_mapping_models_normalize_threshold_and_label_workflow() {
    let raw: GrayAlpha<u8, u8> = GrayAlpha::new(0_u8, 255_u8);

    let normalized: GrayAlpha<f32, u16> = raw.map_gray(|value| f32::from(value) / 255.0);
    assert_eq!(normalized.0, 0.0);
    assert_eq!(normalized.1, 255_u16);

    let labeled: GrayAlpha<f32, &'static str> =
        normalized.map_alpha(|alpha| if alpha == 255_u16 { "opaque" } else { "partial" });
    assert_eq!(labeled.0, 0.0);
    assert_eq!(labeled.1, "opaque");

    let brightened_gray: GrayAlpha<u8, &'static str> =
        labeled.map_gray(|value| ((value + 0.25).clamp(0.0, 1.0) * 255.0).round() as u8);

    let brightened: GrayAlpha<u8, String> =
        brightened_gray.map_alpha(|label| format!("visibility:{label}"));

    assert_eq!(brightened.gray(), Gray::new(64_u8));
    assert_eq!(brightened.0, 64_u8);
    assert_eq!(brightened.1, String::from("visibility:opaque"));

    assert_eq!(raw.gray(), Gray::new(0_u8));
    assert_eq!(raw.1, 255_u8);
}

#[test]
fn map_gray_handles_extreme_channel_values_without_mutating_original() {
    let black_transparent: GrayAlpha<u8, u8> = GrayAlpha::new(u8::MIN, u8::MIN);
    let white_opaque: GrayAlpha<u8, u8> = GrayAlpha::new(u8::MAX, u8::MAX);

    let black_inverted: GrayAlpha<u8, u16> = black_transparent.map_gray(|gray| u8::MAX - gray);
    let white_inverted: GrayAlpha<u8, u16> = white_opaque.map_gray(|gray| u8::MAX - gray);

    assert_eq!(black_inverted.gray(), Gray::new(u8::MAX));
    assert_eq!(black_inverted.1, 0_u16);
    assert_eq!(white_inverted.gray(), Gray::new(u8::MIN));
    assert_eq!(white_inverted.1, 255_u16);

    assert_eq!(black_transparent, GrayAlpha::new(0_u8, 0_u8));
    assert_eq!(white_opaque, GrayAlpha::new(255_u8, 255_u8));
}

#[test]
fn map_alpha_runs_once_and_receives_owned_alpha_value() {
    let source: GrayAlpha<u8, Vec<&'static str>> = GrayAlpha(7_u8, vec!["matte", "masked"]);

    let mut call_count = 0_usize;
    let mapped: GrayAlpha<u8, usize> = source.map_alpha(|mut tags| {
        call_count += 1;
        tags.push("processed");
        tags.len()
    });

    assert_eq!(call_count, 1);
    assert_eq!(mapped.gray(), Gray::new(7_u8));
    assert_eq!(mapped.1, 3);
    assert_eq!(source.gray(), Gray::new(7_u8));
    assert_eq!(source.1, vec!["matte", "masked"]);
}