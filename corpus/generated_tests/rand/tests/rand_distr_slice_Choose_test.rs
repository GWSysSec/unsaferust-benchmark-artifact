use rand::distr::slice::Choose;

#[test]
fn choose_num_choices_matches_slice_length_and_samples_from_slice() {
    let values = ["alpha", "beta", "gamma", "delta"];
    let distribution = Choose::new(&values).expect("non-empty slices create a Choose distribution");

    let count = distribution.num_choices();
    assert_eq!(count.get(), values.len());
    assert!(count.get() > 0);

    let mut rng = rand::rng();
    for _ in 0..32 {
        let sampled: &&str = rand::distr::Distribution::sample(&distribution, &mut rng);
        assert!(
            values.contains(sampled),
            "sampled value must be one of the original slice elements"
        );
    }

    assert_eq!(distribution.num_choices().get(), 4);
}

#[test]
fn choose_num_choices_handles_single_element_slice() {
    let values = [42_u32];
    let distribution = Choose::new(&values).expect("single-element slices are valid");

    assert_eq!(distribution.num_choices().get(), 1);

    let mut rng = rand::thread_rng();
    for _ in 0..16 {
        let sampled: &u32 = rand::distr::Distribution::sample(&distribution, &mut rng);
        assert_eq!(*sampled, 42);
    }

    assert_eq!(distribution.num_choices().get(), values.len());
}

#[test]
fn choose_new_rejects_empty_slice_before_num_choices_can_be_used() {
    let empty: [u8; 0] = [];

    let result = Choose::new(&empty);

    assert!(result.is_err());
    assert_eq!(empty.len(), 0);
}