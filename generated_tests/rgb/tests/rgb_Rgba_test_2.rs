use rgb::Rgba;

#[test]
fn rgba_map_rgb_then_map_alpha_transforms_channels_in_rgb_order() {
    let source: Rgba<u8, u8> = Rgba::new(10, 20, 30, 128);

    let mut visited = Vec::new();
    let color_adjusted: Rgba<u16, u16> = source.map_rgb(|channel| {
        visited.push(channel);
        u16::from(channel) * 2 + 5
    });

    assert_eq!(visited, vec![10_u8, 20_u8, 30_u8]);
    assert_eq!(color_adjusted.r, 25_u16);
    assert_eq!(color_adjusted.g, 45_u16);
    assert_eq!(color_adjusted.b, 65_u16);
    assert_eq!(color_adjusted.a, 128_u16);

    let alpha_labeled: Rgba<u16, String> =
        color_adjusted.map_alpha(|alpha| format!("{alpha}/255"));

    assert_eq!(alpha_labeled.r, 25_u16);
    assert_eq!(alpha_labeled.g, 45_u16);
    assert_eq!(alpha_labeled.b, 65_u16);
    assert_eq!(alpha_labeled.a, String::from("128/255"));

    assert_eq!(source, Rgba::new(10, 20, 30, 128));
}

#[test]
fn rgba_map_alpha_changes_only_alpha_and_can_feed_later_rgb_mapping() {
    let source: Rgba<u8, u16> = Rgba::new_alpha(3, 5, 8, 1024_u16);

    let normalized_alpha: Rgba<u8, f32> = source.map_alpha(|alpha| f32::from(alpha) / 2048.0);

    assert_eq!(normalized_alpha.r, 3);
    assert_eq!(normalized_alpha.g, 5);
    assert_eq!(normalized_alpha.b, 8);
    assert_eq!(normalized_alpha.a, 0.5);

    let remapped_rgb: Rgba<i16, f32> =
        normalized_alpha.map_rgb(|channel| i16::from(channel) - 4);

    assert_eq!(remapped_rgb.r, -1);
    assert_eq!(remapped_rgb.g, 1);
    assert_eq!(remapped_rgb.b, 4);
    assert_eq!(remapped_rgb.a, 0.5);

    assert_eq!(source.r, 3);
    assert_eq!(source.g, 5);
    assert_eq!(source.b, 8);
    assert_eq!(source.a, 1024_u16);
}

#[test]
fn rgba_mapping_handles_boundary_channel_values_without_mutating_source() {
    let transparent_black: Rgba<u8, u8> = Rgba::new(0, 0, 0, 0);
    let inverted: Rgba<u8, u8> = transparent_black.map_rgb(|channel| 255 - channel);

    assert_eq!(inverted, Rgba::new(255, 255, 255, 0));
    assert_eq!(transparent_black, Rgba::new(0, 0, 0, 0));

    let opaque_mixed: Rgba<u8, u8> = Rgba::new(0, 127, 255, 255);
    let widened: Rgba<u16, u16> = opaque_mixed.map_rgb(u16::from);

    assert_eq!(widened.r, 0_u16);
    assert_eq!(widened.g, 127_u16);
    assert_eq!(widened.b, 255_u16);
    assert_eq!(widened.a, 255_u16);

    let alpha_as_flag: Rgba<u16, bool> = widened.map_alpha(|alpha| alpha == 255);
    assert_eq!(alpha_as_flag.r, 0_u16);
    assert_eq!(alpha_as_flag.g, 127_u16);
    assert_eq!(alpha_as_flag.b, 255_u16);
    assert!(alpha_as_flag.a);
}