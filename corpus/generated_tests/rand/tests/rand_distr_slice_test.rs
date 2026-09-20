use rand::distr::slice::Choose;
use rand::distr::Distribution;

#[test]
fn num_choices_reports_nonzero_slice_length_and_sampling_preserves_membership() {
    let words = ["red", "green", "blue", "orange", "purple"];
    let choose = Choose::new(&words).expect("a non-empty slice should build a Choose distribution");

    let choices = choose.num_choices();
    assert_eq!(choices.get(), words.len());
    assert_eq!(choices.get(), 5);
    assert!(choices.get() > 0);

    let mut rng = rand::rng();
    for _ in 0..64 {
        let sampled: &&str = Distribution::sample(&choose, &mut rng);
        assert!(
            words.iter().any(|word| word == sampled),
            "sampled value should come from the original slice"
        );
    }

    assert_eq!(
        choose.num_choices().get(),
        words.len(),
        "sampling must not change the number of choices"
    );
}

#[test]
fn num_choices_handles_single_element_distribution_deterministically() {
    let values = [12345_i32];
    let choose = Choose::new(&values).expect("single-element slices are valid choices");

    assert_eq!(choose.num_choices().get(), 1);
    assert_eq!(choose.num_choices().get(), values.len());

    let mut rng = rand::thread_rng();
    for _ in 0..32 {
        let sampled: &i32 = Distribution::sample(&choose, &mut rng);
        assert_eq!(*sampled, 12345);
    }

    assert_eq!(
        choose.num_choices().get(),
        1,
        "the choice count remains stable after repeated sampling"
    );
}

#[test]
fn num_choices_rejects_empty_slices_and_accepts_larger_slices() {
    let empty: [u8; 0] = [];
    assert!(
        Choose::new(&empty).is_err(),
        "empty slices cannot produce a non-zero number of choices"
    );

    let numbers: Vec<u8> = (10..30).collect();
    let choose = Choose::new(&numbers).expect("non-empty Vec slice should be accepted");

    assert_eq!(choose.num_choices().get(), numbers.len());
    assert_eq!(choose.num_choices().get(), 20);

    let mut rng = rand::rng();
    let mut saw_at_least_one_valid_sample = false;
    for _ in 0..16 {
        let sampled: &u8 = Distribution::sample(&choose, &mut rng);
        assert!(numbers.contains(sampled));
        saw_at_least_one_valid_sample = true;
    }

    assert!(saw_at_least_one_valid_sample);
    assert_eq!(choose.num_choices().get(), numbers.len());
}