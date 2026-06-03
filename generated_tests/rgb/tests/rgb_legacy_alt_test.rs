use rgb::{Gray, GrayAlpha};

#[test]
#[allow(deprecated)]
fn legacy_alpha_replaces_alpha_and_preserves_gray_in_workflow() {
    let original: GrayAlpha<u8, u16> = GrayAlpha(87_u8, 100_u16);

    let darker_step: GrayAlpha<u16, u16> = original.map_gray(|gray| u16::from(gray) / 3);
    let re_alphaed: GrayAlpha<u16, u16> = darker_step.alpha(900_u16);

    assert_eq!(original.gray(), Gray::new(87_u8));
    assert_eq!(original.1, 100_u16);
    assert_eq!(darker_step.gray(), Gray::new(29_u16));
    assert_eq!(darker_step.1, 100_u16);
    assert_eq!(re_alphaed.gray(), Gray::new(29_u16));
    assert_eq!(re_alphaed.1, 900_u16);
}

#[test]
#[allow(deprecated)]
fn legacy_map_alpha_transforms_alpha_only_and_can_be_chained() {
    let source: GrayAlpha<u8, u16> = GrayAlpha(42_u8, 1_024_u16);

    let mut first_callback_seen = None;
    let labeled: GrayAlpha<u8, String> = source.map_alpha(|alpha| {
        first_callback_seen = Some(alpha);
        format!("opacity={alpha}")
    });

    let mut second_callback_seen = None;
    let measured: GrayAlpha<u8, usize> = labeled.clone().map_alpha(|label| {
        second_callback_seen = Some(label.clone());
        label.len()
    });

    assert_eq!(first_callback_seen, Some(1_024_u16));
    assert_eq!(second_callback_seen, Some(String::from("opacity=1024")));
    assert_eq!(source.gray(), Gray::new(42_u8));
    assert_eq!(source.1, 1_024_u16);
    assert_eq!(labeled.gray(), Gray::new(42_u8));
    assert_eq!(labeled.1, "opacity=1024");
    assert_eq!(measured.gray(), Gray::new(42_u8));
    assert_eq!(measured.1, "opacity=1024".len());
}

#[test]
#[allow(deprecated)]
fn legacy_map_gray_transforms_gray_and_converts_alpha_with_from() {
    let source: GrayAlpha<u8, u8> = GrayAlpha::new(13_u8, 200_u8);

    let mut gray_seen = None;
    let widened: GrayAlpha<u16, u16> = source.map_gray(|gray| {
        gray_seen = Some(gray);
        u16::from(gray) * 4 + 1
    });

    let re_alphaed: GrayAlpha<u16, u16> = widened.alpha(255_u16);
    let described: GrayAlpha<String, u16> = re_alphaed.map_gray(|gray| {
        if gray >= 50 {
            format!("bright:{gray}")
        } else {
            format!("dark:{gray}")
        }
    });

    assert_eq!(gray_seen, Some(13_u8));
    assert_eq!(source.gray(), Gray::new(13_u8));
    assert_eq!(source.1, 200_u8);
    assert_eq!(widened.gray(), Gray::new(53_u16));
    assert_eq!(widened.1, 200_u16);
    assert_eq!(re_alphaed.gray(), Gray::new(53_u16));
    assert_eq!(re_alphaed.1, 255_u16);
    assert_eq!(described.0, String::from("bright:53"));
    assert_eq!(described.1, 255_u16);
}

#[test]
#[allow(deprecated)]
fn legacy_gray_alpha_edge_values_round_trip_through_mapping_pipeline() {
    let transparent_black: GrayAlpha<u8, u8> = GrayAlpha::new(0_u8, 0_u8);
    let opaque_white: GrayAlpha<u8, u8> = GrayAlpha::new(u8::MAX, u8::MAX);

    let transparent_promoted: GrayAlpha<u16, u16> =
        transparent_black.map_gray(|gray| u16::from(gray) * 257);
    let transparent_labeled: GrayAlpha<u16, &'static str> =
        transparent_promoted.map_alpha(|alpha| if alpha == 0 { "transparent" } else { "visible" });

    let opaque_promoted: GrayAlpha<u16, u16> = opaque_white.map_gray(|gray| u16::from(gray) * 257);
    let opaque_half: GrayAlpha<u16, u16> = opaque_promoted.alpha(128_u16);

    assert_eq!(transparent_promoted.gray(), Gray::new(0_u16));
    assert_eq!(transparent_promoted.1, 0_u16);
    assert_eq!(transparent_labeled.gray(), Gray::new(0_u16));
    assert_eq!(transparent_labeled.1, "transparent");
    assert_eq!(opaque_promoted.gray(), Gray::new(65_535_u16));
    assert_eq!(opaque_promoted.1, 255_u16);
    assert_eq!(opaque_half.gray(), Gray::new(65_535_u16));
    assert_eq!(opaque_half.1, 128_u16);
}