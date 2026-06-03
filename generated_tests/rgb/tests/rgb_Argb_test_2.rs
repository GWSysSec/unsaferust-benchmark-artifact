use rgb::Argb;

#[test]
fn argb_map_rgb_visits_color_channels_in_rgb_order_and_preserves_alpha() {
    let source: Argb<u8, u16> = Argb::new_alpha(10, 20, 30, 512_u16);

    let mut visited = Vec::new();
    let mapped: Argb<u32, u32> = source.map_rgb(|channel| {
        visited.push(channel);
        u32::from(channel) * 4 + 1
    });

    assert_eq!(visited, vec![10_u8, 20_u8, 30_u8]);
    assert_eq!(mapped.r, 41_u32);
    assert_eq!(mapped.g, 81_u32);
    assert_eq!(mapped.b, 121_u32);
    assert_eq!(mapped.a, 512_u32);

    assert_eq!(source.r, 10_u8);
    assert_eq!(source.g, 20_u8);
    assert_eq!(source.b, 30_u8);
    assert_eq!(source.a, 512_u16);
}

#[test]
fn argb_map_alpha_changes_only_alpha_and_can_change_alpha_type() {
    let source: Argb<u8, u16> = Argb {
        a: 1024_u16,
        r: 7_u8,
        g: 14_u8,
        b: 21_u8,
    };

    let mapped: Argb<u8, String> = source.map_alpha(|alpha| format!("opacity:{alpha}"));

    assert_eq!(mapped.r, 7_u8);
    assert_eq!(mapped.g, 14_u8);
    assert_eq!(mapped.b, 21_u8);
    assert_eq!(mapped.a, String::from("opacity:1024"));

    assert_eq!(
        source,
        Argb {
            a: 1024_u16,
            r: 7_u8,
            g: 14_u8,
            b: 21_u8,
        }
    );
}

#[test]
fn argb_map_rgb_then_map_alpha_supports_multi_step_pixel_workflow() {
    let original: Argb<u8, u8> = Argb::new_argb(128, 0, 127, 255);

    let linearized: Argb<u16, u16> = original.map_rgb(|channel| u16::from(channel) * 257);

    assert_eq!(linearized.a, 128_u16);
    assert_eq!(linearized.r, 0_u16);
    assert_eq!(linearized.g, 32639_u16);
    assert_eq!(linearized.b, 65535_u16);

    let described: Argb<u16, String> = linearized.map_alpha(|alpha| {
        if alpha == 128 {
            String::from("half-transparent")
        } else {
            format!("alpha={alpha}")
        }
    });

    assert_eq!(described.r, 0_u16);
    assert_eq!(described.g, 32639_u16);
    assert_eq!(described.b, 65535_u16);
    assert_eq!(described.a, String::from("half-transparent"));

    assert_eq!(original, Argb::new_argb(128, 0, 127, 255));
}

#[test]
fn argb_map_rgb_handles_edge_channel_values_without_touching_alpha() {
    let source: Argb<u8, u8> = Argb::new(255, 0, 1, 0);

    let inverted: Argb<u8, u8> = source.map_rgb(|channel| 255_u8 - channel);

    assert_eq!(inverted.a, 0_u8);
    assert_eq!(inverted.r, 0_u8);
    assert_eq!(inverted.g, 255_u8);
    assert_eq!(inverted.b, 254_u8);

    let alpha_replaced: Argb<u8, bool> = inverted.map_alpha(|alpha| alpha == 0);

    assert_eq!(alpha_replaced.r, 0_u8);
    assert_eq!(alpha_replaced.g, 255_u8);
    assert_eq!(alpha_replaced.b, 254_u8);
    assert!(alpha_replaced.a);
}