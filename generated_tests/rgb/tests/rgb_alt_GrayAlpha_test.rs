use rgb::alt::{Gray, GrayAlpha};

#[test]
fn map_alpha_changes_alpha_type_without_changing_gray_value() {
    let source: GrayAlpha<u16, u16> = GrayAlpha::new(42_u16, 1_024_u16);

    let mut alpha_seen_by_callback = None;
    let mapped: GrayAlpha<u16, String> = source.map_alpha(|alpha| {
        alpha_seen_by_callback = Some(alpha);
        format!("alpha={alpha}")
    });

    assert_eq!(alpha_seen_by_callback, Some(1_024_u16));
    assert_eq!(mapped.gray(), Gray::new(42_u16));
    assert_eq!(source.gray(), Gray::new(42_u16));

    let mut remapped_alpha_seen = None;
    let remapped: GrayAlpha<u16, usize> = mapped.map_alpha(|alpha| {
        remapped_alpha_seen = Some(alpha.clone());
        alpha.len()
    });

    assert_eq!(remapped_alpha_seen, Some(String::from("alpha=1024")));
    assert_eq!(remapped.gray(), Gray::new(42_u16));

    let mut final_alpha_seen = None;
    let _final_value: GrayAlpha<u16, usize> = remapped.map_alpha(|alpha| {
        final_alpha_seen = Some(alpha);
        alpha
    });
    assert_eq!(final_alpha_seen, Some("alpha=1024".len()));
}

#[test]
fn map_gray_changes_gray_type_and_converts_alpha_type() {
    let source: GrayAlpha<u8, u8> = GrayAlpha::new(13_u8, 200_u8);

    let mut gray_seen_by_callback = None;
    let mapped: GrayAlpha<u16, u16> = source.map_gray(|gray| {
        gray_seen_by_callback = Some(gray);
        u16::from(gray) * 4 + 1
    });

    assert_eq!(gray_seen_by_callback, Some(13_u8));
    assert_eq!(mapped.gray(), Gray::new(53_u16));
    assert_eq!(source.gray(), Gray::new(13_u8));

    let mut converted_alpha_seen = None;
    let _alpha_checked: GrayAlpha<u16, u16> = mapped.map_alpha(|alpha| {
        converted_alpha_seen = Some(alpha);
        alpha
    });

    assert_eq!(converted_alpha_seen, Some(200_u16));
}

#[test]
fn chained_gray_and_alpha_mapping_models_normalization_workflow() {
    let original: GrayAlpha<u8, u8> = GrayAlpha::new(0_u8, 255_u8);

    let normalized: GrayAlpha<f32, u8> = original.map_gray(|gray| f32::from(gray) / 255.0);
    assert_eq!(normalized.gray(), Gray::new(0.0_f32));

    let labeled: GrayAlpha<f32, String> = normalized.map_alpha(|alpha| {
        if alpha == 255 {
            String::from("opaque")
        } else {
            format!("alpha:{alpha}")
        }
    });

    assert_eq!(labeled.gray(), Gray::new(0.0_f32));

    let mut label_seen = None;
    let relabeled: GrayAlpha<f32, bool> = labeled.map_alpha(|label| {
        label_seen = Some(label.clone());
        label == "opaque"
    });

    assert_eq!(label_seen, Some(String::from("opaque")));
    assert_eq!(relabeled.gray(), Gray::new(0.0_f32));

    let mut final_alpha_seen = None;
    let _checked: GrayAlpha<f32, bool> = relabeled.map_alpha(|is_opaque| {
        final_alpha_seen = Some(is_opaque);
        is_opaque
    });

    assert_eq!(final_alpha_seen, Some(true));
}

#[test]
fn edge_values_are_preserved_through_identity_and_widening_mappings() {
    let black_transparent: GrayAlpha<u8, u8> = GrayAlpha::new(u8::MIN, u8::MIN);
    let white_opaque: GrayAlpha<u8, u8> = GrayAlpha::new(u8::MAX, u8::MAX);

    let widened_black: GrayAlpha<u16, u16> = black_transparent.map_gray(u16::from);
    let widened_white: GrayAlpha<u16, u16> = white_opaque.map_gray(u16::from);

    assert_eq!(widened_black.gray(), Gray::new(0_u16));
    assert_eq!(widened_white.gray(), Gray::new(255_u16));

    let mut black_alpha_seen = None;
    let _black_checked: GrayAlpha<u16, u16> = widened_black.map_alpha(|alpha| {
        black_alpha_seen = Some(alpha);
        alpha
    });

    let mut white_alpha_seen = None;
    let _white_checked: GrayAlpha<u16, u16> = widened_white.map_alpha(|alpha| {
        white_alpha_seen = Some(alpha);
        alpha
    });

    assert_eq!(black_alpha_seen, Some(0_u16));
    assert_eq!(white_alpha_seen, Some(255_u16));
}