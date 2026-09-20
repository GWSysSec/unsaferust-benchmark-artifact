use rgb::GrayA44;

#[test]
fn gray_a44_preserves_all_valid_grayscale_and_alpha_nibbles() {
    let samples = [(0_u8, 0_u8), (0, 15), (7, 8), (15, 0), (15, 15)];

    let pixels: Vec<GrayA44> = samples
        .iter()
        .map(|&(gray, alpha)| {
            let constructed = GrayA44::new(gray, alpha);
            assert!(
                constructed.is_ok(),
                "valid 4-bit components ({gray}, {alpha}) should construct a pixel"
            );
            constructed.unwrap()
        })
        .collect();

    for (pixel, &(expected_gray, expected_alpha)) in pixels.iter().copied().zip(samples.iter()) {
        assert_eq!(pixel.v(), expected_gray);
        assert_eq!(pixel.a(), expected_alpha);
    }
}

#[test]
fn gray_a44_rejects_components_outside_four_bit_range() {
    assert!(GrayA44::new(16, 0).is_err());
    assert!(GrayA44::new(0, 16).is_err());
    assert!(GrayA44::new(16, 16).is_err());
    assert!(GrayA44::new(u8::MAX, 15).is_err());
    assert!(GrayA44::new(15, u8::MAX).is_err());
}

#[test]
fn gray_a44_components_can_drive_a_simple_alpha_blending_workflow() {
    let foreground = GrayA44::new(12, 10).unwrap();
    let background = GrayA44::new(4, 15).unwrap();

    let fg_alpha = u16::from(foreground.a());
    let inverse_alpha = 15_u16 - fg_alpha;
    let blended = (u16::from(foreground.v()) * fg_alpha
        + u16::from(background.v()) * inverse_alpha
        + 7)
        / 15;

    assert_eq!(foreground.v(), 12);
    assert_eq!(foreground.a(), 10);
    assert_eq!(background.v(), 4);
    assert_eq!(background.a(), 15);
    assert_eq!(blended, 9);
}