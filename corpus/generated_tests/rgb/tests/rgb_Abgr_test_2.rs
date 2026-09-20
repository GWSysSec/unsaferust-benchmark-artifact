use rgb::Abgr;

#[test]
fn abgr_map_rgb_then_map_alpha_preserves_abgr_storage_and_changes_types() {
    let source: Abgr<u8, u16> = Abgr {
        a: 512_u16,
        b: 3_u8,
        g: 5_u8,
        r: 7_u8,
    };

    let mut visited = Vec::new();
    let mapped_rgb: Abgr<u32, u32> = source.map_rgb(|channel| {
        visited.push(channel);
        u32::from(channel) * 100 + 9
    });

    assert_eq!(visited, vec![7_u8, 5_u8, 3_u8]);
    assert_eq!(mapped_rgb.r, 709_u32);
    assert_eq!(mapped_rgb.g, 509_u32);
    assert_eq!(mapped_rgb.b, 309_u32);
    assert_eq!(mapped_rgb.a, 512_u32);

    let label_prefix = String::from("opacity");
    let mapped_alpha: Abgr<u32, String> =
        mapped_rgb.map_alpha(move |alpha| format!("{label_prefix}:{alpha}"));

    assert_eq!(mapped_alpha.r, 709_u32);
    assert_eq!(mapped_alpha.g, 509_u32);
    assert_eq!(mapped_alpha.b, 309_u32);
    assert_eq!(mapped_alpha.a, String::from("opacity:512"));

    assert_eq!(
        source,
        Abgr {
            a: 512_u16,
            b: 3_u8,
            g: 5_u8,
            r: 7_u8,
        }
    );
}

#[test]
fn abgr_map_rgb_handles_boundary_channel_values_without_touching_alpha() {
    let transparent_blue_max: Abgr<u8> = Abgr::new_abgr(0, 255, 128, 0);

    let inverted: Abgr<i16, u16> =
        transparent_blue_max.map_rgb(|channel| 255_i16 - i16::from(channel));

    assert_eq!(inverted.a, 0_u16);
    assert_eq!(inverted.r, 255_i16);
    assert_eq!(inverted.g, 127_i16);
    assert_eq!(inverted.b, 0_i16);

    let original_bgr = transparent_blue_max.bgr();
    assert_eq!(original_bgr.r, 0_u8);
    assert_eq!(original_bgr.g, 128_u8);
    assert_eq!(original_bgr.b, 255_u8);

    assert_eq!(transparent_blue_max.a, 0_u8);
    assert_eq!(transparent_blue_max.r, 0_u8);
    assert_eq!(transparent_blue_max.g, 128_u8);
    assert_eq!(transparent_blue_max.b, 255_u8);
}

#[test]
fn abgr_map_alpha_can_use_fn_once_state_and_leaves_color_channels_intact() {
    let source: Abgr<u8, u8> = Abgr {
        r: 10_u8,
        g: 20_u8,
        b: 30_u8,
        a: 200_u8,
    };
    let threshold = 128_u8;
    let description = String::from("visible");

    let classified: Abgr<u8, (bool, String)> = source.map_alpha(move |alpha| {
        assert_eq!(alpha, 200_u8);
        (alpha >= threshold, description)
    });

    assert_eq!(classified.r, 10_u8);
    assert_eq!(classified.g, 20_u8);
    assert_eq!(classified.b, 30_u8);
    assert_eq!(classified.a, (true, String::from("visible")));

    let replaced_alpha: Abgr<u8, u16> = source.map_alpha(|_| 1024_u16);
    assert_eq!(replaced_alpha.r, 10_u8);
    assert_eq!(replaced_alpha.g, 20_u8);
    assert_eq!(replaced_alpha.b, 30_u8);
    assert_eq!(replaced_alpha.a, 1024_u16);

    assert_eq!(
        source,
        Abgr {
            r: 10_u8,
            g: 20_u8,
            b: 30_u8,
            a: 200_u8,
        }
    );
}