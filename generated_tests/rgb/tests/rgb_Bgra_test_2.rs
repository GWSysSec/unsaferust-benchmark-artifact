use rgb::Bgra;

#[test]
fn bgra_map_rgb_then_map_alpha_converts_types_and_preserves_bgra_semantics() {
    let source: Bgra<u8, u16> = Bgra::new_alpha(40, 20, 10, 1_000);

    assert_eq!(source.r, 40);
    assert_eq!(source.g, 20);
    assert_eq!(source.b, 10);
    assert_eq!(source.a, 1_000);

    let mut visited_channels = Vec::new();
    let brighter: Bgra<u32, u32> = source.map_rgb(|channel| {
        visited_channels.push(channel);
        u32::from(channel) + 100
    });

    assert_eq!(visited_channels, vec![40_u8, 20_u8, 10_u8]);
    assert_eq!(brighter.r, 140);
    assert_eq!(brighter.g, 120);
    assert_eq!(brighter.b, 110);
    assert_eq!(brighter.a, 1_000_u32);

    let labeled: Bgra<u32, String> = brighter.map_alpha(|alpha| format!("opacity={alpha}"));

    assert_eq!(labeled.r, 140);
    assert_eq!(labeled.g, 120);
    assert_eq!(labeled.b, 110);
    assert_eq!(labeled.a, String::from("opacity=1000"));

    assert_eq!(source, Bgra::new_alpha(40, 20, 10, 1_000));
}

#[test]
fn bgra_mapping_handles_zero_channels_and_alpha_independently() {
    let transparent_black: Bgra<u8, u8> = Bgra::new(0, 0, 0, 0);

    let mapped_rgb: Bgra<i16, u16> = transparent_black.map_rgb(|channel| {
        assert_eq!(channel, 0);
        i16::from(channel) - 12
    });

    assert_eq!(mapped_rgb.r, -12);
    assert_eq!(mapped_rgb.g, -12);
    assert_eq!(mapped_rgb.b, -12);
    assert_eq!(mapped_rgb.a, 0_u16);

    let mapped_alpha: Bgra<i16, bool> = mapped_rgb.map_alpha(|alpha| alpha == 0);

    assert_eq!(mapped_alpha.r, -12);
    assert_eq!(mapped_alpha.g, -12);
    assert_eq!(mapped_alpha.b, -12);
    assert!(mapped_alpha.a);
}

#[test]
fn bgra_map_alpha_can_consume_complex_alpha_without_touching_color_channels() {
    let source: Bgra<u8, Vec<&'static str>> = Bgra {
        r: 9,
        g: 8,
        b: 7,
        a: vec!["matte", "layer"],
    };

    let alpha_len: Bgra<u8, usize> = source.map_alpha(|labels| {
        assert_eq!(labels, vec!["matte", "layer"]);
        labels.len()
    });

    assert_eq!(alpha_len.r, 9);
    assert_eq!(alpha_len.g, 8);
    assert_eq!(alpha_len.b, 7);
    assert_eq!(alpha_len.a, 2);

    let scaled: Bgra<u16, usize> = alpha_len.map_rgb(|channel| u16::from(channel) * 256);

    assert_eq!(scaled.r, 2_304);
    assert_eq!(scaled.g, 2_048);
    assert_eq!(scaled.b, 1_792);
    assert_eq!(scaled.a, 2);
}