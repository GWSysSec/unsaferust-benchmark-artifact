use rgb::Bgra;

#[test]
fn map_rgb_transforms_rgb_channels_in_order_and_preserves_convertible_alpha() {
    let source: Bgra<u8, u8> = Bgra {
        b: 3,
        g: 7,
        r: 11,
        a: 128,
    };

    let mut seen = Vec::new();
    let mapped: Bgra<u16, u16> = source.map_rgb(|channel| {
        seen.push(channel);
        u16::from(channel) * 10 + 1
    });

    assert_eq!(seen, vec![11_u8, 7_u8, 3_u8]);
    assert_eq!(mapped.b, 31_u16);
    assert_eq!(mapped.g, 71_u16);
    assert_eq!(mapped.r, 111_u16);
    assert_eq!(mapped.a, 128_u16);

    assert_eq!(
        source,
        Bgra {
            b: 3,
            g: 7,
            r: 11,
            a: 128
        }
    );
}

#[test]
fn map_alpha_transforms_only_alpha_and_allows_new_alpha_type() {
    let source: Bgra<u8, u16> = Bgra {
        b: 10,
        g: 20,
        r: 30,
        a: 1024,
    };

    let mapped: Bgra<u8, String> = source.map_alpha(|alpha| format!("opacity={alpha}"));

    assert_eq!(mapped.b, 10_u8);
    assert_eq!(mapped.g, 20_u8);
    assert_eq!(mapped.r, 30_u8);
    assert_eq!(mapped.a, "opacity=1024");

    assert_eq!(source.b, 10_u8);
    assert_eq!(source.g, 20_u8);
    assert_eq!(source.r, 30_u8);
    assert_eq!(source.a, 1024_u16);
}

#[test]
fn chained_mapping_can_normalize_colors_then_label_alpha() {
    let source: Bgra<u8, u8> = Bgra {
        b: 0,
        g: 128,
        r: 255,
        a: 0,
    };

    let normalized: Bgra<f32, u16> = source.map_rgb(|channel| f32::from(channel) / 255.0);
    let labeled: Bgra<f32, Option<&'static str>> =
        normalized.map_alpha(|alpha| if alpha == 0 { None } else { Some("opaque") });

    assert_eq!(labeled.b, 0.0);
    assert!((labeled.g - (128.0 / 255.0)).abs() < f32::EPSILON);
    assert_eq!(labeled.r, 1.0);
    assert_eq!(labeled.a, None);

    let opaque_source: Bgra<u8, u8> = Bgra {
        b: 4,
        g: 5,
        r: 6,
        a: 255,
    };
    let opaque: Bgra<u8, Option<&'static str>> =
        opaque_source.map_alpha(|alpha| if alpha == 0 { None } else { Some("opaque") });

    assert_eq!(opaque.a, Some("opaque"));
    assert_eq!((opaque.b, opaque.g, opaque.r), (4_u8, 5_u8, 6_u8));
}

#[test]
fn map_rgb_handles_edge_channel_values_without_touching_alpha() {
    let source: Bgra<u8, u8> = Bgra {
        b: u8::MIN,
        g: 1,
        r: u8::MAX,
        a: 42,
    };

    let inverted: Bgra<u8, u16> = source.map_rgb(|channel| u8::MAX - channel);

    assert_eq!(inverted.b, u8::MAX);
    assert_eq!(inverted.g, 254);
    assert_eq!(inverted.r, u8::MIN);
    assert_eq!(inverted.a, 42_u16);
}

#[test]
fn map_alpha_can_compute_from_alpha_while_preserving_previously_mapped_rgb() {
    let source: Bgra<u8, u8> = Bgra {
        b: 25,
        g: 50,
        r: 75,
        a: 200,
    };

    let widened: Bgra<u32, u16> = source.map_rgb(|channel| u32::from(channel) * u32::from(channel));
    let final_pixel: Bgra<u32, bool> = widened.map_alpha(|alpha| alpha >= 128);

    assert_eq!(final_pixel.b, 625_u32);
    assert_eq!(final_pixel.g, 2500_u32);
    assert_eq!(final_pixel.r, 5625_u32);
    assert!(final_pixel.a);
}