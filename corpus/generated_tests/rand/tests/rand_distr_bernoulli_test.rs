use rand::distr::Bernoulli;

#[test]
fn bernoulli_p_matches_constructor_inputs_and_sampling_extremes() {
    let never = Bernoulli::new(0.0).expect("zero probability is valid");
    let quarter = Bernoulli::new(0.25).expect("fractional probability is valid");
    let ratio_quarter = Bernoulli::from_ratio(2, 8).expect("valid ratio is accepted");
    let certain = Bernoulli::from_ratio(7, 7).expect("ratio equal to one is valid");

    assert_eq!(never.p(), 0.0);
    assert!((quarter.p() - 0.25).abs() <= f64::EPSILON);
    assert!((ratio_quarter.p() - 0.25).abs() <= f64::EPSILON);
    assert_eq!(quarter.p(), ratio_quarter.p());
    assert_eq!(certain.p(), 1.0);
    assert!(never.p() < quarter.p());
    assert!(quarter.p() < certain.p());

    let mut rng = rand::rng();

    for _ in 0..64 {
        let never_sample: bool = rand::distr::Distribution::sample(&never, &mut rng);
        let certain_sample: bool = rand::distr::Distribution::sample(&certain, &mut rng);

        assert!(!never_sample, "p() == 0.0 distributions must never sample true");
        assert!(certain_sample, "p() == 1.0 distributions must always sample true");
    }

    assert_eq!(never.p(), 0.0);
    assert_eq!(certain.p(), 1.0);
    assert_eq!(quarter.p(), ratio_quarter.p());
}

#[test]
fn bernoulli_p_remains_stable_across_repeated_sampling_workflow() {
    let distribution = Bernoulli::from_ratio(3, 10).expect("3/10 is a valid Bernoulli ratio");
    let original_probability = distribution.p();

    assert!((original_probability - 0.3).abs() <= f64::EPSILON);
    assert!(original_probability > 0.0);
    assert!(original_probability < 1.0);

    let mut rng = rand::thread_rng();
    let samples: Vec<bool> = (0..128)
        .map(|_| rand::distr::Distribution::sample(&distribution, &mut rng))
        .collect();

    assert_eq!(samples.len(), 128);
    assert!(samples.iter().all(|value| matches!(value, true | false)));
    assert_eq!(distribution.p(), original_probability);

    let true_count = samples.iter().filter(|&&value| value).count();
    let false_count = samples.len() - true_count;

    assert_eq!(true_count + false_count, 128);
    assert_eq!(distribution.p(), 0.3);
}

#[test]
fn bernoulli_rejects_invalid_probabilities_without_producing_p_value() {
    assert!(Bernoulli::new(-f64::EPSILON).is_err());
    assert!(Bernoulli::new(1.0 + f64::EPSILON).is_err());
    assert!(Bernoulli::new(f64::NAN).is_err());
    assert!(Bernoulli::from_ratio(2, 1).is_err());
    assert!(Bernoulli::from_ratio(1, 0).is_err());

    let valid = Bernoulli::new(0.5).expect("0.5 remains valid after invalid attempts");
    assert_eq!(valid.p(), 0.5);
    assert!(valid.p() >= 0.0);
    assert!(valid.p() <= 1.0);
}