use rgb::Argb;

#[test]
fn map_rgb_transforms_color_channels_preserves_alpha_and_source() {
    let source: Argb<u8> = Argb {
        a: 200,
        r: 10,
        g: 20,
        b: 30,
    };

    let mut calls = 0_u8;
    let mapped: Argb<u16, u16> = source.map_rgb(|channel| {
        calls += 1;
        u16::from(channel) * 4
    });

    assert_eq!(calls, 3);
    assert_eq!(mapped.a, 200_u16);
    assert_eq!(mapped.r, 40_u16);
    assert_eq!(mapped.g, 80_u16);
    assert_eq!(mapped.b, 120_u16);

    assert_eq!(source.a, 200_u8);
    assert_eq!(source.r, 10_u8);
    assert_eq!(source.g, 20_u8);
    assert_eq!(source.b, 30_u8);
}

#[test]
fn map_alpha_transforms_only_alpha_and_changes_alpha_type() {
    let source: Argb<u8, u16> = Argb {
        a: 1024,
        r: 7,
        g: 8,
        b: 9,
    };

    let mapped: Argb<u8, String> = source.map_alpha(|alpha| format!("alpha={alpha}"));

    assert_eq!(mapped.a, "alpha=1024");
    assert_eq!(mapped.r, 7_u8);
    assert_eq!(mapped.g, 8_u8);
    assert_eq!(mapped.b, 9_u8);

    assert_eq!(source.a, 1024_u16);
    assert_eq!(source.r, 7_u8);
    assert_eq!(source.g, 8_u8);
    assert_eq!(source.b, 9_u8);
}

#[test]
fn chained_mapping_can_normalize_rgb_then_classify_alpha() {
    let source: Argb<u8, u8> = Argb {
        a: 0,
        r: 0,
        g: 128,
        b: 255,
    };

    let normalized: Argb<f32, u16> = source.map_rgb(|channel| f32::from(channel) / 255.0);
    let classified: Argb<f32, &'static str> =
        normalized.map_alpha(|alpha| if alpha == 0 { "transparent" } else { "visible" });

    assert_eq!(classified.a, "transparent");
    assert_eq!(classified.r, 0.0);
    assert!((classified.g - (128.0 / 255.0)).abs() < f32::EPSILON);
    assert_eq!(classified.b, 1.0);
}

#[test]
fn map_rgb_handles_edge_channel_values_without_clamping() {
    let source: Argb<u8, u8> = Argb {
        a: 255,
        r: 0,
        g: 1,
        b: 255,
    };

    let inverted: Argb<i16, u16> = source.map_rgb(|channel| 255_i16 - i16::from(channel));

    assert_eq!(inverted.a, 255_u16);
    assert_eq!(inverted.r, 255_i16);
    assert_eq!(inverted.g, 254_i16);
    assert_eq!(inverted.b, 0_i16);
}

#[test]
fn map_alpha_supports_fn_once_closure_consuming_captured_value() {
    let source: Argb<u8, u8> = Argb {
        a: 42,
        r: 11,
        g: 22,
        b: 33,
    };
    let prefix = String::from("opacity");

    let mapped: Argb<u8, String> = source.map_alpha(|alpha| {
        let consumed_prefix = prefix;
        format!("{consumed_prefix}:{alpha}")
    });

    assert_eq!(mapped.a, "opacity:42");
    assert_eq!((mapped.r, mapped.g, mapped.b), (11_u8, 22_u8, 33_u8));
}