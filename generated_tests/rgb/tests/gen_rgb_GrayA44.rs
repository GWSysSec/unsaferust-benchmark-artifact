use rgb::*;

#[test]
fn graya44_basic_accessors() {
    let p = GrayA44::new(0x0A, 0x05).unwrap();
    assert_eq!(p.v(), 0x0A);
    assert_eq!(p.a(), 0x05);
    assert_ne!(p.v(), p.a());

    let zero = GrayA44::new(0, 0).unwrap();
    assert_eq!(zero.v(), 0);
    assert_eq!(zero.a(), 0);

    let max = GrayA44::new(0x0F, 0x0F).unwrap();
    assert_eq!(max.v(), 0x0F);
    assert_eq!(max.a(), 0x0F);
}

#[test]
fn graya44_boundary_nibbles() {

    let p_v_only = GrayA44::new(0x0F, 0x00).unwrap();
    assert_eq!(p_v_only.v(), 0x0F);
    assert_eq!(p_v_only.a(), 0x00);

    let p_a_only = GrayA44::new(0x00, 0x0F).unwrap();
    assert_eq!(p_a_only.v(), 0x00);
    assert_eq!(p_a_only.a(), 0x0F);


    let mid = GrayA44::new(0x07, 0x08).unwrap();
    assert_eq!(mid.v(), 0x07);
    assert_eq!(mid.a(), 0x08);
    assert_ne!(mid.v(), mid.a());


    let asym = GrayA44::new(0x01, 0x0E).unwrap();
    assert_eq!(asym.v(), 0x01);
    assert_eq!(asym.a(), 0x0E);
}

#[test]
fn graya44_bulk_roundtrip() {
    let mut pixels: Vec<GrayA44> = Vec::with_capacity(256);
    for v in 0..16u8 {
        for a in 0..16u8 {
            pixels.push(GrayA44::new(v, a).unwrap());
        }
    }
    assert_eq!(pixels.len(), 256);
    assert_eq!(pixels.capacity() >= 256, true);


    assert_eq!(pixels[0].v(), 0);
    assert_eq!(pixels[0].a(), 0);
    assert_eq!(pixels[255].v(), 15);
    assert_eq!(pixels[255].a(), 15);


    let mut mismatches = 0usize;
    for (idx, p) in pixels.iter().enumerate() {
        let expected_v = (idx / 16) as u8;
        let expected_a = (idx % 16) as u8;
        if p.v() != expected_v || p.a() != expected_a {
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0);


    let sum_v: u32 = pixels.iter().map(|p| p.v() as u32).sum();
    let sum_a: u32 = pixels.iter().map(|p| p.a() as u32).sum();

    assert_eq!(sum_v, 1920);
    assert_eq!(sum_a, 1920);
}

#[test]
fn graya44_copy_semantics() {
    let p = GrayA44::new(0x0C, 0x03).unwrap();

    let v1 = p.v();
    let v2 = p.v();
    let a1 = p.a();
    let a2 = p.a();
    assert_eq!(v1, v2);
    assert_eq!(a1, a2);
    assert_eq!(v1, 0x0C);
    assert_eq!(a1, 0x03);


    let q = p;
    assert_eq!(q.v(), 0x0C);
    assert_eq!(q.a(), 0x03);
    assert_eq!(p.v(), q.v());
    assert_eq!(p.a(), q.a());
}