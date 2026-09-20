use rgb::Rgba;

#[test]
fn rgba_map_rgb_then_map_alpha_preserves_channel_order_and_changes_types() {
    let source: Rgba<u8, u16> = Rgba {
        r: 10,
        g: 20,
        b: 30,
        a: 1024,
    };

    let mut seen = Vec::new();
    let mapped_rgb: Rgba<u32, u16> = source.map_rgb(|channel| {
        seen.push(channel);
        u32::from(channel) * 3 + 1
    });

    assert_eq!(seen, vec![10_u8, 20_u8, 30_u8]);
    assert_eq!(mapped_rgb.r, 31_u32);
    assert_eq!(mapped_rgb.g, 61_u32);
    assert_eq!(mapped_rgb.b, 91_u32);
    assert_eq!(mapped_rgb.a, 1024_u16);

    let mapped_alpha: Rgba<u32, String> = mapped_rgb.map_alpha(|alpha| format!("alpha={alpha}"));

    assert_eq!(mapped_alpha.r, 31_u32);
    assert_eq!(mapped_alpha.g, 61_u32);
    assert_eq!(mapped_alpha.b, 91_u32);
    assert_eq!(mapped_alpha.a, String::from("alpha=1024"));

    assert_eq!(
        source,
        Rgba {
            r: 10,
            g: 20,
            b: 30,
            a: 1024
        }
    );
}

#[test]
fn rgba_alpha_and_mapping_handle_edge_channel_values() {
    let transparent_black: Rgba<u8, u8> = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    assert_eq!(transparent_black.a, 0_u8);

    let inverted: Rgba<u8, u8> = transparent_black.map_rgb(|channel| u8::MAX - channel);

    assert_eq!(inverted.r, 255);
    assert_eq!(inverted.g, 255);
    assert_eq!(inverted.b, 255);
    assert_eq!(inverted.a, 0);

    let opaque: Rgba<u8, u16> = inverted.map_alpha(|alpha| u16::from(alpha) + 65_535);

    assert_eq!(opaque.r, 255);
    assert_eq!(opaque.g, 255);
    assert_eq!(opaque.b, 255);
    assert_eq!(opaque.a, 65_535_u16);
}

#[test]
fn rgba_workflow_can_quantize_colors_and_normalize_alpha() {
    let source: Rgba<u16, u16> = Rgba {
        r: 0,
        g: 32_768,
        b: 65_535,
        a: 32_767,
    };

    let eight_bit: Rgba<u8, u16> = source.map_rgb(|channel| (channel / 257) as u8);

    assert_eq!(eight_bit.r, 0);
    assert_eq!(eight_bit.g, 127);
    assert_eq!(eight_bit.b, 255);
    assert_eq!(eight_bit.a, 32_767);

    let normalized: Rgba<u8, f32> = eight_bit.map_alpha(|alpha| f32::from(alpha) / 65_535.0);

    assert_eq!(normalized.r, 0);
    assert_eq!(normalized.g, 127);
    assert_eq!(normalized.b, 255);
    assert!((normalized.a - (32_767.0_f32 / 65_535.0_f32)).abs() < f32::EPSILON);
}