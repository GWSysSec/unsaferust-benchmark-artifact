use rand::distr::weighted::WeightedIndex;

#[test]
fn total_weight_matches_weights_after_multiple_updates() {
    let mut distribution =
        WeightedIndex::new([2_u32, 4, 6, 8]).expect("non-empty positive weights are valid");

    assert_eq!(distribution.total_weight(), 20);
    assert_eq!(distribution.weight(0), Some(2));
    assert_eq!(distribution.weight(1), Some(4));
    assert_eq!(distribution.weight(2), Some(6));
    assert_eq!(distribution.weight(3), Some(8));

    let weights_before_update: Vec<u32> = distribution.weights().collect();
    assert_eq!(weights_before_update, vec![2, 4, 6, 8]);
    assert_eq!(
        distribution.total_weight(),
        weights_before_update.iter().copied().sum::<u32>()
    );

    let ten = 10_u32;
    let zero = 0_u32;
    distribution
        .update_weights(&[(1, &ten), (3, &zero)])
        .expect("valid in-bounds updates leaving a positive total should succeed");

    assert_eq!(distribution.total_weight(), 18);
    assert_eq!(distribution.weight(0), Some(2));
    assert_eq!(distribution.weight(1), Some(10));
    assert_eq!(distribution.weight(2), Some(6));
    assert_eq!(distribution.weight(3), Some(0));
    assert_eq!(distribution.weight(4), None);

    let weights_after_update: Vec<u32> = distribution.weights().collect();
    assert_eq!(weights_after_update, vec![2, 10, 6, 0]);
    assert_eq!(
        distribution.total_weight(),
        weights_after_update.iter().copied().sum::<u32>()
    );
}

#[test]
fn total_weight_handles_zero_entries_and_failed_updates_do_not_change_state() {
    let mut distribution =
        WeightedIndex::new([0_u32, 5, 0, 7]).expect("at least one positive weight is valid");

    assert_eq!(distribution.total_weight(), 12);
    assert_eq!(distribution.weight(0), Some(0));
    assert_eq!(distribution.weight(1), Some(5));
    assert_eq!(distribution.weight(2), Some(0));
    assert_eq!(distribution.weight(3), Some(7));

    let zero = 0_u32;
    let result = distribution.update_weights(&[(1, &zero), (3, &zero)]);
    assert!(
        result.is_err(),
        "an update making the total weight zero should be rejected"
    );

    assert_eq!(
        distribution.total_weight(),
        12,
        "failed update should leave total weight unchanged"
    );
    assert_eq!(distribution.weights().collect::<Vec<u32>>(), vec![0, 5, 0, 7]);

    let three = 3_u32;
    distribution
        .update_weights(&[(0, &three), (3, &zero)])
        .expect("leaving a positive total should succeed");

    assert_eq!(distribution.total_weight(), 8);
    assert_eq!(distribution.weights().collect::<Vec<u32>>(), vec![3, 5, 0, 0]);
}

#[test]
fn total_weight_supports_floating_point_weights() {
    let mut distribution =
        WeightedIndex::new([0.5_f64, 1.25, 2.25]).expect("positive finite float weights are valid");

    assert_eq!(distribution.total_weight(), 4.0);
    assert_eq!(distribution.weight(0), Some(0.5));
    assert_eq!(distribution.weight(1), Some(1.25));
    assert_eq!(distribution.weight(2), Some(2.25));

    let two = 2.0_f64;
    let one = 1.0_f64;
    distribution
        .update_weights(&[(0, &two), (2, &one)])
        .expect("valid float weight updates should succeed");

    assert_eq!(distribution.total_weight(), 4.25);
    assert_eq!(
        distribution.weights().collect::<Vec<f64>>(),
        vec![2.0, 1.25, 1.0]
    );
}