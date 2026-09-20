use rgb::Rgba;

#[test]
fn map_rgb_transforms_channels_in_rgb_order_and_converts_alpha() {
    let source: Rgba<u8, u8> = Rgba {
        r: 12,
        g: 34,
        b: 56,
        a: 128,
    };

    let mut seen = Vec::new();
    let mapped: Rgba<u16, u16> = source.map_rgb(|channel| {
        seen.push(channel);
        u16::from(channel) * 2 + 1
    });

    assert_eq!(seen, vec![12_u8, 34_u8, 56_u8]);
    assert_eq!(mapped.r, 25_u16);
    assert_eq!(mapped.g, 69_u16);
    assert_eq!(mapped.b, 113_u16);
    assert_eq!(mapped.a, 128_u16);

    assert_eq!(
        source,
        Rgba {
            r: 12,
            g: 34,
            b: 56,
            a: 128
        }
    );
}

#[test]
fn map_alpha_changes_only_alpha_type_and_preserves_rgb_channels() {
    let source: Rgba<u8, u16> = Rgba {
        r: 5,
        g: 10,
        b: 15,
        a: 1024,
    };

    let mapped: Rgba<u8, String> = source.map_alpha(|alpha| format!("opacity:{alpha}"));

    assert_eq!(mapped.r, 5_u8);
    assert_eq!(mapped.g, 10_u8);
    assert_eq!(mapped.b, 15_u8);
    assert_eq!(mapped.a, String::from("opacity:1024"));

    assert_eq!(source.a, 1024_u16);
    assert_eq!(source.r, 5_u8);
    assert_eq!(source.g, 10_u8);
    assert_eq!(source.b, 15_u8);
}

#[test]
fn chained_rgb_and_alpha_mapping_supports_realistic_normalization_workflow() {
    let source: Rgba<u8, u8> = Rgba {
        r: 0,
        g: 128,
        b: 255,
        a: 64,
    };

    let normalized: Rgba<f32, u16> = source.map_rgb(|channel| f32::from(channel) / 255.0);
    let with_fractional_alpha: Rgba<f32, f32> =
        normalized.map_alpha(|alpha| f32::from(alpha) / 255.0);

    assert_eq!(with_fractional_alpha.r, 0.0);
    assert!((with_fractional_alpha.g - (128.0 / 255.0)).abs() < f32::EPSILON);
    assert_eq!(with_fractional_alpha.b, 1.0);
    assert!((with_fractional_alpha.a - (64.0 / 255.0)).abs() < f32::EPSILON);

    assert_eq!(normalized.r, 0.0);
    assert!((normalized.g - (128.0 / 255.0)).abs() < f32::EPSILON);
    assert_eq!(normalized.b, 1.0);
    assert_eq!(normalized.a, 64_u16);
}

#[test]
fn edge_values_can_be_mapped_without_mutating_original_pixel() {
    let source: Rgba<u8, u8> = Rgba {
        r: u8::MIN,
        g: 1,
        b: u8::MAX,
        a: u8::MAX,
    };

    let inverted: Rgba<u8, u16> = source.map_rgb(|channel| u8::MAX - channel);
    let labeled: Rgba<u8, Option<u8>> = inverted.map_alpha(|alpha| {
        if alpha == u16::from(u8::MAX) {
            Some(u8::MAX)
        } else {
            None
        }
    });

    assert_eq!(inverted.r, u8::MAX);
    assert_eq!(inverted.g, 254);
    assert_eq!(inverted.b, u8::MIN);
    assert_eq!(inverted.a, 255_u16);

    assert_eq!(labeled.r, u8::MAX);
    assert_eq!(labeled.g, 254);
    assert_eq!(labeled.b, u8::MIN);
    assert_eq!(labeled.a, Some(u8::MAX));

    assert_eq!(
        source,
        Rgba {
            r: u8::MIN,
            g: 1,
            b: u8::MAX,
            a: u8::MAX
        }
    );
}