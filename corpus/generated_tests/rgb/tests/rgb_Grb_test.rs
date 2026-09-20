use rgb::Grb;

#[test]
fn new_grb_assigns_components_in_grb_order() {
    let green = 10u8;
    let red = 20u8;
    let blue = 30u8;

    let pixel = Grb::new_grb(green, red, blue);

    assert_eq!(pixel.g, green);
    assert_eq!(pixel.r, red);
    assert_eq!(pixel.b, blue);

    let literal = Grb { g: green, r: red, b: blue };
    assert_eq!(pixel, literal);
}

#[test]
fn new_grb_is_usable_in_const_contexts_and_arrays() {
    const BLACK: Grb<u8> = Grb::new_grb(0, 0, 0);
    const WHITE: Grb<u8> = Grb::new_grb(255, 255, 255);
    const STATUS_COLORS: [Grb<u8>; 3] = [
        Grb::new_grb(255, 0, 0),
        Grb::new_grb(0, 255, 0),
        Grb::new_grb(0, 0, 255),
    ];

    assert_eq!(BLACK, Grb { g: 0, r: 0, b: 0 });
    assert_eq!(WHITE, Grb { g: 255, r: 255, b: 255 });
    assert_eq!(STATUS_COLORS[0].g, 255);
    assert_eq!(STATUS_COLORS[1].r, 255);
    assert_eq!(STATUS_COLORS[2].b, 255);
}

#[test]
fn new_grb_supports_realistic_palette_workflow() {
    let source_components = [
        (0u16, 1023u16, 128u16),
        (512u16, 256u16, 64u16),
        (1023u16, 0u16, 900u16),
        (1u16, 2u16, 3u16),
    ];

    let palette: Vec<Grb<u16>> = source_components
        .iter()
        .map(|&(g, r, b)| Grb::new_grb(g, r, b))
        .collect();

    assert_eq!(palette.len(), source_components.len());
    assert_eq!(palette.first(), Some(&Grb { g: 0, r: 1023, b: 128 }));
    assert_eq!(palette.last(), Some(&Grb { g: 1, r: 2, b: 3 }));

    let totals = palette.iter().fold((0u32, 0u32, 0u32), |(g_total, r_total, b_total), pixel| {
        (
            g_total + u32::from(pixel.g),
            r_total + u32::from(pixel.r),
            b_total + u32::from(pixel.b),
        )
    });

    assert_eq!(totals, (1536, 1281, 1095));

    let brightest_green = palette.iter().max_by_key(|pixel| pixel.g);
    assert_eq!(brightest_green, Some(&Grb { g: 1023, r: 0, b: 900 }));
}

#[test]
fn new_grb_preserves_edge_values_without_channel_swapping() {
    let min_green = Grb::new_grb(u8::MIN, 100, 200);
    let max_red = Grb::new_grb(50, u8::MAX, 150);
    let max_blue = Grb::new_grb(25, 75, u8::MAX);

    assert_eq!((min_green.g, min_green.r, min_green.b), (0, 100, 200));
    assert_eq!((max_red.g, max_red.r, max_red.b), (50, 255, 150));
    assert_eq!((max_blue.g, max_blue.r, max_blue.b), (25, 75, 255));

    let packed_for_device: Vec<u8> = [min_green, max_red, max_blue]
        .iter()
        .flat_map(|pixel| [pixel.g, pixel.r, pixel.b])
        .collect();

    assert_eq!(
        packed_for_device,
        vec![
            0, 100, 200,
            50, 255, 150,
            25, 75, 255,
        ]
    );
}