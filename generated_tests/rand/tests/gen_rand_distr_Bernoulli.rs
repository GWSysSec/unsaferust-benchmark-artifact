use rand::distr::Bernoulli;
use rand::distr::Distribution;
use rand::rngs::mock::StepRng;

#[test]
fn bernoulli_p_returns_input_probability() {
    let probs = [0.0, 0.25, 0.5, 0.75, 1.0];
    for &p in &probs {
        let b = Bernoulli::new(p).expect("valid p");
        let returned = b.p();
        assert!(
            (returned - p).abs() < 1e-9,
            "expected p ~= {}, got {}",
            p,
            returned
        );
    }


    let zero = Bernoulli::new(0.0).unwrap();
    let one = Bernoulli::new(1.0).unwrap();
    assert_eq!(zero.p(), 0.0);
    assert_eq!(one.p(), 1.0);

    let mut rng = StepRng::new(0, 1);
    for _ in 0..32 {
        assert_eq!(zero.sample(&mut rng), false);
        assert_eq!(one.sample(&mut rng), true);
    }
}

#[test]
fn bernoulli_p_invalid_and_clone_consistency() {

    assert!(Bernoulli::new(-0.1).is_err());
    assert!(Bernoulli::new(1.1).is_err());
    assert!(Bernoulli::new(f64::NAN).is_err());

    let b = Bernoulli::new(0.3333333333333333).expect("ok");
    let p1 = b.p();
    let b2 = b.clone();
    let p2 = b2.p();
    assert_eq!(p1, p2);
    assert!(p1 > 0.33 && p1 < 0.34);


    let target = 0.7;
    let dist = Bernoulli::new(target).unwrap();
    assert!((dist.p() - target).abs() < 1e-12);

    let mut rng = rand::rng();
    let n = 20_000;
    let trues = (0..n).filter(|_| dist.sample(&mut rng)).count();
    let freq = trues as f64 / n as f64;
    assert!(
        (freq - target).abs() < 0.05,
        "freq {} not near {}",
        freq,
        target
    );
    assert!(trues > 0);
    assert!(trues < n);
}