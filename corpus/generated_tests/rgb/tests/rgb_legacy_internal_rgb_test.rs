use rgb::{Rgb, Rgba};

trait OpaqueAlpha {
    fn opaque() -> Self;
}

impl OpaqueAlpha for u8 {
    fn opaque() -> Self {
        u8::MAX
    }
}

impl OpaqueAlpha for u16 {
    fn opaque() -> Self {
        u16::MAX
    }
}

fn alpha<T: OpaqueAlpha>() -> T {
    T::opaque()
}

#[test]
fn legacy_alpha_supplies_opaque_integer_channel_values() {
    let alpha_u8: u8 = alpha();
    let alpha_u16: u16 = alpha();

    assert_eq!(alpha_u8, u8::MAX);
    assert_eq!(alpha_u16, u16::MAX);

    let base = Rgb {
        r: 12_u8,
        g: 34_u8,
        b: 56_u8,
    };

    let with_legacy_alpha = Rgba {
        r: base.r,
        g: base.g,
        b: base.b,
        a: alpha::<u8>(),
    };

    assert_eq!(with_legacy_alpha.r, 12);
    assert_eq!(with_legacy_alpha.g, 34);
    assert_eq!(with_legacy_alpha.b, 56);
    assert_eq!(with_legacy_alpha.a, 255);
    assert_eq!(
        with_legacy_alpha,
        Rgba {
            r: 12,
            g: 34,
            b: 56,
            a: u8::MAX,
        }
    );
}

#[test]
fn legacy_alpha_can_be_used_to_restore_opacity_after_pixel_processing() {
    let original = Rgba {
        r: 200_u8,
        g: 120_u8,
        b: 40_u8,
        a: 17_u8,
    };

    let color_only = Rgb {
        r: original.r.saturating_sub(50),
        g: original.g.saturating_add(10),
        b: original.b / 2,
    };

    let restored = Rgba {
        r: color_only.r,
        g: color_only.g,
        b: color_only.b,
        a: alpha::<u8>(),
    };

    assert_eq!(color_only, Rgb { r: 150, g: 130, b: 20 });
    assert_ne!(restored.a, original.a);
    assert_eq!(restored.a, u8::MAX);
    assert_eq!(restored.r, 150);
    assert_eq!(restored.g, 130);
    assert_eq!(restored.b, 20);
}