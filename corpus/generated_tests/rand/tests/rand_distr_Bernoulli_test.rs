use rand::distr::Bernoulli;

#[test]
fn bernoulli_p_reports_probability_used_by_certain_outcomes() {
    let always_false = Bernoulli::new(0.0).expect("p = 0 is a valid Bernoulli probability");
    let always_true = Bernoulli::new(1.0).expect("p = 1 is a valid Bernoulli probability");

    assert_eq!(always_false.p(), 0.0);
    assert_eq!(always_true.p(), 1.0);
    assert!(always_false.p() < always_true.p());

    let mut rng = rand::rng();

    for _ in 0..32 {
        let sampled_false: bool = rand::distr::Distribution::sample(&always_false, &mut rng);
        let sampled_true: bool = rand::distr::Distribution::sample(&always_true, &mut rng);

        assert!(!sampled_false, "p = 0 must never sample true");
        assert!(sampled_true, "p = 1 must always sample true");
    }

    assert_eq!(always_false.p(), 0.0, "sampling must not mutate the distribution probability");
    assert_eq!(always_true.p(), 1.0, "sampling must not mutate the distribution probability");
}

#[test]
fn bernoulli_p_preserves_fractional_probabilities_from_constructors() {
    let from_float = Bernoulli::new(0.25).expect("probability in [0, 1] should be valid");
    let from_ratio = Bernoulli::from_ratio(1, 4).expect("valid ratio should create Bernoulli");

    assert!((from_float.p() - 0.25).abs() <= f64::EPSILON);
    assert!((from_ratio.p() - 0.25).abs() <= f64::EPSILON);
    assert_eq!(from_float.p(), from_ratio.p());

    let half = Bernoulli::from_ratio(3, 6).expect("reducible ratio should be accepted");
    assert!((half.p() - 0.5).abs() <= f64::EPSILON);
    assert!(from_ratio.p() < half.p());

    let three_quarters = Bernoulli::new(0.75).expect("probability in [0, 1] should be valid");
    assert!((three_quarters.p() - 0.75).abs() <= f64::EPSILON);
    assert!(half.p() < three_quarters.p());
}

#[test]
fn bernoulli_p_values_can_drive_follow_up_workflows() {
    let distributions = [
        Bernoulli::new(0.0).expect("valid probability"),
        Bernoulli::from_ratio(1, 2).expect("valid ratio"),
        Bernoulli::new(1.0).expect("valid probability"),
    ];

    let probabilities: Vec<f64> = distributions.iter().map(Bernoulli::p).collect();

    assert_eq!(probabilities.len(), 3);
    assert_eq!(probabilities[0], 0.0);
    assert!((probabilities[1] - 0.5).abs() <= f64::EPSILON);
    assert_eq!(probabilities[2], 1.0);
    assert!(probabilities.windows(2).all(|pair| pair[0] <= pair[1]));

    let expected_true_counts: Vec<usize> = distributions
        .iter()
        .map(|distribution| {
            let mut rng = rand::rng();
            (0..20)
                .filter(|_| rand::distr::Distribution::sample(distribution, &mut rng))
                .count()
        })
        .collect();

    assert_eq!(expected_true_counts[0], 0, "p = 0 should produce no true values");
    assert_eq!(expected_true_counts[2], 20, "p = 1 should produce only true values");
    assert!(expected_true_counts[1] <= 20);
}

#[test]
fn invalid_bernoulli_construction_does_not_produce_a_probability_to_read() {
    assert!(Bernoulli::new(-0.0001).is_err());
    assert!(Bernoulli::new(1.0001).is_err());
    assert!(Bernoulli::new(f64::NAN).is_err());
    assert!(Bernoulli::from_ratio(2, 1).is_err());
    assert!(Bernoulli::from_ratio(1, 0).is_err());

    let valid_after_errors = Bernoulli::from_ratio(2, 5).expect("valid ratio after errors should work");
    assert!((valid_after_errors.p() - 0.4).abs() <= f64::EPSILON);
}