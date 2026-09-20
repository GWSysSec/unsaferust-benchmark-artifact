use rgb::Rgba;

#[test]
fn rgba_legacy_map_rgb_alpha_and_map_alpha_chain_preserves_expected_channels() {
    let source: Rgba<u8, u16> = Rgba {
        r: 4,
        g: 8,
        b: 12,
        a: 300,
    };

    let mut visited = Vec::new();
    let expanded: Rgba<u32, u16> = source.map_rgb(|channel| {
        visited.push(channel);
        u32::from(channel) * u32::from(channel)
    });

    assert_eq!(visited, vec![4_u8, 8_u8, 12_u8]);
    assert_eq!(expanded.r, 16_u32);
    assert_eq!(expanded.g, 64_u32);
    assert_eq!(expanded.b, 144_u32);
    assert_eq!(expanded.a, 300_u16);

    let replaced_alpha: Rgba<u32, u16> = expanded.alpha(999);
    assert_eq!(replaced_alpha.r, 16_u32);
    assert_eq!(replaced_alpha.g, 64_u32);
    assert_eq!(replaced_alpha.b, 144_u32);
    assert_eq!(replaced_alpha.a, 999_u16);

    let described: Rgba<u32, String> = replaced_alpha.map_alpha(|alpha| format!("opacity:{alpha}"));
    assert_eq!(described.r, 16_u32);
    assert_eq!(described.g, 64_u32);
    assert_eq!(described.b, 144_u32);
    assert_eq!(described.a, String::from("opacity:999"));

    assert_eq!(
        source,
        Rgba {
            r: 4,
            g: 8,
            b: 12,
            a: 300
        }
    );
}

#[test]
fn rgba_legacy_alpha_replacement_does_not_mutate_original_or_reorder_rgb() {
    let source: Rgba<i16, &'static str> = Rgba {
        r: -5,
        g: 0,
        b: 5,
        a: "initial",
    };

    let replaced: Rgba<i16, &'static str> = source.alpha("updated");

    assert_eq!(source.r, -5);
    assert_eq!(source.g, 0);
    assert_eq!(source.b, 5);
    assert_eq!(source.a, "initial");

    assert_eq!(replaced.r, -5);
    assert_eq!(replaced.g, 0);
    assert_eq!(replaced.b, 5);
    assert_eq!(replaced.a, "updated");

    let doubled: Rgba<i32, &'static str> = replaced.map_rgb(|channel| i32::from(channel) * 2);
    assert_eq!(
        doubled,
        Rgba {
            r: -10,
            g: 0,
            b: 10,
            a: "updated"
        }
    );
}

#[test]
fn rgba_legacy_map_alpha_can_change_alpha_type_after_rgb_mapping_edge_values() {
    let source: Rgba<u8, u8> = Rgba {
        r: 0,
        g: 128,
        b: 255,
        a: 0,
    };

    let inverted: Rgba<u8, u8> = source.map_rgb(|channel| 255_u8 - channel);
    assert_eq!(
        inverted,
        Rgba {
            r: 255,
            g: 127,
            b: 0,
            a: 0
        }
    );

    let alpha_as_option: Rgba<u8, Option<u8>> = inverted.map_alpha(|alpha| {
        if alpha == 0 {
            None
        } else {
            Some(alpha)
        }
    });

    assert_eq!(alpha_as_option.r, 255);
    assert_eq!(alpha_as_option.g, 127);
    assert_eq!(alpha_as_option.b, 0);
    assert_eq!(alpha_as_option.a, None);

    let made_visible: Rgba<u8, Option<u8>> = alpha_as_option.alpha(Some(255));
    assert_eq!(made_visible.r, 255);
    assert_eq!(made_visible.g, 127);
    assert_eq!(made_visible.b, 0);
    assert_eq!(made_visible.a, Some(255));
}